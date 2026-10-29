use super::{
    super::output::StreamOutput,
    state::{Context, Control},
};
use codelet_common::debug_capture::{get_debug_capture_manager, CaptureOptions};
use std::{error::Error as StdError, sync::atomic::Ordering::Acquire};
use tracing::{error, info};

pub(super) fn handle<Output: StreamOutput>(
    error: anyhow::Error,
    context: &mut Context<'_, Output>,
) -> anyhow::Result<Control> {
    let error_str = error.to_string();
    error!(
        "API error received from provider: {} (messages={}, provider={})",
        error_str,
        context.session.messages.len(),
        context.session.current_provider_name()
    );
    let err_ref: &dyn StdError = error.as_ref();
    let mut source = err_ref.source();
    while let Some(cause) = source {
        error!("Caused by: {}", cause);
        source = cause.source();
    }
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture("api.error", serde_json::json!({
                    "error": error_str, "duration": context.api_start_time.elapsed().as_millis() as u64,
                }), Some(CaptureOptions { request_id: Some(context.request_id.clone()) }));
            }
        }
    }
    if !context.is_interrupted.load(Acquire) && context.session.current_provider_name() != "codex" {
        let stripped = super::super::recovery_unrecoverable::strip_failed_tool_call_tail(
            &mut context.session.messages,
        );
        if matches!(
            stripped,
            super::super::recovery_unrecoverable::StrippedTail::ToolPair
                | super::super::recovery_unrecoverable::StrippedTail::ToolCallOnly
        ) {
            context.session.token_tracker.cache_read_input_tokens = None;
            context.session.token_tracker.cache_creation_input_tokens = None;
            crate::interactive_helpers::recalculate_token_tracker(context.session);
            if let Ok(mut tokens) = context.token_state.lock() {
                tokens.compaction_needed = false;
                tokens.input_tokens = context.session.token_tracker.input_tokens;
                tokens.cache_read_input_tokens = 0;
                tokens.cache_creation_input_tokens = 0;
                tokens.output_tokens = 0;
            }
            info!("BUG-170: stripped failed tool-call tail ({stripped:?}) — session remains interactive");
            context.output.emit_error(&format!("{error_str}\n\n[Removed the failed tool call from context; send your next message to continue]"));
            return Ok(Control::Stop);
        }
    }
    context.output.emit_error(&error_str);
    Err(error.context("Agent error"))
}
