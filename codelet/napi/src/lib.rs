//! Codelet NAPI-RS Native Module Bindings
//!
//! This module exposes codelet's Rust AI agent functionality to Node.js via NAPI-RS.
//! It enables fspec's Ink/React TUI to serve as the frontend for codelet.
//!
//! Uses the same streaming infrastructure as codelet-cli but with callbacks
//! instead of stdout printing.

#[cfg_attr(not(feature = "noop"), macro_use)]
extern crate napi_derive;

// These modules use ThreadsafeFunction and must be excluded in noop mode
#[cfg(not(feature = "noop"))]
mod output;
#[cfg(not(feature = "noop"))]
mod session;
#[cfg(not(feature = "noop"))]
mod types;

// Persistence module works in both modes (pure Rust with optional NAPI bindings)
pub mod persistence;

pub use persistence::*;

#[cfg(not(feature = "noop"))]
pub use session::CodeletSession;
#[cfg(not(feature = "noop"))]
pub use types::*;
