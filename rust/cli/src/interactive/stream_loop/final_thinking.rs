use super::{
    super::{output::StreamOutput, recovery_thinking::*},
    state::{AgentStream, Context, Control, State},
};
use codelet_core::{RigAgent, StreamingTokenDisplay};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, GetTokenUsage, Usage},
    message::{Message, UserContent},
    one_or_many::OneOrMany,
    wasm_compat::WasmCompatSend,
};
use std::sync::atomic::Ordering::Acquire;
use tracing::{debug, info, warn};

pub(super) async fn recover<'agent, Model, Output>(
    agent: &'agent RigAgent<Model>,
    stream: &mut AgentStream<'agent, Model>,
    context: &mut Context<'_, Output>,
    state: &mut State,
    usage: Usage,
) -> anyhow::Result<Control>
where
    Model: CompletionModel,
    Model::StreamingResponse: WasmCompatSend + GetTokenUsage,
    Output: StreamOutput,
{
    let reasoning_tokens = usage.reasoning_tokens.unwrap_or(0);
    let output_tokens = usage.output_tokens;
    let stop_reason_ref = state.final_stop_reason.as_deref();
    if context.is_interrupted.load(Acquire)
        || !(is_thinking_exhaustion(
            stop_reason_ref,
            reasoning_tokens,
            output_tokens,
            THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
        ) || is_thinking_only_exhaustion(
            stop_reason_ref,
            &state.accumulated_reasoning,
            &state.assistant_text,
            state.auto_continue_segment_tool_calls > 0,
        ))
    {
        return Ok(Control::Next);
    }
    state.thinking_exhaustion_retry_count += 1;
    if state.thinking_exhaustion_retry_count == 1 {
        context.session.thinking_exhaustion_cross_turn_count += 1;
    }
    info!("PROV-041: Thinking exhaustion detected (attempt {}/{}, cross-turn #{}): reasoning_tokens={}, output_tokens={}, stop_reason={:?}",
        state.thinking_exhaustion_retry_count, MAX_THINKING_EXHAUSTION_RETRIES,
        context.session.thinking_exhaustion_cross_turn_count, reasoning_tokens, output_tokens, stop_reason_ref);
    if state.thinking_exhaustion_retry_count == 1
        && context.session.thinking_exhaustion_cross_turn_count
            >= THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD
    {
        let previous_level = context.session.session_thinking_level;
        context.session.session_thinking_level =
            downgrade_thinking_level(context.session.session_thinking_level);
        context.session.thinking_exhaustion_cross_turn_count = 0;
        if context.session.session_thinking_level != previous_level {
            context.output.emit_status(&format!(
                "Reasoning preference for subsequent turns reduced to {:?} due to repeated thinking budget exhaustion; current retry settings are unchanged",
                context.session.session_thinking_level));
        }
    }
    if state.thinking_exhaustion_retry_count <= MAX_THINKING_EXHAUSTION_RETRIES {
        let captured_reasoning = if !state.accumulated_reasoning.is_empty() {
            Some(state.accumulated_reasoning.as_str())
        } else if !state.assistant_text.is_empty() {
            Some(state.assistant_text.as_str())
        } else {
            None
        };
        let recovery_msg = build_thinking_exhaustion_recovery_message(
            reasoning_tokens,
            output_tokens,
            captured_reasoning,
        );
        context.output.emit_status(&format!(
            "Thinking exhaustion detected (attempt {}/{MAX_THINKING_EXHAUSTION_RETRIES}). Retrying with a prompt-only concise-reasoning request; model settings and token limits are unchanged...",
            state.thinking_exhaustion_retry_count));
        if context.is_interrupted.load(Acquire) {
            return Ok(Control::Continue);
        }
        let current_tokens = context.session.token_tracker.input_tokens
            + context.session.token_tracker.output_tokens;
        let utilization_pct = if context.context_window > 0 {
            (current_tokens as f64 / context.context_window as f64) * 100.0
        } else {
            0.0
        };
        if utilization_pct > 90.0 {
            info!(
                "PROV-041: Context utilization at {:.1}% — persisting session state before retry",
                utilization_pct
            );
            context.output.emit_status(&format!(
                "Context window at {utilization_pct:.0}% — session state preserved before retry"
            ));
        }
        let retry_hook = super::super::retry_compaction_hook(
            &context.token_state,
            context.session.token_tracker.input_tokens,
            0,
            context.threshold,
        );
        debug!(
            "API REQUEST (thinking exhaustion recovery {}/{}) - Provider: {}, Model: {}",
            state.thinking_exhaustion_retry_count,
            MAX_THINKING_EXHAUSTION_RETRIES,
            context.session.current_provider_name(),
            context
                .session
                .current_model_id()
                .as_deref()
                .unwrap_or("NONE")
        );
        *stream = agent
            .prompt_streaming_with_history_and_hook(
                &recovery_msg,
                &mut context.session.messages,
                retry_hook,
            )
            .await
            .boxed();
        context.session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(&recovery_msg)),
        });
        state.assistant_text.clear();
        state.accumulated_reasoning.clear();
        state.final_stop_reason = None;
        state.tool_calls_buffer.clear();
        state.last_tool_name = None;
        state.turn_tool_infos.clear();
        state.tool_execution_in_progress = false;
        state.streaming_display = StreamingTokenDisplay::from_cache_inclusive_total(
            context.session.token_tracker.input_tokens,
            context.session.token_tracker.output_tokens,
            context
                .session
                .token_tracker
                .cache_read_input_tokens
                .unwrap_or(0),
            context
                .session
                .token_tracker
                .cache_creation_input_tokens
                .unwrap_or(0),
        )
        .with_prev_reasoning(state.prev_reasoning);
        debug!("[stream_loop] PROV-041: Created new stream for thinking exhaustion retry");
        return Ok(Control::Continue);
    }
    let budget_msg = build_thinking_budget_exhausted_message(MAX_THINKING_EXHAUSTION_RETRIES);
    warn!(
        "PROV-041: Thinking exhaustion retry budget exhausted after {} attempts",
        MAX_THINKING_EXHAUSTION_RETRIES
    );
    context.output.emit_status(&budget_msg);
    Ok(Control::Next)
}
