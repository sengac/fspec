//! Interactive TUI mode entry point
//!
//! Main REPL loop coordinating terminal events, agent streaming, and user input.
//! Based on OpenAI codex architecture with tokio::select! pattern.

mod agent_runner;
mod error_classifiers;
mod gemini_continuation;
mod message_helpers;
mod multimodal;
pub mod output;
mod recovery_compaction;
mod recovery_image;
mod recovery_network;
mod recovery_stall;
mod recovery_thinking;
mod recovery_truncation;
mod repl_loop;
mod stream_handlers;
pub mod stream_loop;

pub use error_classifiers::{
    classify_compaction_branch, is_image_content_error, is_prompt_too_long_error,
    is_stall_timeout_error, is_transient_network_error, is_truncated_tool_call_error,
    CompactionBranch, CompactionDisagreement,
};
pub use multimodal::{build_user_content_with_images, BridgeImage};
pub use output::{
    CliOutput, ContextFillInfo, StreamEvent, StreamOutput, TokenInfo, ToolCallEvent,
    ToolResultEvent,
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

use crate::session::Session;
use anyhow::Result;
use repl_loop::repl_loop;

/// MODEL-001: Interactive mode now accepts optional model string (provider/model-id format)
pub async fn run_interactive_mode(
    provider_name: Option<&str>,
    model_string: Option<&str>,
) -> Result<()> {
    use codelet_providers::ProviderManager;

    // MODEL-001: Initialize session with model support if model is specified
    let mut session = if let Some(model) = model_string {
        // Use async model support for dynamic model selection
        let mut mgr = ProviderManager::with_model_support().await?;
        mgr.select_model(model)?;
        Session::from_provider_manager(mgr)
    } else {
        // Initialize session with persistent context (CLI-008)
        Session::new(provider_name)?
    };

    // CLI-016: Inject context reminders (CLAUDE.md discovery + environment info)
    session.inject_context_reminders();

    // Display startup card
    display_startup_card(&session)?;

    // Main REPL loop (raw mode is enabled/disabled per-request, not globally)
    let result = repl_loop(&mut session).await;

    result
}

fn display_startup_card(session: &Session) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    println!("\nfspec v{version}");

    let manager = session.provider_manager();
    if !manager.has_any_provider() {
        println!("Available models: No providers configured");
        println!("Please set ANTHROPIC_API_KEY, OPENAI_API_KEY, or other credentials\n");
    } else {
        let providers = manager.list_available_providers();
        println!("Available models: {}\n", providers.join(", "));
    }

    Ok(())
}
