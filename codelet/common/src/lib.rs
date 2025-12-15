//! Common utilities for codelet
//!
//! This crate provides shared functionality used across all codelet crates:
//! - Logging infrastructure with file rotation
//! - Debug capture utilities
//! - Shared types for LLM conversations

pub mod debug_capture;
pub mod logging;
pub mod types;

// Re-export common types for convenience
pub use types::{ContentPart, Message, MessageContent, MessageRole};
