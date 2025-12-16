//! Agent Execution bounded context
//!
//! RigAgent for LLM communication and tool execution.
//! All tool execution uses rig::tool::Tool trait.

pub mod compaction;
pub mod rig_agent;

pub use rig_agent::{RigAgent, DEFAULT_MAX_DEPTH};

// Re-export common types for convenience
pub use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
