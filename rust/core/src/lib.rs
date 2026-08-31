//! Agent Execution bounded context
//!
//! RigAgent for LLM communication and tool execution.
//! All tool execution uses rig::tool::Tool trait.

pub mod compaction;
pub mod compaction_hook;
pub mod file_search;
pub mod gemini_history_hook;
pub mod history_strip;
pub mod lifecycle_hooks;
pub mod loops;
pub mod message_estimator;
pub mod persistence;
pub mod rig_agent;
pub mod scheduler;
pub mod session_manager_handle;
pub mod streaming_display;
pub mod token_usage;
pub mod tool_specs;
pub mod work_units;
pub mod work_units_write;

pub use compaction_hook::{CompactionHook, TokenState};
pub use gemini_history_hook::{
    ensure_thought_signatures, GeminiHistoryHook, SYNTHETIC_THOUGHT_SIGNATURE,
};
pub use message_estimator::estimate_messages_tokens;
pub use history_strip::strip_reasoning_from_history;
pub use rig_agent::{RigAgent, DEFAULT_MAX_DEPTH};
pub use streaming_display::{
    DisplayThrottle, OutputTokenTracker, StreamingTokenDisplay, TokPerSecCalculator,
    TokenDisplayUpdate,
};
pub use token_usage::ApiTokenUsage;
pub use tool_specs::ToolSpec;

// RPC-042: re-export `SessionManagerHandle` at the crate root so the
// production impl in `rust/sessions/src/handle_impl.rs` (and any
// future consumers) can name it via the short path
// `codelet_core::SessionManagerHandle` rather than the long
// `codelet_core::session_manager_handle::SessionManagerHandle`.
pub use session_manager_handle::SessionManagerHandle;

// Re-export turn completion facade from codelet-tools for convenience
pub use codelet_tools::facade::{
    ContinuationStrategy, DefaultTurnCompletionFacade, GeminiTurnCompletionFacade,
    TurnCompletionFacade,
};

// Re-export token_estimator from codelet-common for backwards compatibility
pub use codelet_common::token_estimator;

// Re-export common types for convenience
pub use codelet_common::web_search::{WebSearchAction, WebSearchBeginEvent, WebSearchEndEvent};
pub use codelet_common::{ContentPart, Message, MessageContent, MessageRole};
