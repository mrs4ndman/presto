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
    Play(usize),
    Stop,
    TogglePause,
    ToggleShuffle,
    SetQueue(Vec<usize>),
    SetLoopMode(LoopMode),
    Next,
    Prev,
    Quit { fade_out_ms: u64 },
    SeekBy(i32), // seconds, positive or negative
}

#[derive(Debug, Clone)]
pub struct PlaybackInfo {
    pub index: Option<usize>,
    pub elapsed: Duration,
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
