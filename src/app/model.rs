//! Application model types: `App` and `PlaybackState`.
//!
//! The `App` struct holds the current library, selected track and playback
//! related flags used by the UI and runtime.

use crate::audio::{LoopMode, PlaybackHandle};
use crate::library::Track;

/// The playback state of the application.
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

/// The main application model.
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
    pub metadata_window: bool,
}

impl App {
    pub fn toggle_metadata_window(&mut self) {
        self.metadata_window = !self.metadata_window;
    }
    /// Create a new `App` with the provided list of `tracks`.
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

            loop_mode: LoopMode::LoopAll,
            queue_dirty: true,
            shuffle: false,
            filter_mode: false,
            filter_query: String::new(),
            order_handle: None,
            current_dir: None,
            metadata_window: false,
        }
    }

    /// Mark the queue as needing regeneration (flags that the order changed).
    pub fn mark_queue_dirty(&mut self) {
        self.queue_dirty = true;
    }
    /// Clear the "queue dirty" flag.
    pub fn clear_queue_dirty(&mut self) {
        self.queue_dirty = false;
    }
    /// Cycle `loop_mode` through `NoLoop -> LoopAll -> LoopOne`.
    pub fn cycle_loop_mode(&mut self) {
        self.loop_mode = match self.loop_mode {
            LoopMode::NoLoop => LoopMode::LoopAll,
            LoopMode::LoopAll => LoopMode::LoopOne,
            LoopMode::LoopOne => LoopMode::NoLoop,
        };
    }
    /// Enable following playback (cursor follows currently playing track).
    pub fn follow_playback_on(&mut self) {
        self.follow_playback = true;
    }
    /// Disable follow-playback and clear any pending follow index.
    pub fn follow_playback_off(&mut self) {
        self.follow_playback = false;
        self.pending_follow_index = None;
    }
    /// Set an index to follow once playback information becomes available.
    pub fn set_pending_follow_index(&mut self, idx: usize) {
        self.pending_follow_index = Some(idx);
    }
    /// Clear the pending follow index.
    pub fn clear_pending_follow_index(&mut self) {
        self.pending_follow_index = None;
    }
    /// Attach a `PlaybackHandle` used to observe playback progress.
    pub fn set_playback_handle(&mut self, h: PlaybackHandle) {
        self.playback_handle = Some(h);
    }
    /// Set the shared `OrderHandle` used for shuffled display order.
    pub fn set_order_handle(&mut self, h: crate::audio::OrderHandle) {
        self.order_handle = Some(h);
    }
    /// Record the current directory in the app state.
    pub fn set_current_dir(&mut self, dir: String) {
        self.current_dir = Some(dir);
    }

    /// Return the display order of track indices, taking into account shuffle
    /// `order_handle` and active filtering.
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

    /// Return true if this `App` uses precomputed lowercase titles.
    pub fn uses_lower_titles(&self) -> bool {
        self.lower_titles.is_some()
    }

    /// Fuzzy-match `query_lower` against a specific track by index.
    ///
    /// Returns the character positions that match, or `None` when there is no match.
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

    /// Return the next visible index in the current display order after `current`.
    /// Wraps around to the first element.
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

    /// Return the previous visible index in the current display order before `current`.
    /// Wraps around to the last element.
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

    /// Toggle shuffle mode and mark the queue as dirty.
    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
        self.mark_queue_dirty();
    }
    /// Set the selected track index and ensure it is visible in the display.
    pub fn set_selected(&mut self, idx: usize) {
        self.selected = idx;
        self.ensure_selected_visible();
    }
    /// Return true if the library contains any tracks.
    pub fn has_tracks(&self) -> bool {
        !self.tracks.is_empty()
    }

    // Fuzzy/subsequence match: return the character positions (by char index)
    // in `title` that match the query, or None if not matched.
    /// Fuzzy/subsequence match: return the character positions in `title`
    /// that match `query`, or `None` if not matched.
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

    /// Enter filter mode: enable filtering and adjust cursor behavior.
    pub fn enter_filter_mode(&mut self) {
        self.filter_mode = true;
        self.follow_playback_off();
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }
    /// Exit filter mode and mark the queue dirty.
    pub fn exit_filter_mode(&mut self) {
        self.filter_mode = false;
        self.mark_queue_dirty();
    }
    /// Clear the active filter and restore selection visibility.
    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.filter_mode = false;
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }
    /// Append a character to the filter query and refresh view.
    pub fn push_filter_char(&mut self, c: char) {
        self.filter_query.push(c);
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }
    /// Remove the last character from the filter query and refresh view.
    pub fn pop_filter_char(&mut self) {
        self.filter_query.pop();
        self.mark_queue_dirty();
        self.ensure_selected_visible();
    }

    /// Ensure that `selected` is part of the current filtered/shuffled view,
    /// otherwise move selection to the first visible track.
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
    /// Move selection to the next visible track.
    pub fn next(&mut self) {
        if let Some(next) = self.next_in_view_from(self.selected) {
            self.selected = next;
        }
    }

    /// Move selection to the previous visible track.
    pub fn prev(&mut self) {
        if let Some(prev) = self.prev_in_view_from(self.selected) {
            self.selected = prev;
        }
    }
}
