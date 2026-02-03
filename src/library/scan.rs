use std::path::Path;
use std::time::Duration;

use lofty::{AudioFile, ItemKey, TaggedFileExt};
use walkdir::WalkDir;

use crate::config::LibrarySettings;

use super::display::display_from_fields;
use super::model::Track;

fn is_audio_file(path: &Path, settings: &LibrarySettings) -> bool {
    let exts: Vec<String> = settings
        .extensions
        .iter()
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();

    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| {
            let ext = ext.to_ascii_lowercase();
            exts.iter().any(|e| e == &ext)
        })
        .unwrap_or(false)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

pub fn scan(dir: &Path, settings: &LibrarySettings) -> Vec<Track> {
    let mut tracks: Vec<Track> = Vec::new();

    let mut walker = WalkDir::new(dir).follow_links(settings.follow_links);

    // Non-recursive = only the root directory.
    let depth_cap = if settings.recursive {
        settings.max_depth
    } else {
        Some(1)
    };
    if let Some(d) = depth_cap {
        walker = walker.max_depth(d);
    }

    for entry in walker
        .into_iter()
        .filter_entry(|e| settings.include_hidden || e.depth() == 0 || !is_hidden(e.path()))
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file()
            && (settings.include_hidden || !is_hidden(path))
            && is_audio_file(path, settings)
        {
            let default_title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            let mut title = default_title;
            let mut artist: Option<String> = None;
            let mut album: Option<String> = None;
            let mut duration: Option<Duration> = None;

            if let Ok(tagged) = lofty::read_from_path(path) {
                duration = Some(tagged.properties().duration());

                if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
                    if let Some(v) = tag.get_string(&ItemKey::TrackTitle) {
                        if !v.trim().is_empty() {
                            title = v.to_string();
                        }
                    }
                    if let Some(v) = tag.get_string(&ItemKey::TrackArtist) {
                        let v = v.trim();
                        if !v.is_empty() {
                            artist = Some(v.to_string());
                        }
                    }
                    if let Some(v) = tag.get_string(&ItemKey::AlbumTitle) {
                        let v = v.trim();
                        if !v.is_empty() {
                            album = Some(v.to_string());
                        }
                    }
                }
            }

            let display = display_from_fields(
                path,
                &title,
                artist.as_deref(),
                album.as_deref(),
                &settings.display_fields,
                &settings.display_separator,
            );

            tracks.push(Track {
                path: path.to_path_buf(),
                title,
                artist,
                album,
                duration,
                display,
            });
        }
    }

    tracks.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));
    tracks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TrackDisplayField;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn is_audio_file_matches_configured_extensions_case_insensitive() {
        let settings = LibrarySettings::default();
        assert!(is_audio_file(Path::new("/tmp/a.mp3"), &settings));
        assert!(is_audio_file(Path::new("/tmp/a.MP3"), &settings));
        assert!(is_audio_file(Path::new("/tmp/a.flac"), &settings));
        assert!(is_audio_file(Path::new("/tmp/a.wav"), &settings));
        assert!(is_audio_file(Path::new("/tmp/a.ogg"), &settings));
        assert!(!is_audio_file(Path::new("/tmp/a.txt"), &settings));
        assert!(!is_audio_file(Path::new("/tmp/a"), &settings));
    }

    #[test]
    fn scan_filters_non_audio_and_sorts_by_display_case_insensitive() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("b.MP3"), b"not a real mp3").unwrap();
        fs::write(dir.path().join("A.ogg"), b"not a real ogg").unwrap();
        fs::write(dir.path().join("c.txt"), b"ignore me").unwrap();

        let settings = LibrarySettings {
            // Match previous test behavior: display = filename
            display_fields: vec![TrackDisplayField::Title],
            ..LibrarySettings::default()
        };
        let tracks = scan(dir.path(), &settings);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "A");
        assert_eq!(tracks[0].display, "A");
        assert_eq!(tracks[1].title, "b");
        assert_eq!(tracks[1].display, "b");
    }

    #[test]
    fn scan_respects_include_hidden_false() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".hidden.mp3"), b"not real").unwrap();
        fs::write(dir.path().join("visible.mp3"), b"not real").unwrap();

        let settings = LibrarySettings {
            include_hidden: false,
            display_fields: vec![TrackDisplayField::Filename],
            ..LibrarySettings::default()
        };
        let tracks = scan(dir.path(), &settings);

        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].display, "visible");
    }

    #[test]
    fn scan_respects_recursive_false() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("root.mp3"), b"not real").unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("child.mp3"), b"not real").unwrap();

        let settings = LibrarySettings {
            recursive: false,
            display_fields: vec![TrackDisplayField::Filename],
            ..LibrarySettings::default()
        };
        let tracks = scan(dir.path(), &settings);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].display, "root");
    }

    #[test]
    fn scan_respects_max_depth() {
        let dir = tempdir().unwrap();
        let d1 = dir.path().join("d1");
        let d2 = d1.join("d2");
        fs::create_dir_all(&d2).unwrap();
        fs::write(dir.path().join("root.mp3"), b"not real").unwrap();
        fs::write(d1.join("one.mp3"), b"not real").unwrap();
        fs::write(d2.join("two.mp3"), b"not real").unwrap();

        // WalkDir depth counts root as 0, children as 1, grandchildren as 2...
        // With max_depth=2 we should see root + d1/*, but not d1/d2/*.
        let settings = LibrarySettings {
            max_depth: Some(2),
            display_fields: vec![TrackDisplayField::Filename],
            ..LibrarySettings::default()
        };
        let tracks = scan(dir.path(), &settings);

        let names: Vec<String> = tracks.iter().map(|t| t.display.clone()).collect();
        assert!(names.contains(&"root".to_string()));
        assert!(names.contains(&"one".to_string()));
        assert!(!names.contains(&"two".to_string()));
    }
}
