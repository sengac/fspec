use super::{
    super::{
        auto_continue::{apply_segment_outcome, ContinueDecision, AUTO_CONTINUE_NUDGE_PROMPT},
        output::{ContinueStateReason, StreamOutput},
    },
    state::{AgentStream, Context, Control, State},
};
use codelet_core::{RigAgent, StreamingTokenDisplay};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    message::{Message, UserContent},
    one_or_many::OneOrMany,
    wasm_compat::WasmCompatSend,
};
use std::sync::atomic::Ordering::Acquire;

pub(super) async fn complete<'agent, Model, Output>(
    agent: &'agent RigAgent<Model>,
    stream: &mut AgentStream<'agent, Model>,
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<Control>
where
    Model: CompletionModel,
    Model::StreamingResponse: WasmCompatSend + GetTokenUsage,
    Output: StreamOutput,
{
    let done_summary = codelet_tools::take_done_acceptance(context.session_id);
    if std::mem::take(&mut state.auto_continue_nudge_pending_refund) {
        context.session.continue_nudges_used = apply_segment_outcome(
            context.session.continue_nudges_used,
            state.auto_continue_segment_tool_calls,
        );
        if state.auto_continue_segment_tool_calls == 0 && done_summary.is_none() {
            state.goal_consecutive_zero_activity_nudges = state
                .goal_consecutive_zero_activity_nudges
                .saturating_add(1);
        } else {
            state.goal_consecutive_zero_activity_nudges = 0;
        }
        super::super::continue_state::emit_continue_state(
            context.session,
            context.output,
            ContinueStateReason::RefundSettled,
        );
    }
    let goal_active = context.session.goal.is_some();
    let decision = if goal_active {
        context.session.done_rejections = codelet_tools::done_rejection_count(context.session_id);
        super::super::auto_continue::decide_goal_continuation(
            done_summary.as_deref(),
            state.final_stop_reason.as_deref(),
            context.session.continue_nudges_used,
            super::super::goal::effective_goal_budget(context.session),
            context.session.done_rejections,
            state.goal_consecutive_zero_activity_nudges,
            context.is_interrupted.load(Acquire),
        )
    } else {
        super::super::auto_continue::decide_continuation(
            context.session.continue_enabled,
            done_summary.as_deref(),
            state.final_stop_reason.as_deref(),
            context.session.continue_nudges_used,
            context.session.continue_budget,
            context.is_interrupted.load(Acquire),
        )
    };
    match decision {
        ContinueDecision::Finish => {}
        ContinueDecision::FinishWithSummary(summary) => {
            super::super::done_early_exit::apply_finish_with_summary(
                context.session,
                context.session_id,
                &summary,
                context.output,
            );
        }
        ContinueDecision::FinishWithWarning(warning) => {
            context.output.emit_status(&warning);
            super::super::continue_state::emit_continue_state(
                context.session,
                context.output,
                ContinueStateReason::BudgetExhausted,
            );
        }
        ContinueDecision::Escalate(message) => {
            context.output.emit_status(&message);
            let _resolution =
                super::super::goal::raise_goal_escalation(context.session_id, &message);
        }
        ContinueDecision::Nudge => {
            context.session.continue_nudges_used += 1;
            state.auto_continue_nudge_pending_refund = true;
            state.auto_continue_segment_tool_calls = 0;
            tracing::info!(
                "CONT-002: auto-continue nudge {}/{} (stop_reason={:?})",
                context.session.continue_nudges_used,
                context.session.continue_budget,
                state.final_stop_reason.as_deref()
            );
            super::super::continue_state::emit_continue_state(
                context.session,
                context.output,
                ContinueStateReason::NudgeConsumed,
            );
            let nudge_hook = super::super::retry_compaction_hook(
                &context.token_state,
                context.session.token_tracker.input_tokens,
                0,
                context.threshold,
            );
            *stream = agent
                .prompt_streaming_with_history_and_hook(
                    AUTO_CONTINUE_NUDGE_PROMPT,
                    &mut context.session.messages,
                    nudge_hook,
                )
                .await
                .boxed();
            context.session.messages.push(Message::User {
                content: OneOrMany::one(UserContent::text(AUTO_CONTINUE_NUDGE_PROMPT)),
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
            return Ok(Control::Continue);
        }
    }
    context
        .output
        .emit_done_with_stop_reason(state.final_stop_reason.take());
    Ok(Control::Stop)
}
