use super::{
    super::output::StreamOutput,
    state::{Context, State},
};
use codelet_common::debug_capture::{get_debug_capture_manager, CaptureOptions};
use codelet_core::StreamingTokenDisplay;
use rig::{agent::FinalResponse, completion::Usage};
use tracing::debug;

pub(super) fn observed<Output: StreamOutput>(
    usage: Usage,
    context: &mut Context<'_, Output>,
    state: &mut State,
) {
    state.received(context.output);
    state
        .input_observation
        .observe(&mut context.session.token_tracker, &usage);
    if usage.output_tokens == 0 {
        state.streaming_display.start_new_segment(&usage);
    } else if let Some(update) = state.streaming_display.update_from_usage(&usage) {
        context.output.emit_tokens(&update.into());
        context
            .context_projection
            .emit(context.output, None, Some(usage));
    }
}

pub(super) fn final_response<Output: StreamOutput>(
    response: &FinalResponse,
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> Usage {
    state.received(context.output);
    debug!("[stream_loop] FinalResponse received - checking compaction_needed state");
    state.final_stop_reason = response.stop_reason().map(String::from);
    if let Some(reason) = &state.final_stop_reason {
        debug!("[stream_loop] FinalResponse stop_reason={}", reason);
    }
    let usage = response.usage();
    let request_context = response.request_context();
    let physical = request_context.map_or(usage, |request_context| {
        request_context.usage.unwrap_or_default()
    });
    state
        .input_observation
        .finish(&mut context.session.token_tracker, &physical);
    let final_update = if physical.input_tokens > 0
        || physical.cache_read_input_tokens.unwrap_or(0) > 0
        || physical.cache_creation_input_tokens.unwrap_or(0) > 0
        || usage.output_tokens > 0
        || usage.reasoning_tokens.unwrap_or(0) > 0
    {
        let display_usage = Usage {
            input_tokens: physical.input_tokens,
            cache_read_input_tokens: physical.cache_read_input_tokens,
            cache_creation_input_tokens: physical.cache_creation_input_tokens,
            ..usage
        };
        state.streaming_display = StreamingTokenDisplay::new(
            physical.input_tokens,
            state.finalized_output,
            physical.cache_read_input_tokens.unwrap_or(0),
            physical.cache_creation_input_tokens.unwrap_or(0),
        )
        .with_prev_reasoning(state.finalized_reasoning);
        state
            .streaming_display
            .update_from_final_response(&display_usage)
    } else {
        state.streaming_display.current()
    };
    state.finalized_output = final_update.output_tokens;
    state.finalized_reasoning = final_update.reasoning_tokens;
    context.output.emit_tokens(&final_update.into());
    context.context_projection.emit(
        context.output,
        request_context,
        request_context.and_then(|request_context| request_context.usage),
    );
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                let duration_ms = context.api_start_time.elapsed().as_millis() as u64;
                manager.capture("api.response.end", serde_json::json!({
                    "duration": duration_ms,
                    "aggregatedUsage": {
                        "inputTokens": usage.input_tokens, "outputTokens": usage.output_tokens,
                        "cacheReadInputTokens": usage.cache_read_input_tokens,
                        "cacheCreationInputTokens": usage.cache_creation_input_tokens,
                        "reasoningTokens": usage.reasoning_tokens,
                        "totalInputTokens": usage.input_tokens + usage.cache_read_input_tokens.unwrap_or(0)
                            + usage.cache_creation_input_tokens.unwrap_or(0),
                    },
                    "displayUsage": {
                        "inputTokens": final_update.input_tokens, "outputTokens": final_update.output_tokens,
                        "cacheReadInputTokens": final_update.cache_read_tokens,
                        "cacheCreationInputTokens": final_update.cache_creation_tokens,
                        "reasoningTokens": usage.reasoning_tokens, "totalInputTokens": final_update.total_input(),
                    },
                    "responseLength": state.assistant_text.len(),
                }), Some(CaptureOptions { request_id: Some(context.request_id.clone()) }));
                manager.capture("token.update", serde_json::json!({
                    "inputTokens": final_update.input_tokens, "outputTokens": final_update.output_tokens,
                    "cacheReadInputTokens": final_update.cache_read_tokens,
                    "cacheCreationInputTokens": final_update.cache_creation_tokens,
                    "reasoningTokens": usage.reasoning_tokens, "totalInputTokens": final_update.total_input(),
                    "totalOutputTokens": final_update.output_tokens,
                }), None);
            }
        }
    }
    usage
}
