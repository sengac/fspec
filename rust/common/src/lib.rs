//! Common utilities for codelet
//!
//! This crate provides shared functionality used across all codelet crates:
//! - Data directory management (single source of truth)
//! - Logging infrastructure with file rotation
//! - Debug capture utilities
//! - Shared types for LLM conversations
//! - Token estimation using tiktoken-rs (PROV-002)
//! - File locking (proper-lockfile-compatible inter-process mkdir lock)

pub mod counts;
pub mod data_dir;
pub mod debug_capture;
pub mod file_lock;
pub mod fspec_config;
pub mod image_dimensions;
pub mod input_inbox;
pub mod logging;
pub mod panic_log;
pub mod project_root;
pub mod read_work;
pub mod saved_daemons;
pub mod spec_lock;
pub mod spec_paths;
pub mod token_estimator;
pub mod tool_usage;
pub mod types;
pub mod user_dir;
pub mod web_search;
pub mod workspace_registry;

// Re-export common types for convenience
pub use data_dir::{get_data_dir, set_data_directory};
pub use types::{ContentPart, ImageSource, Message, MessageContent, MessageRole, ToolResultPart};

pub use counts::narrow_count;
