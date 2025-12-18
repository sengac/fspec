//! Codelet NAPI-RS Native Module Bindings
//!
//! This module exposes codelet's Rust AI agent functionality to Node.js via NAPI-RS.
//! It enables fspec's Ink/React TUI to serve as the frontend for codelet.

#[macro_use]
extern crate napi_derive;

mod session;
mod types;

pub use session::CodeletSession;
pub use types::*;
