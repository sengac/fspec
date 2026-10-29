use super::{
    super::output::StreamOutput,
    state::{Context, State},
};
use codelet_core::ApiTokenUsage;
use std::sync::atomic::Ordering::Acquire;
use tracing::warn;

pub(super) async fn finish<Output: StreamOutput>(
    context: &mut Context<'_, Output>,
    state: &State,
) -> anyhow::Result<()> {
    codelet_tools::set_tool_progress_callback(context.session_id, None);
    let post_loop_needs_recovery = !state.summary_handoff
        && !context.is_interrupted.load(Acquire)
        && context
            .token_state
            .lock()
            .map(|tokens| tokens.compaction_needed)
            .unwrap_or(false);
    if post_loop_needs_recovery {
        warn!(
            "[stream_loop] CMPCT-032: POST-LOOP safety net fired — loop exited with \
            compaction_needed=true without invoking the in-loop macro. This indicates \
            a break path bypassed both the FinalResponse compaction check and the \
            error-arm in-loop macro (likely a thinking-exhaustion retry, stream-end \
            with a late-firing hook, or a new break path added without compaction \
            handling). Running compaction in place so the next user turn does not \
            exceed the context window."
        );
        let mut post_loop_text = String::new();
        match super::super::recovery_compaction::begin_compaction_recovery(
            context.session,
            &context.token_state,
            &state.streaming_display,
            &mut post_loop_text,
            context.output,
            false,
        ) {
            Ok(_policy) => {
                if let Err(error) =
                    super::super::recovery_compaction::execute_compaction_and_capture_events(
                        context.session,
                        context.compaction_in_progress.clone(),
                        context.prompt,
                        context.threshold,
                        context.context_window,
                        &context.token_state,
                        context.output,
                    )
                    .await
                {
                    warn!(
                        "[stream_loop] CMPCT-032: POST-LOOP execute_compaction failed: {error}; \
                        flag remains set — next turn may still exceed context window"
                    );
                } else if let Ok(mut tokens) = context.token_state.lock() {
                    tokens.compaction_needed = false;
                    tokens.input_tokens = context.session.token_tracker.input_tokens;
                    tokens.cache_read_input_tokens = 0;
                    tokens.cache_creation_input_tokens = 0;
                    tokens.output_tokens = 0;
                }
            }
            Err(error) => {
                warn!("[stream_loop] CMPCT-032: POST-LOOP begin_compaction_recovery failed: {error}; \
                    flag remains set — next turn may still exceed context window");
            }
        }
    }
    if !context.is_interrupted.load(Acquire) {
        let final_display = state.streaming_display.current();
        let per_turn_output_delta = context
            .session
            .token_tracker
            .compute_output_delta(final_display.output_tokens);
        let final_usage = ApiTokenUsage::new(
            final_display.input_tokens,
            final_display.cache_read_tokens,
            final_display.cache_creation_tokens,
            per_turn_output_delta,
        )
        .with_reasoning_tokens(final_display.reasoning_tokens);
        tracing::debug!(
            "CMPCT-001: Before update: cumulative_billed_input={}, final_display.input_tokens={}",
            context.session.token_tracker.cumulative_billed_input,
            final_display.input_tokens
        );
        context
            .session
            .token_tracker
            .update_display_only(&final_usage, final_display.output_tokens);
        let settled = codelet_core::persistence::load_session(context.session_id)
            .map_err(anyhow::Error::msg)?;
        context.session.token_tracker.cumulative_billed_input =
            settled.token_usage.cumulative_billed_input;
        context.session.token_tracker.cumulative_billed_output =
            settled.token_usage.cumulative_billed_output;
        tracing::debug!(
            "CMPCT-001: After update: cumulative_billed_input={}, cumulative_billed_output={}",
            context.session.token_tracker.cumulative_billed_input,
            context.session.token_tracker.cumulative_billed_output
        );
    }
    Ok(())
}
