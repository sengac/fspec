//! Agent Execution bounded context
//!
//! RigAgent for LLM communication and tool execution.
//! All tool execution uses rig::tool::Tool trait.

pub mod compaction;
pub mod compaction_hook;
pub mod rig_agent;
pub mod tool_specs;

pub use compaction_hook::{CompactionHook, TokenState};
pub use rig_agent::{RigAgent, DEFAULT_MAX_DEPTH};
pub use tool_specs::ToolSpec;

// Re-export common types for convenience
pub use codelet_common::web_search::{WebSearchAction, WebSearchBeginEvent, WebSearchEndEvent};
pub use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
