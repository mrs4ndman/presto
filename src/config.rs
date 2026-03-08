//! Configuration loader and schema types.
//!
//! This module exposes the configuration schema used to drive runtime
//! behavior and helpers to load configuration from disk.

pub mod load;
mod schema;

pub use schema::*;

#[cfg(test)]
mod tests;
