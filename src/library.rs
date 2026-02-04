//! Library crate: scanning and track model for the music library.
//!
//! This module provides the `Track` model and a `scan` helper used to
//! discover audio files on disk.

mod display;
mod model;
mod scan;

pub use model::Track;
pub use scan::scan;

#[cfg(test)]
mod tests;
