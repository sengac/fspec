use super::{
    super::{
        gemini_continuation::{handle_gemini_continuation, GeminiContinuationResult},
        output::StreamOutput,
        stream_handlers::handle_final_response,
    },
    annotations::process_turn_annotations,
    state::{AgentStream, Context, Control, State},
};
use codelet_core::RigAgent;
use rig::{
    agent::FinalResponse,
    completion::{CompletionModel, GetTokenUsage},
    wasm_compat::WasmCompatSend,
};
use std::sync::atomic::Ordering::Acquire;
use tracing::{debug, warn};

pub(super) async fn handle<'agent, Model, Output>(
    response: FinalResponse,
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
    let mut completion_usage = super::usage::final_response(&response, context, state);
    match handle_gemini_continuation(
        agent,
        context.session,
        context.output,
        &context.is_interrupted,
        &mut context.input_queue,
        &context.token_state,
        context.threshold,
        &state.assistant_text,
        &state.streaming_display,
        context.session_id,
    )
    .await?
    {
        GeminiContinuationResult::NoContinuation => {}
        GeminiContinuationResult::Completed {
            text,
            display,
            usage,
            tool_calls,
            stop_reason,
        } => {
            state.assistant_text = text;
            state.streaming_display = display;
            completion_usage = usage;
            state.final_stop_reason = stop_reason;
            state.auto_continue_segment_tool_calls += tool_calls;
            let completed = state.streaming_display.current();
            state.finalized_output = completed.output_tokens;
            state.finalized_reasoning = completed.reasoning_tokens;
        }
        GeminiContinuationResult::Finished { display } => {
            state.streaming_display = display;
            return Ok(Control::Stop);
        }
        GeminiContinuationResult::CompactionNeeded(policy) => {
            debug!(policy = ?policy, "[stream_loop] CMPCT-027: in-loop compaction restart (Path D — Gemini continuation)");
            state.assistant_text.clear();
            super::restart::compact(agent, stream, context, state, policy).await?;
            return Ok(Control::Continue);
        }
    }
    handle_final_response(&state.assistant_text, &mut context.session.messages)?;
    match super::final_thinking::recover(agent, stream, context, state, completion_usage).await? {
        Control::Next => {}
        control => return Ok(control),
    }
    process_turn_annotations(
        context.session,
        &mut state.turn_tool_infos,
        &mut state.previous_turn_tool_infos,
    );
    let final_response_needs_recovery = !context.is_interrupted.load(Acquire)
        && context
            .token_state
            .lock()
            .map(|tokens| tokens.compaction_needed)
            .unwrap_or(false);
    if final_response_needs_recovery {
        warn!(
            "[stream_loop] CMPCT-032: FinalResponse branch exiting with \
            compaction_needed=true — running recovery before emit_done \
            to prevent silent context-window exhaustion on the next turn"
        );
        state.assistant_text.clear();
        let policy = super::super::recovery_compaction::begin_compaction_recovery(
            context.session,
            &context.token_state,
            &state.streaming_display,
            &mut state.assistant_text,
            context.output,
            false,
        )?;
        debug!(policy = ?policy, "[stream_loop] CMPCT-032: in-loop compaction restart (FinalResponse clean-exit)");
        super::restart::compact(agent, stream, context, state, policy).await?;
        return Ok(Control::Continue);
    }
    super::final_completion::complete(agent, stream, context, state).await
}
