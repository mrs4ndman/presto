use std::path::{Path, PathBuf};

use lofty::{AudioFile, ItemKey, TaggedFileExt};
use std::time::Duration;
use walkdir::WalkDir;

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<Duration>,
    pub display: String,
}

fn make_display(title: &str, artist: Option<&str>) -> String {
    match artist {
        Some(a) if !a.trim().is_empty() => format!("{} - {}", a.trim(), title),
        _ => title.to_string(),
    }
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "wav" | "ogg"
            )
        })
        .unwrap_or(false)
}

pub fn scan(dir: &Path) -> Vec<Track> {
    let mut tracks: Vec<Track> = Vec::new();
    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() && is_audio_file(path) {
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

            let display = make_display(&title, artist.as_deref());

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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn make_display_prefers_artist_dash_title() {
        assert_eq!(make_display("Song", Some("Artist")), "Artist - Song");
        assert_eq!(make_display("Song", Some("  Artist  ")), "Artist - Song");
        assert_eq!(make_display("Song", None), "Song");
        assert_eq!(make_display("Song", Some("")), "Song");
        assert_eq!(make_display("Song", Some("   ")), "Song");
    }

    #[test]
    fn is_audio_file_matches_known_extensions_case_insensitive() {
        assert!(is_audio_file(Path::new("/tmp/a.mp3")));
        assert!(is_audio_file(Path::new("/tmp/a.MP3")));
        assert!(is_audio_file(Path::new("/tmp/a.flac")));
        assert!(is_audio_file(Path::new("/tmp/a.wav")));
        assert!(is_audio_file(Path::new("/tmp/a.ogg")));
        assert!(!is_audio_file(Path::new("/tmp/a.txt")));
        assert!(!is_audio_file(Path::new("/tmp/a")));
    }

    #[test]
    fn scan_filters_non_audio_and_sorts_by_display_case_insensitive() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("b.MP3"), b"not a real mp3").unwrap();
        fs::write(dir.path().join("A.ogg"), b"not a real ogg").unwrap();
        fs::write(dir.path().join("c.txt"), b"ignore me").unwrap();

        let tracks = scan(dir.path());
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "A");
        assert_eq!(tracks[0].display, "A");
        assert_eq!(tracks[1].title, "b");
        assert_eq!(tracks[1].display, "b");
    }
}
