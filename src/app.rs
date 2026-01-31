use crate::audio::{LoopMode, PlaybackHandle};
use crate::library::Track;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::OrderHandle;
    use std::sync::{Arc, Mutex};

    fn t(title: &str) -> Track {
        Track {
            path: std::path::PathBuf::new(),
            title: title.into(),
            artist: None,
            album: None,
            duration: None,
            display: title.into(),
        }
    }

    #[test]
    fn fuzzy_match_simple() {
        let title = "Hello World";
        assert!(App::fuzzy_match_positions(title, "hw").is_some());
        assert!(App::fuzzy_match_positions(title, "ello").is_some());
        assert!(App::fuzzy_match_positions(title, "xyz").is_none());
    }

    #[test]
    fn display_indices_respects_filter_query() {
        let tracks = vec![t("Alpha"), t("Beta"), t("Gamma")];
        let mut app = App::new(tracks);
        app.push_filter_char('a');
        let visible = app.display_indices();
        assert!(!visible.is_empty());
    }

    #[test]
    fn display_indices_respects_order_and_filter() {
        let tracks = vec![t("Alpha"), t("Beta"), t("Gamma"), t("Delta")];

        let mut app = App::new(tracks);
        // custom order: 2,0,3,1
        let order = vec![2usize, 0, 3, 1];
        let oh: OrderHandle = Arc::new(Mutex::new(order.clone()));
        app.set_order_handle(oh);
        app.shuffle = true;

        let disp = app.display_indices();
        assert_eq!(disp, order);

        // apply fuzzy filter 'et' -> matches Delta(3) and Beta(1)
        app.filter_query = "et".into();
        let disp2 = app.display_indices();
        assert_eq!(disp2, vec![3usize, 1usize]);
    }

    #[test]
    fn display_indices_uses_fuzzy_not_substring_only() {
        let tracks = vec![t("Metallica - Blackened"), t("Black Sabbath - Paranoid")];

        let mut app = App::new(tracks);
        // Fuzzy query: letters appear in order but not necessarily contiguously
        app.filter_query = "mtbk".into();

        let disp = app.display_indices();
        assert_eq!(disp, vec![0]);
    }

    #[test]
    fn trimming_filter_query_affects_matching() {
        let tracks = vec![t("Black Sabbath - Paranoid")];

        let mut app = App::new(tracks);
        app.filter_query = "Black ".into();
        assert_eq!(app.display_indices(), vec![0]);

        app.filter_query = "   ".into();
        assert_eq!(app.display_indices(), vec![0]);
    }

    #[test]
    fn next_prev_in_view_helpers_work() {
        let tracks = vec![t("Alpha"), t("Beta"), t("Gamma")];

        let mut app = App::new(tracks);
        app.filter_query = "et".into(); // only Beta is visible

        assert_eq!(app.next_in_view_from(0), Some(1));
        assert_eq!(app.prev_in_view_from(0), Some(1));
        assert_eq!(app.next_in_view_from(1), Some(1));
        assert_eq!(app.prev_in_view_from(1), Some(1));
    }

    #[test]
    fn cycle_loop_mode_cycles_three_states() {
        let tracks = vec![t("A")];

        let mut app = App::new(tracks);
        assert_eq!(app.loop_mode, crate::audio::LoopMode::NoLoop);

        app.cycle_loop_mode();
        assert_eq!(app.loop_mode, crate::audio::LoopMode::LoopAll);

        app.cycle_loop_mode();
        assert_eq!(app.loop_mode, crate::audio::LoopMode::LoopOne);

        app.cycle_loop_mode();
        assert_eq!(app.loop_mode, crate::audio::LoopMode::NoLoop);
    }

    #[test]
    fn queue_dirty_is_set_on_filter_changes() {
        let tracks = vec![t("Alpha")];

        let mut app = App::new(tracks);
        // new() starts dirty so initial queue can be synced
        assert!(app.queue_dirty);
        app.clear_queue_dirty();
        assert!(!app.queue_dirty);

        app.push_filter_char('a');
        assert!(app.queue_dirty);
        app.clear_queue_dirty();
        app.pop_filter_char();
        assert!(app.queue_dirty);
    }
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
    pub playback_handle: Option<PlaybackHandle>,

    lower_titles: Option<Vec<String>>,

    pub follow_playback: bool,
    pub pending_follow_index: Option<usize>,

    pub loop_mode: LoopMode,
    pub queue_dirty: bool,

    pub shuffle: bool,
    pub filter_mode: bool,
    pub filter_query: String,
    pub order_handle: Option<crate::audio::OrderHandle>,
    pub current_dir: Option<String>,
}

impl App {
    pub fn new(tracks: Vec<Track>) -> Self {
        // Optimization: for larger libraries, precompute lowercase titles to speed up fuzzy
        // filtering (avoid per-char lowercase conversions on every redraw/keystroke).
        let lower_titles = if tracks.len() > 100 {
            Some(
                tracks
                    .iter()
                    .map(|t| t.display.to_ascii_lowercase())
                    .collect(),
            )
        } else {
            None
        };

        Self {
            tracks,
            selected: 0,
            playback: PlaybackState::Stopped,
            playback_handle: None,

            lower_titles,

            follow_playback: true,
            pending_follow_index: None,

            loop_mode: LoopMode::NoLoop,
            queue_dirty: true,
            shuffle: false,
            filter_mode: false,
            filter_query: String::new(),
            order_handle: None,
            current_dir: None,
        }
    }

    pub fn mark_queue_dirty(&mut self) {
        self.queue_dirty = true;
    }

    pub fn clear_queue_dirty(&mut self) {
        self.queue_dirty = false;
    }

    pub fn cycle_loop_mode(&mut self) {
        self.loop_mode = match self.loop_mode {
            LoopMode::NoLoop => LoopMode::LoopAll,
            LoopMode::LoopAll => LoopMode::LoopOne,
            LoopMode::LoopOne => LoopMode::NoLoop,
        };
    }

    pub fn follow_playback_on(&mut self) {
        self.follow_playback = true;
    }

    pub fn follow_playback_off(&mut self) {
        self.follow_playback = false;
        self.pending_follow_index = None;
    }

    pub fn set_pending_follow_index(&mut self, idx: usize) {
        self.pending_follow_index = Some(idx);
    }

    pub fn clear_pending_follow_index(&mut self) {
        self.pending_follow_index = None;
    }

    pub fn set_playback_handle(&mut self, h: PlaybackHandle) {
        self.playback_handle = Some(h);
    }

    pub fn set_order_handle(&mut self, h: crate::audio::OrderHandle) {
        self.order_handle = Some(h);
    }

    pub fn set_current_dir(&mut self, dir: String) {
        self.current_dir = Some(dir);
    }

    // Return the display order of indices, taking into account shuffle `order_handle`.
    pub fn display_indices(&self) -> Vec<usize> {
        let base: Vec<usize> = if self.shuffle {
            if let Some(ref oh) = self.order_handle {
                if let Ok(v) = oh.lock() {
                    v.clone()
                } else {
                    (0..self.tracks.len()).collect()
                }
            } else {
                (0..self.tracks.len()).collect()
            }
        } else {
            (0..self.tracks.len()).collect()
        };

        // Apply filtering (retain only indices that match filter)
        let query = self.filter_query.trim();
        if query.is_empty() {
            base
        } else {
            match self.lower_titles.as_deref() {
                Some(lower_titles) => {
                    let query_lower = query.to_ascii_lowercase();
                    base.into_iter()
                        .filter(|&i| {
                            Self::fuzzy_match_positions_lower(&lower_titles[i], &query_lower)
                                .is_some()
                        })
                        .collect()
                }
                None => base
                    .into_iter()
                    .filter(|&i| {
                        Self::fuzzy_match_positions(&self.tracks[i].display, query).is_some()
                    })
                    .collect(),
            }
        }
    }

    pub fn uses_lower_titles(&self) -> bool {
        self.lower_titles.is_some()
    }

    pub fn fuzzy_match_positions_for_track_lower(
        &self,
        track_index: usize,
        query_lower: &str,
    ) -> Option<Vec<usize>> {
        if query_lower.is_empty() {
            return Some(Vec::new());
        }

        match self.lower_titles.as_deref() {
            Some(lower_titles) => {
                Self::fuzzy_match_positions_lower(&lower_titles[track_index], query_lower)
            }
            None => Self::fuzzy_match_positions(&self.tracks[track_index].display, query_lower),
        }
    }

    pub fn next_in_view_from(&self, current: usize) -> Option<usize> {
        let display = self.display_indices();
        if display.is_empty() {
            return None;
        }

        let pos = display.iter().position(|&i| i == current);
        match pos {
            Some(p) => Some(display[(p + 1) % display.len()]),
            None => Some(display[0]),
        }
    }

    pub fn prev_in_view_from(&self, current: usize) -> Option<usize> {
        let display = self.display_indices();
        if display.is_empty() {
            return None;
        }

        let pos = display.iter().position(|&i| i == current);
        match pos {
            Some(0) => Some(display[display.len() - 1]),
            Some(p) => Some(display[p - 1]),
            None => Some(display[display.len() - 1]),
        }
    }

    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.mark_queue_dirty();
    }

    pub fn set_selected(&mut self, idx: usize) {
        self.selected = idx;
        self.ensure_selected_visible();
    }

    pub fn has_tracks(&self) -> bool {
        !self.tracks.is_empty()
    }

    // Fuzzy/subsequence match: return the character positions (by char index)
    // in `title` that match the query, or None if not matched.
    pub fn fuzzy_match_positions(title: &str, query: &str) -> Option<Vec<usize>> {
        if query.is_empty() {
            return Some(Vec::new());
        }

        let mut positions: Vec<usize> = Vec::new();
        let mut title_iter = title.chars().enumerate();

        for qc in query.chars() {
            let qc_low = qc.to_ascii_lowercase();
            loop {
                match title_iter.next() {
                    Some((ti, tc)) if tc.to_ascii_lowercase() == qc_low => {
                        positions.push(ti);
                        break;
                    }
                    Some(_) => continue,
                    None => return None,
                }
            }
        }

        Some(positions)
    }

    fn fuzzy_match_positions_lower(title_lower: &str, query_lower: &str) -> Option<Vec<usize>> {
        if query_lower.is_empty() {
            return Some(Vec::new());
        }

        let mut positions: Vec<usize> = Vec::new();
        let mut title_iter = title_lower.chars().enumerate();

        for qc in query_lower.chars() {
            loop {
                match title_iter.next() {
                    Some((ti, tc)) if tc == qc => {
                        positions.push(ti);
                        break;
                    }
                    Some(_) => continue,
                    None => return None,
                }
            }
        }

        Some(positions)
    }

    pub fn enter_filter_mode(&mut self) {
        self.filter_mode = true;
        self.follow_playback_off();
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }

    pub fn exit_filter_mode(&mut self) {
        self.filter_mode = false;
        self.mark_queue_dirty();
    }

    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.filter_mode = false;
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }

    pub fn push_filter_char(&mut self, c: char) {
        self.filter_query.push(c);
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }

    pub fn pop_filter_char(&mut self) {
        self.filter_query.pop();
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }

    fn ensure_selected_visible(&mut self) {
        let display = self.display_indices();
        if display.is_empty() {
            self.selected = 0;
            return;
        }

        if !display.contains(&self.selected) {
            self.selected = display[0];
        }
    }

    pub fn next(&mut self) {
        if let Some(next) = self.next_in_view_from(self.selected) {
            self.selected = next;
        }
    }

    pub fn prev(&mut self) {
        if let Some(prev) = self.prev_in_view_from(self.selected) {
            self.selected = prev;
        }
    }
}
