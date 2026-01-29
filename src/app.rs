use crate::library::Track;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::Stopped
    }
}

pub struct App {
    pub tracks: Vec<Track>,
    pub selected: usize,
    pub playback: PlaybackState,

    pub filter_mode: bool,
    pub filter_query: String,
}

impl App {
    pub fn new(tracks: Vec<Track>) -> Self {
        Self {
            tracks,
            selected: 0,
            playback: PlaybackState::Stopped,

            filter_mode: false,
            filter_query: String::new(),
        }
    }

    pub fn has_tracks(&self) -> bool {
        !self.tracks.is_empty()
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        if self.tracks.is_empty() {
            return Vec::new();
        }

        if self.filter_query.is_empty() {
            return (0..self.tracks.len()).collect();
        }

        let query = self.filter_query.to_ascii_lowercase();
        self.tracks
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                if t.title.to_ascii_lowercase().contains(&query) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn enter_filter_mode(&mut self) {
        self.filter_mode = true;
        self.ensure_selected_visible();
    }

    pub fn exit_filter_mode(&mut self) {
        self.filter_mode = false;
    }

    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.filter_mode = false;
        self.ensure_selected_visible();
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.filter_query.push(c);
        self.ensure_selected_visible();
    }

    pub fn pop_filter_char(&mut self) {
        self.filter_query.pop();
        self.ensure_selected_visible();
    }

    fn ensure_selected_visible(&mut self) {
        let filtered = self.filtered_indices();
        if filtered.is_empty() {
            self.selected = 0;
            return;
        }

        if !filtered.contains(&self.selected) {
            self.selected = filtered[0];
        }
    }

    pub fn next(&mut self) {
        let filtered = self.filtered_indices();
        if filtered.is_empty() {
            return;
        }

        let pos = filtered
            .iter()
            .position(|&i| i == self.selected)
            .unwrap_or(0);
        let next_pos = (pos + 1) % filtered.len();
        self.selected = filtered[next_pos];
    }

    pub fn prev(&mut self) {
        let filtered = self.filtered_indices();
        if filtered.is_empty() {
            return;
        }

        let pos = filtered
            .iter()
            .position(|&i| i == self.selected)
            .unwrap_or(0);
        let prev_pos = if pos == 0 {
            filtered.len() - 1
        } else {
            pos - 1
        };
        self.selected = filtered[prev_pos];
    }
}
