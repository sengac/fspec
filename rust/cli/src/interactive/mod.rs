//! Interactive streaming engine and recovery helpers
//!
//! Shared by `codelet-agent-loop`, `codelet-napi`, and `codelet-fspec-tui`.

pub mod auto_continue; // CONT-002: auto-continue decision fn + /continue grammar
pub mod continue_state; // CONT-007: live continue/goal counter snapshot builders
pub mod done_early_exit; // CONT-005: done() immediate termination — early-exit decision + shared teardown
mod error_classifiers;
mod gemini_continuation;
pub mod goal; // CONT-003: goal mode — derived mode, /goal grammar, escalation
mod message_helpers;
mod multimodal;
pub mod output;
mod recovery_compaction;
mod recovery_image;
mod recovery_network;
mod recovery_stall;
mod recovery_thinking;
mod recovery_truncation;
mod stream_handlers;
pub mod stream_loop;

pub use error_classifiers::{
    classify_compaction_branch, is_image_content_error, is_prompt_too_long_error,
    is_stall_timeout_error, is_transient_network_error, is_truncated_tool_call_error,
    CompactionBranch, CompactionDisagreement,
};
pub use multimodal::{build_user_content_with_images, BridgeImage};
pub use output::{
    CliOutput, ContextFillInfo, ContinueStateEvent, ContinueStateReason, StreamEvent, StreamOutput,
    TokenInfo, ToolCallEvent, ToolResultEvent,
};
pub use recovery_compaction::{
    begin_compaction_recovery, build_compaction_budget_exhausted_message, compaction_retry_prompt,
    execute_compaction_and_capture_events, flush_partial_state_before_compaction,
    CompactionRecoveryPolicy, MAX_COMPACTION_RETRIES,
};
pub use recovery_image::sanitize_image_content;
pub use recovery_network::{network_retry_delay, MAX_NETWORK_RETRIES};
pub use recovery_stall::{
    build_deep_search_timeout_message, build_stall_timeout_message, deep_search_wall_clock_timeout,
    stall_timeout_duration, DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS, STALL_TIMEOUT_ERROR_PREFIX,
    STALL_TIMEOUT_SECS,
};
pub use recovery_thinking::{
    build_thinking_budget_exhausted_message, build_thinking_exhaustion_recovery_message,
    downgrade_thinking_level, is_thinking_exhaustion, MAX_THINKING_EXHAUSTION_RETRIES,
    THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD, THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
};
pub use recovery_truncation::{
    build_truncation_budget_exhausted_message, build_truncation_recovery_message,
    MAX_TRUNCATION_RETRIES,
};
pub use stream_loop::{run_agent_stream, run_agent_stream_with_images};
