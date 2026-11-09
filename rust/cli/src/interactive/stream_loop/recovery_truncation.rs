use super::{
    super::{
        output::StreamOutput,
        recovery_truncation::{
            build_truncation_budget_exhausted_message, build_truncation_recovery_message,
            MAX_TRUNCATION_RETRIES,
        },
        stream_handlers::handle_final_response,
    },
    state::{AgentStream, Context, State},
};
use codelet_core::{RigAgent, StreamingTokenDisplay};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    message::{Message, UserContent},
    one_or_many::OneOrMany,
    wasm_compat::WasmCompatSend,
};
use tracing::{debug, info, warn};

pub(super) async fn recover<'agent, Model, Output>(
    error_str: &str,
    agent: &'agent RigAgent<Model>,
    stream: &mut AgentStream<'agent, Model>,
    context: &mut Context<'_, Output>,
    state: &mut State,
) -> anyhow::Result<()>
where
    Model: CompletionModel,
    Model::StreamingResponse: WasmCompatSend + GetTokenUsage,
    Output: StreamOutput,
{
    state.truncation_retry_count += 1;
    if state.truncation_retry_count <= MAX_TRUNCATION_RETRIES {
        info!(
            "PROV-040: Truncated tool call detected (attempt {}/{}), injecting recovery prompt",
            state.truncation_retry_count, MAX_TRUNCATION_RETRIES
        );
        if !state.assistant_text.is_empty() {
            handle_final_response(&state.assistant_text, &mut context.session.messages)?;
            state.assistant_text.clear();
        }
        let recovery_prompt = build_truncation_recovery_message(error_str);
        let retry_hook = super::super::retry_compaction_hook(
            &context.token_state,
            context.session.token_tracker.input_tokens,
            0,
            context.threshold,
        );
        debug!(
            "API REQUEST (truncation recovery {}/{}) - Provider: {}, Model: {}",
            state.truncation_retry_count,
            MAX_TRUNCATION_RETRIES,
            context.session.current_provider_name(),
            context
                .session
                .current_model_id()
                .as_deref()
                .unwrap_or("NONE")
        );
        *stream = agent
            .prompt_streaming_with_history_and_hook(
                &recovery_prompt,
                &mut context.session.messages,
                retry_hook,
            )
            .await
            .boxed();
        context.session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(&recovery_prompt)),
        });
        state.tool_calls_buffer.clear();
        state.last_tool_name = None;
        state.final_stop_reason = None;
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
        return Ok(());
    }
    warn!(
        "PROV-040: Truncation retry budget exhausted after {} attempts",
        MAX_TRUNCATION_RETRIES
    );
    let budget_error = build_truncation_budget_exhausted_message(MAX_TRUNCATION_RETRIES);
    context.output.emit_error(&budget_error);
    Err(anyhow::anyhow!(
        "Agent error: truncation retry budget exhausted after {MAX_TRUNCATION_RETRIES} attempts"
    ))
}
