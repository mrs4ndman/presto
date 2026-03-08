use std::path::PathBuf;
use std::time::Duration;

/// Representation of a single audio track discovered in the library.
#[derive(Clone)]
pub struct Track {
    /// Filesystem path to the audio file.
    pub path: PathBuf,
    /// Track title (from tags or filename fallback).
    pub title: String,
    /// Optional artist metadata.
    pub artist: Option<String>,
    /// Optional album metadata.
    pub album: Option<String>,
    /// Optional duration if it could be read from file properties.
    pub duration: Option<Duration>,
    /// Precomputed display string used for sorting and UI.
    pub display: String,
}
