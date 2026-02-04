use std::path::Path;

use crate::config::TrackDisplayField;

/// Build a display string for a track according to the provided `fields` and separator.
///
/// This composes metadata fields (artist, title, album, filename, path) in the
/// configured order and falls back to `title` when no parts were produced.
pub fn display_from_fields(
    path: &Path,
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    fields: &[TrackDisplayField],
    sep: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    for f in fields {
        match f {
            TrackDisplayField::Display => {
                // If someone includes "display" here, treat it as "artist - title" by default.
                if let Some(a) = artist.map(str::trim).filter(|s| !s.is_empty()) {
                    parts.push(a.to_string());
                }
                if !title.trim().is_empty() {
                    parts.push(title.trim().to_string());
                }
            }
            TrackDisplayField::Title => {
                if !title.trim().is_empty() {
                    parts.push(title.trim().to_string());
                }
            }
            TrackDisplayField::Artist => {
                if let Some(a) = artist.map(str::trim).filter(|s| !s.is_empty()) {
                    parts.push(a.to_string());
                }
            }
            TrackDisplayField::Album => {
                if let Some(a) = album.map(str::trim).filter(|s| !s.is_empty()) {
                    parts.push(a.to_string());
                }
            }
            TrackDisplayField::Filename => {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if !stem.trim().is_empty() {
                        parts.push(stem.to_string());
                    }
                }
            }
            TrackDisplayField::Path => {
                parts.push(path.display().to_string());
            }
        }
    }

    if parts.is_empty() {
        title.to_string()
    } else {
        parts.join(sep)
    }
}
