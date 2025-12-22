//! Codelet NAPI-RS Native Module Bindings
//!
//! This module exposes codelet's Rust AI agent functionality to Node.js via NAPI-RS.
//! It enables fspec's Ink/React TUI to serve as the frontend for codelet.
//!
//! Uses the same streaming infrastructure as codelet-cli but with callbacks
//! instead of stdout printing.

#[macro_use]
extern crate napi_derive;

mod output;
pub mod persistence;
mod session;
mod types;

pub use persistence::*;
pub use session::CodeletSession;
pub use types::*;
