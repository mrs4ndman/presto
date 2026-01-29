use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Clone)]
pub struct Track {
    pub path: PathBuf,
    pub title: String,
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
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("UNKNOWN")
                .to_string();

            tracks.push(Track {
                path: path.to_path_buf(),
                title,
            });
        }
    }

    tracks.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    tracks
}
