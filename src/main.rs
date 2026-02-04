//! Binary entry for presto — a small terminal music player.
//!
//! This crate contains the `main` function which initializes and runs
//! the runtime for the application.

mod app;
mod audio;
mod config;
mod library;
mod mpris;
mod runtime;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    runtime::run()
}
