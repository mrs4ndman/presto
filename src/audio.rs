//! Audio subsystem: player, queue and sink management.
//!
//! This module re-exports the `AudioPlayer` and audio-related types used
//! by the rest of the application.

mod player;
mod queue;
mod sink;
mod thread;
mod types;

pub use player::AudioPlayer;
pub use types::*;

#[cfg(test)]
mod tests;
