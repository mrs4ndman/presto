//! Application module: exposes the app model used by the TUI and runtime.
//!
//! The `App` model lives in `app::model` and holds the current library,
//! selection and playback state.

mod model;

pub use model::*;

#[cfg(test)]
mod tests;
