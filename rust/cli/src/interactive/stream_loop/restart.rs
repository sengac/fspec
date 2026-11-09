use super::{
    super::{
        output::StreamOutput,
        recovery_compaction::{CompactionRecoveryPolicy, MAX_COMPACTION_RETRIES},
    },
    state::{AgentStream, Context, State},
};
use codelet_core::{CompactionHook, RigAgent, StreamingTokenDisplay};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    message::Message,
    wasm_compat::WasmCompatSend,
};
use std::sync::Arc;
use tracing::{debug, warn};

/// BUG-641: what started an in-turn compaction. A threshold compaction whose
/// builder fails carries on uncompacted while the context fits; a provider
/// overflow means the context does not fit, so its failure holds the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Trigger {
    Threshold,
    Overflow,
}

pub(super) async fn compact<'agent, Model, Output>(
    agent: &'agent RigAgent<Model>,
    stream: &mut AgentStream<'agent, Model>,
    context: &mut Context<'_, Output>,
    state: &mut State,
    policy: CompactionRecoveryPolicy,
    trigger: Trigger,
) -> anyhow::Result<()>
where
    Model: CompletionModel,
    Model::StreamingResponse: WasmCompatSend + GetTokenUsage,
    Output: StreamOutput,
{
    let use_compactor_sub_agent =
        crate::compactor_sub_agent::has_compactor_sub_agent_handler(context.session_id);
    state.compaction_retry_count += 1;
    if state.compaction_retry_count > MAX_COMPACTION_RETRIES {
        let message = super::super::recovery_compaction::build_compaction_budget_exhausted_message(
            MAX_COMPACTION_RETRIES,
        );
        warn!(
            "CMPCT-027: Compaction retry budget exhausted after {} attempts",
            MAX_COMPACTION_RETRIES
        );
        context.output.emit_error(&message);
        return Err(anyhow::anyhow!(
            "Compaction retry budget exhausted after {MAX_COMPACTION_RETRIES} attempts"
        ));
    }
    let policy_prompt = super::super::recovery_compaction::compaction_retry_prompt(policy);
    let mut retry_prompt = if use_compactor_sub_agent {
        format!("{}\n\n{policy_prompt}", context.prompt)
    } else {
        policy_prompt.to_owned()
    };
    if use_compactor_sub_agent {
        let admitted_prompt = retry_prompt.clone();
        context
            .session
            .install_selected_preparer(context.preparer.clone().map_history(move |messages| {
                let mut history = messages.to_vec();
                history.push(Message::user(admitted_prompt.clone()));
                history
            }));
        let round = crate::compactor_sub_agent::run_compactor_sub_agent_round_cancellable(
            context.session,
            context.session_id,
            &context.token_state,
            &context.compaction_in_progress,
            context.output,
            &context.compaction_cancellation,
        )
        .await;
        match round {
            Ok(()) => context.output.emit_compaction_continuing(),
            Err(failure) if trigger == Trigger::Threshold => {
                // BUG-641: the history still holds the user prompt unless
                // recovery popped it before any tool round consumed it.
                let consumed = matches!(
                    context.session.messages.get(context.request_history_len),
                    Some(Message::User { .. })
                );
                retry_prompt = if consumed {
                    policy_prompt.to_owned()
                } else {
                    format!("{}\n\n{policy_prompt}", context.prompt)
                };
                let tracked = context
                    .token_state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("token state lock poisoned"))?
                    .total();
                let occupied = super::uncompacted::occupied_tokens(
                    tracked,
                    context.session.inference_messages(),
                    &retry_prompt,
                );
                context
                    .session
                    .install_selected_preparer(context.preparer.clone());
                super::uncompacted::carry_on(
                    context.session,
                    context.output,
                    failure,
                    occupied,
                    context.usable_window,
                )?;
                context.compaction_trigger = context.usable_window;
                if let Ok(mut tokens) = context.token_state.lock() {
                    tokens.compaction_needed = false;
                }
            }
            Err(failure) => return Err(failure.into()),
        }
    } else {
        super::super::recovery_compaction::execute_compaction_and_capture_events(
            context.session,
            context.compaction_in_progress.clone(),
            context.prompt,
            context.threshold,
            context.context_window,
            &context.token_state,
            context.output,
        )
        .await?;
        if let Ok(mut tokens) = context.token_state.lock() {
            tokens.compaction_needed = false;
            tokens.input_tokens = context.session.token_tracker.input_tokens;
            tokens.cache_read_input_tokens = 0;
            tokens.cache_creation_input_tokens = 0;
            tokens.output_tokens = 0;
        }
    }
    let new_hook =
        CompactionHook::new(Arc::clone(&context.token_state), context.compaction_trigger);
    debug!(policy = ?policy, retry_prompt, compaction_retry_count = state.compaction_retry_count,
        max_compaction_retries = MAX_COMPACTION_RETRIES, use_compactor_sub_agent,
        "[stream_loop] CMPCT-028: issuing in-loop post-compaction retry stream with policy-selected prompt");
    debug!(
        "API REQUEST (compaction retry {}/{}) - Provider: {}, Model: {}",
        state.compaction_retry_count,
        MAX_COMPACTION_RETRIES,
        context.session.current_provider_name(),
        context
            .session
            .current_model_id()
            .as_deref()
            .unwrap_or("NONE")
    );
    *stream = agent
        .prompt_streaming_with_history_and_hook(
            &retry_prompt,
            context.session.inference_messages(),
            new_hook,
        )
        .await
        .boxed();
    state.tool_calls_buffer.clear();
    state.last_tool_name = None;
    state.final_stop_reason = None;
    state.accumulated_reasoning.clear();
    state.turn_tool_infos.clear();
    state.tool_execution_in_progress = false;
    state.streaming_display = StreamingTokenDisplay::from_cache_inclusive_total(
        context.session.token_tracker.input_tokens,
        0,
        0,
        0,
    )
    .with_prev_reasoning(state.prev_reasoning);
    Ok(())
}
