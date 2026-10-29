use super::{
    super::{
        error_classifiers::{
            classify_compaction_branch, extract_prompt_cancelled, is_context_overflow_error,
            is_stall_timeout_error, CompactionBranch,
        },
        message_helpers::add_assistant_tool_calls_message,
        output::StreamOutput,
    },
    state::{AgentStream, Context, State},
};
use crate::interactive_helpers::{
    convert_messages_to_turns, inject_synthetic_tool_results_for_orphans,
    reconcile_session_messages,
};
use codelet_core::RigAgent;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    wasm_compat::WasmCompatSend,
};
use tracing::{debug, error, info, warn};

pub(super) async fn recover<'agent, Model, Output>(
    error: &anyhow::Error,
    agent: &'agent RigAgent<Model>,
    stream: &mut AgentStream<'agent, Model>,
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<bool>
where
    Model: CompletionModel,
    Model::StreamingResponse: WasmCompatSend + GetTokenUsage,
    Output: StreamOutput,
{
    let branch = classify_compaction_branch(error, &context.token_state);
    debug!("[stream_loop] Error classification: branch={:?}", branch);
    if matches!(branch, CompactionBranch::Recover { .. }) {
        if let Some(rig_chat_history) = extract_prompt_cancelled(error) {
            debug!(rig_history_len = rig_chat_history.len(),
                "[stream_loop] CMPCT-029: reconciling session.messages with rig PromptCancelled chat_history");
            reconcile_session_messages(&mut context.session.messages, rig_chat_history);
        } else {
            debug!("[stream_loop] CMPCT-029: PromptCancelled payload not found in error chain; skipping reconcile");
        }
        if !state.tool_calls_buffer.is_empty() {
            debug!(buffer_len = state.tool_calls_buffer.len(),
                "[stream_loop] CMPCT-029: draining tool_calls_buffer into session.messages (site 486 recovery)");
            add_assistant_tool_calls_message(
                &mut context.session.messages,
                state.tool_calls_buffer.clone(),
            )?;
            state.tool_calls_buffer.clear();
        }
        let injected = inject_synthetic_tool_results_for_orphans(&mut context.session.messages);
        if injected > 0 {
            warn!(injected, "[stream_loop] CMPCT-029: injected {} synthetic cancelled tool_result(s) before compaction", injected);
        }
        debug!("[stream_loop] CMPCT-023: invoking begin_compaction_recovery (Path C, pop_user_prompt=true)");
        let policy = super::super::recovery_compaction::begin_compaction_recovery(
            context.session,
            &context.token_state,
            &state.streaming_display,
            &mut state.assistant_text,
            context.output,
            true,
        )?;
        debug!(policy = ?policy, "[stream_loop] CMPCT-027: in-loop compaction restart (Path C)");
        super::restart::compact(agent, stream, context, state, policy).await?;
        return Ok(true);
    }
    let error_str = error.to_string();
    if is_stall_timeout_error(&error_str) {
        error!("AMGR-016: Stall timeout reached error classifier cascade (unexpected path)");
        context.output.emit_error(&error_str);
        return Ok(false);
    }
    let is_prompt_too_long = is_context_overflow_error(error);
    let has_compactable_turns = !convert_messages_to_turns(&context.session.messages).is_empty();
    if is_prompt_too_long && has_compactable_turns {
        info!("Received 'prompt is too long' error, triggering recovery compaction");
        debug!("[stream_loop] CMPCT-023: invoking begin_compaction_recovery (Path B, pop_user_prompt=true)");
        let policy = super::super::recovery_compaction::begin_compaction_recovery(
            context.session,
            &context.token_state,
            &state.streaming_display,
            &mut state.assistant_text,
            context.output,
            true,
        )?;
        debug!(policy = ?policy, "[stream_loop] CMPCT-027: in-loop compaction restart (Path B)");
        super::restart::compact(agent, stream, context, state, policy).await?;
        return Ok(true);
    }
    let is_context_overflow = is_context_overflow_error(error);
    if is_context_overflow && has_compactable_turns {
        info!("Received context-overflow error ({error_str}), triggering overflow recovery");
        debug!("[stream_loop] CMPCT-044: invoking begin_compaction_recovery (overflow, pop_user_prompt=true)");
        let policy = super::super::recovery_compaction::begin_compaction_recovery(
            context.session,
            &context.token_state,
            &state.streaming_display,
            &mut state.assistant_text,
            context.output,
            true,
        )?;
        debug!(policy = ?policy, compactor = state.compaction_retry_count > 0,
            "[stream_loop] CMPCT-027: in-loop compaction restart (overflow)");
        super::restart::compact(agent, stream, context, state, policy).await?;
        return Ok(true);
    }
    Ok(false)
}
