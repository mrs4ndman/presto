//! Audio-related small types and handles.
//!
//! This module defines common enums and type aliases used by the
//! audio subsystem (looping mode, commands, playback info and handles).

use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LoopMode {
    /// Do not wrap at the end of the current queue.
    NoLoop,
    /// Wrap around to the start of the current queue.
    LoopAll,
    /// Repeat the current song when it ends.
    LoopOne,
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::LoopAll
    }
}

#[derive(Debug)]
pub enum AudioCmd {
    /// Start playing the track at the given index.
    Play(usize),
    /// Stop playback immediately.
    Stop,
    /// Toggle pause/resume.
    TogglePause,
    /// Toggle shuffle mode in the audio thread.
    ToggleShuffle,
    /// Set the current queue/order to the provided indices.
    SetQueue(Vec<usize>),
    /// Set the loop mode used by the player.
    SetLoopMode(LoopMode),
    /// Skip to the next track.
    Next,
    /// Go to the previous track.
    Prev,
    /// Quit the audio thread, optionally fading out over `fade_out_ms` milliseconds.
    Quit { fade_out_ms: u64 },
    /// Seek by the specified number of seconds (positive or negative).
    SeekBy(i32), // seconds, positive or negative
}

#[derive(Debug, Clone)]
/// Runtime playback information shared with the UI.
pub struct PlaybackInfo {
    /// Currently playing track index in the library (if any).
    pub index: Option<usize>,
    /// Elapsed playback time for the current track.
    pub elapsed: Duration,
    /// Whether playback is currently active.
    pub playing: bool,
}

impl Default for PlaybackInfo {
    fn default() -> Self {
        Self {
            index: None,
            elapsed: Duration::ZERO,
            playing: false,
        }
    }
}

pub type PlaybackHandle = Arc<Mutex<PlaybackInfo>>;
pub type OrderHandle = Arc<Mutex<Vec<usize>>>;
