use serde::Deserialize;

/// Top-level application settings loaded from `config.toml`.
///
/// File format: TOML
/// Default path (Linux/XDG): `$XDG_CONFIG_HOME/presto/config.toml` or `~/.config/presto/config.toml`
///
/// Precedence (highest wins):
/// 1) Environment variables (prefix `PRESTO__`, `__` as nested separator)
/// 2) Config file (if present)
/// 3) Struct defaults
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub audio: AudioSettings,
    pub ui: UiSettings,
    pub controls: ControlsSettings,
    pub playback: PlaybackSettings,
    pub library: LibrarySettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            audio: AudioSettings::default(),
            ui: UiSettings::default(),
            controls: ControlsSettings::default(),
            playback: PlaybackSettings::default(),
            library: LibrarySettings::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// Crossfade duration when switching tracks (milliseconds).
    /// Set to 0 to disable crossfade.
    pub crossfade_ms: u64,
    /// Number of steps used to fade volumes (higher = smoother, more CPU).
    pub crossfade_steps: u64,
    /// Fade-out duration when quitting (milliseconds).
    /// Set to 0 to stop immediately.
    pub quit_fade_out_ms: u64,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            crossfade_ms: 250,
            crossfade_steps: 10,
            quit_fade_out_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UiSettings {
    /// Whether the cursor starts in "follow playback" mode.
    pub follow_playback: bool,

    /// The text rendered inside the top "presto" header box.
    pub header_text: String,

    /// Which track fields to show in the status "Song:" line, and in what order.
    ///
    /// Example: ["artist", "title", "album"]
    pub now_playing_track_fields: Vec<TrackDisplayField>,

    /// Separator used to join `now_playing_track_fields`.
    pub now_playing_track_separator: String,

    /// Which time fields to show for the status line, and in what order.
    ///
    /// Example: ["elapsed", "total", "remaining"]
    pub now_playing_time_fields: Vec<TimeField>,

    /// Separator used to join `now_playing_time_fields`.
    pub now_playing_time_separator: String,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            follow_playback: true,
            header_text: " ~ And presto! It's music ~ ".to_string(),
            now_playing_track_fields: vec![TrackDisplayField::Display],
            now_playing_track_separator: " - ".to_string(),
            now_playing_time_fields: vec![TimeField::Elapsed, TimeField::Total, TimeField::Remaining],
            now_playing_time_separator: " / ".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ControlsSettings {
    /// Number of seconds to scrub when pressing `H` / `L`.
    pub scrub_seconds: u64,
}

impl Default for ControlsSettings {
    fn default() -> Self {
        Self { scrub_seconds: 5 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PlaybackSettings {
    /// Whether shuffle starts enabled.
    pub shuffle: bool,
    /// Default loop mode.
    pub loop_mode: LoopModeSetting,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            shuffle: false,
            loop_mode: LoopModeSetting::LoopAll,
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoopModeSetting {
    #[serde(alias = "no_loop", alias = "no-loop")]
    NoLoop,
    #[serde(
        alias = "loopall",
        alias = "loop_all",
        alias = "loop-all",
        alias = "loop-around"
    )]
    LoopAll,
    #[serde(
        alias = "loopone",
        alias = "loop_one",
        alias = "loop-one",
        alias = "repeat-one"
    )]
    LoopOne,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeField {
    Elapsed,
    Total,
    Remaining,
}

#[derive(Debug, Copy, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackDisplayField {
    /// Use `track.display` (whatever the scanner produced).
    Display,
    Title,
    Artist,
    Album,
    Filename,
    Path,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LibrarySettings {
    /// File extensions to treat as audio (case-insensitive, without dot).
    pub extensions: Vec<String>,
    /// Whether to follow symlinks during scanning.
    pub follow_links: bool,
    /// Whether to include hidden files/directories (dotfiles).
    pub include_hidden: bool,
    /// Whether to recurse into subdirectories.
    pub recursive: bool,
    /// Optional cap on directory recursion depth.
    pub max_depth: Option<usize>,

    /// Which fields to use to build `Track.display` and its ordering.
    ///
    /// Example: ["artist", "title"] -> "Artist - Title"
    pub display_fields: Vec<TrackDisplayField>,
    /// Separator used to join `display_fields`.
    pub display_separator: String,
}

impl Default for LibrarySettings {
    fn default() -> Self {
        Self {
            extensions: vec!["mp3".into(), "flac".into(), "wav".into(), "ogg".into()],
            follow_links: true,
            include_hidden: true,
            recursive: true,
            max_depth: None,
            display_fields: vec![TrackDisplayField::Artist, TrackDisplayField::Title],
            display_separator: " - ".to_string(),
        }
    }
}
