//! Common utilities for codelet
//!
//! This crate provides shared functionality used across all codelet crates:
//! - Logging infrastructure with file rotation
//! - Debug capture utilities
//! - Shared types for LLM conversations
//! - Token estimation using tiktoken-rs (PROV-002)

pub mod debug_capture;
pub mod logging;
pub mod token_estimator;
pub mod types;
pub mod web_search;

// Re-export common types for convenience
pub use types::{ContentPart, Message, MessageContent, MessageRole};
