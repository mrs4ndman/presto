use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::audio::LoopMode;
use crate::config::load::default_config_path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectoryState {
    pub selected_path: Option<String>,
    pub last_played_path: Option<String>,
    pub volume_percent: Option<u8>,
    pub filter_query: Option<String>,
    pub shuffle: Option<bool>,
    pub loop_mode: Option<LoopMode>,
    pub follow_playback: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PersistedState {
    dirs: BTreeMap<String, DirectoryState>,
}

#[derive(Debug)]
pub struct StateStoreError {
    path: PathBuf,
    source: io::Error,
}

impl StateStoreError {
    fn new(path: &PathBuf, source: io::Error) -> Self {
        Self {
            path: path.clone(),
            source,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Display for StateStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.source)
    }
}

impl std::error::Error for StateStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

pub struct StateStore {
    path: Option<PathBuf>,
}

impl StateStore {
    pub fn new_default() -> Self {
        Self {
            path: state_file_path(),
        }
    }

    #[cfg(test)]
    pub fn with_path(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub fn load_directory_state(
        &self,
        dir: &str,
    ) -> Result<Option<DirectoryState>, StateStoreError> {
        let all = self.load_all_state()?;
        Ok(all.dirs.get(dir).cloned())
    }

    pub fn persist_directory_state(&self, dir: &str, app: &App) -> Result<(), StateStoreError> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(()),
        };

        let mut all = self.load_all_state()?;

        let selected_path = app
            .tracks
            .get(app.selected)
            .map(|t| t.path.to_string_lossy().to_string());

        let last_played_path = app
            .playback_handle
            .as_ref()
            .and_then(|h| h.lock().ok().and_then(|info| info.index))
            .and_then(|idx| app.tracks.get(idx))
            .map(|t| t.path.to_string_lossy().to_string())
            .or_else(|| selected_path.clone());

        all.dirs.insert(
            dir.to_string(),
            DirectoryState {
                selected_path,
                last_played_path,
                volume_percent: Some(app.volume_percent()),
                filter_query: if app.filter_query.trim().is_empty() {
                    None
                } else {
                    Some(app.filter_query.clone())
                },
                shuffle: Some(app.shuffle),
                loop_mode: Some(app.loop_mode),
                follow_playback: Some(app.follow_playback),
            },
        );

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| StateStoreError::new(path, e))?;
        }

        let data = toml::to_string(&all)
            .map_err(|e| StateStoreError::new(path, io::Error::new(io::ErrorKind::Other, e)))?;
        fs::write(path, data).map_err(|e| StateStoreError::new(path, e))
    }

    /// Load the full persisted state file (or an empty default if missing).
    fn load_all_state(&self) -> Result<PersistedState, StateStoreError> {
        let path = match &self.path {
            Some(p) => p,
            None => return Ok(PersistedState::default()),
        };

        let data = match fs::read_to_string(path) {
            Ok(data) => data,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(PersistedState::default());
            }
            Err(err) => return Err(StateStoreError::new(path, err)),
        };

        toml::from_str(&data)
            .map_err(|e| StateStoreError::new(path, io::Error::new(io::ErrorKind::InvalidData, e)))
    }
}

/// Compute the state file path alongside the config directory.
fn state_file_path() -> Option<PathBuf> {
    default_config_path().and_then(|p| p.parent().map(|d| d.join("state.toml")))
}

/// Apply persisted filter and selection to the app, if present.
pub fn apply_filter_and_selection(app: &mut App, state: Option<&DirectoryState>) {
    if let Some(st) = state {
        if let Some(filter) = st.filter_query.as_ref() {
            app.filter_query = filter.clone();
            app.mark_queue_dirty();
        }

        if let Some(pct) = st.volume_percent {
            app.set_initial_volume_percent(pct);
        }

        let candidate_path = st.last_played_path.as_ref().or(st.selected_path.as_ref());

        let mut selected_set = false;
        if let Some(path) = candidate_path {
            if let Some((idx, _)) = app
                .tracks
                .iter()
                .enumerate()
                .find(|(_, t)| t.path.to_string_lossy() == path.as_str())
            {
                app.set_selected(idx);
                selected_set = true;
            }
        }

        if !selected_set {
            let display = app.display_indices();
            if let Some(&first) = display.first() {
                app.set_selected(first);
            }
        }
    } else {
        let display = app.display_indices();
        if let Some(&first) = display.first() {
            app.set_selected(first);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Track;
    fn track(path: &str, title: &str) -> Track {
        Track {
            path: std::path::PathBuf::from(path),
            title: title.to_string(),
            artist: None,
            album: None,
            duration: None,
            display: title.to_string(),
        }
    }

    #[test]
    fn persist_and_load_directory_state_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::with_path(Some(dir.path().join("state.toml")));

        let tracks = vec![
            track("/music/a.mp3", "Alpha"),
            track("/music/b.mp3", "Beta"),
        ];
        let mut app = App::new(tracks);
        app.set_selected(1);
        app.filter_query = "abc".to_string();
        app.shuffle = true;
        app.loop_mode = LoopMode::LoopOne;
        app.follow_playback = false;

        store.persist_directory_state("/music", &app).unwrap();

        let loaded = store.load_directory_state("/music").unwrap().unwrap();
        assert_eq!(loaded.selected_path, Some("/music/b.mp3".to_string()));
        assert_eq!(loaded.last_played_path, Some("/music/b.mp3".to_string()));
        assert_eq!(loaded.volume_percent, Some(100));
        assert_eq!(loaded.filter_query, Some("abc".to_string()));
        assert_eq!(loaded.shuffle, Some(true));
        assert_eq!(loaded.loop_mode, Some(LoopMode::LoopOne));
        assert_eq!(loaded.follow_playback, Some(false));
    }

    #[test]
    fn persist_omits_blank_filter_query() {
        let dir = tempfile::tempdir().unwrap();
        let store = StateStore::with_path(Some(dir.path().join("state.toml")));

        let tracks = vec![track("/music/a.mp3", "A")];
        let mut app = App::new(tracks);
        app.filter_query = "   ".to_string();

        store.persist_directory_state("/music", &app).unwrap();

        let loaded = store.load_directory_state("/music").unwrap().unwrap();
        assert_eq!(loaded.filter_query, None);
    }

    #[test]
    fn apply_filter_and_selection_marks_queue_dirty() {
        let tracks = vec![
            track("/music/a.mp3", "Alpha"),
            track("/music/b.mp3", "Beta"),
        ];
        let mut app = App::new(tracks);
        app.clear_queue_dirty();

        let state = DirectoryState {
            selected_path: Some("/music/b.mp3".to_string()),
            last_played_path: None,
            volume_percent: None,
            filter_query: Some("beta".to_string()),
            shuffle: None,
            loop_mode: None,
            follow_playback: None,
        };

        apply_filter_and_selection(&mut app, Some(&state));
        assert_eq!(app.filter_query, "beta");
        assert!(app.queue_dirty);
        assert_eq!(app.selected, 1);
    }
}
