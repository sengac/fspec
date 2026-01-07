//! Agent Execution bounded context
//!
//! RigAgent for LLM communication and tool execution.
//! All tool execution uses rig::tool::Tool trait.

pub mod compaction;
pub mod compaction_hook;
pub mod message_estimator;
pub mod rig_agent;
pub mod token_usage;
pub mod tool_specs;

pub use compaction_hook::{CompactionHook, TokenState};
pub use message_estimator::estimate_messages_tokens;
pub use token_usage::ApiTokenUsage;
pub use rig_agent::{RigAgent, DEFAULT_MAX_DEPTH};
pub use tool_specs::ToolSpec;

// Re-export token_estimator from codelet-common for backwards compatibility
pub use codelet_common::token_estimator;

// Re-export common types for convenience
pub use codelet_common::web_search::{WebSearchAction, WebSearchBeginEvent, WebSearchEndEvent};
pub use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
