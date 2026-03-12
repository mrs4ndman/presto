//! Application model types: `App` and `PlaybackState`.
//!
//! The `App` struct holds the current library, selected track and playback
//! related flags used by the UI and runtime.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::audio::{LoopMode, PlaybackHandle};
use crate::library::{Lyrics, Track};

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
    pub volume: f32,
    pub initial_volume: f32,

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
    pub notice: Option<String>,
    pub pending_count: Option<u32>,
    pub controls_popup: bool,
    pub lyrics_popup: bool,

    current_track_lyrics_index: Option<usize>,
    current_track_lyrics: Option<Lyrics>,
    lyrics_cache: HashMap<PathBuf, Option<Lyrics>>,
}

impl App {
    /// Toggle the metadata popup visibility.
    pub fn toggle_metadata_window(&mut self) {
        self.metadata_window = !self.metadata_window;
    }
    /// Toggle the controls popup visibility.
    pub fn toggle_controls_popup(&mut self) {
        self.controls_popup = !self.controls_popup;
    }
    /// Toggle the lyrics popup visibility.
    pub fn toggle_lyrics_popup(&mut self) {
        self.lyrics_popup = !self.lyrics_popup;
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
            volume: 1.0,
            initial_volume: 1.0,

            // Optional computation help ↑
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
            notice: None,
            pending_count: None,
            controls_popup: false,
            lyrics_popup: false,
            current_track_lyrics_index: None,
            current_track_lyrics: None,
            lyrics_cache: HashMap::new(),
        }
    }

    /// Set a user-facing notice message.
    pub fn set_notice(&mut self, message: String) {
        self.notice = Some(message);
    }

    /// Clear any user-facing notice.
    pub fn clear_notice(&mut self) {
        self.notice = None;
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

    /// Disable follow-playback and clear any pending follow index (free-roam on)
    pub fn follow_playback_off(&mut self) {
        self.follow_playback = false;
        self.pending_follow_index = None;
    }

    /// Return current volume as 0.0-1.0 scalar.
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Return current volume rounded to a whole percent.
    pub fn volume_percent(&self) -> u8 {
        (self.volume * 100.0).round().clamp(0.0, 100.0) as u8
    }

    /// Set the current volume using a scalar (0.0-1.0), clamping out-of-range values.
    pub fn set_volume(&mut self, v: f32) -> f32 {
        let clamped = v.clamp(0.0, 1.0);
        self.volume = clamped;
        clamped
    }

    /// Set the initial volume (and current volume) using a percentage (0-100).
    pub fn set_initial_volume_percent(&mut self, pct: u8) -> f32 {
        let v = (pct as f32) / 100.0;
        let clamped = v.clamp(0.0, 1.0);
        self.initial_volume = clamped;
        self.volume = clamped;
        clamped
    }

    /// Reset current volume back to the stored initial volume.
    pub fn reset_volume_to_initial(&mut self) -> f32 {
        let v = self.initial_volume;
        self.volume = v;
        v
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

    /// Update the cached lyrics for the currently playing track.
    pub fn sync_current_track_lyrics(&mut self, track_index: Option<usize>) {
        if self.current_track_lyrics_index == track_index {
            return;
        }

        self.current_track_lyrics_index = track_index;
        self.current_track_lyrics = track_index.and_then(|idx| {
            let path = self.tracks.get(idx)?.path.clone();

            if let Some(cached) = self.lyrics_cache.get(&path) {
                return cached.clone();
            }

            let lyrics = crate::library::load_lyrics_from_path(&path);
            self.lyrics_cache.insert(path, lyrics.clone());
            lyrics
        });
    }

    /// Clear any cached current-track lyrics view state.
    pub fn clear_current_track_lyrics(&mut self) {
        self.current_track_lyrics_index = None;
        self.current_track_lyrics = None;
        self.lyrics_popup = false;
    }

    /// Return the current playing index for which lyrics are stored.
    pub fn current_track_lyrics_index(&self) -> Option<usize> {
        self.current_track_lyrics_index
    }

    /// Return the currently cached lyrics for the playing track.
    pub fn current_track_lyrics(&self) -> Option<&Lyrics> {
        self.current_track_lyrics.as_ref()
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
        if query.trim().is_empty() {
            return Some(Vec::new());
        }

        let query_terms = Self::query_terms(query);
        if query_terms.is_empty() {
            return None;
        }

        let title_lower = title.to_ascii_lowercase();
        Self::fuzzy_match_positions_wordwise(&title_lower, &query_terms)
    }

    fn query_terms(query: &str) -> Vec<String> {
        query
            .split_whitespace()
            .map(|term| term.to_ascii_lowercase())
            .filter(|term| !term.is_empty())
            .collect()
    }

    fn title_words_with_positions(text: &str) -> Vec<(String, Vec<usize>)> {
        let mut words: Vec<(String, Vec<usize>)> = Vec::new();
        let mut current = String::new();
        let mut positions: Vec<usize> = Vec::new();

        for (idx, ch) in text.chars().enumerate() {
            if ch.is_whitespace() {
                if !current.is_empty() {
                    words.push((std::mem::take(&mut current), std::mem::take(&mut positions)));
                }
                continue;
            }

            current.push(ch);
            positions.push(idx);
        }

        if !current.is_empty() {
            words.push((current, positions));
        }

        words
    }

    fn subsequence_positions(haystack: &str, needle: &str) -> Option<Vec<usize>> {
        if needle.is_empty() {
            return Some(Vec::new());
        }

        let mut positions: Vec<usize> = Vec::new();
        let mut haystack_iter = haystack.chars().enumerate();

        for qc in needle.chars() {
            loop {
                match haystack_iter.next() {
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

    fn fuzzy_match_positions_wordwise(
        title_lower: &str,
        query_terms: &[String],
    ) -> Option<Vec<usize>> {
        let title_words = Self::title_words_with_positions(title_lower);
        let mut title_word_index = 0usize;
        let mut matches: Vec<usize> = Vec::new();

        for term in query_terms {
            let mut matched = false;

            while title_word_index < title_words.len() {
                let (word, positions) = &title_words[title_word_index];
                title_word_index += 1;

                if let Some(word_matches) = Self::subsequence_positions(word, term) {
                    matches.extend(word_matches.into_iter().map(|idx| positions[idx]));
                    matched = true;
                    break;
                }
            }

            if !matched {
                return None;
            }
        }

        Some(matches)
    }

    /// Lowercase-only fuzzy match optimized for pre-lowered strings.
    fn fuzzy_match_positions_lower(title_lower: &str, query_lower: &str) -> Option<Vec<usize>> {
        if query_lower.trim().is_empty() {
            return Some(Vec::new());
        }

        let query_terms = Self::query_terms(query_lower);
        if query_terms.is_empty() {
            return None;
        }

        Self::fuzzy_match_positions_wordwise(title_lower, &query_terms)
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
    ///
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
