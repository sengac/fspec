use super::{
    super::{
        output::StreamOutput,
        recovery_network::{network_retry_delay, MAX_NETWORK_RETRIES},
        stream_handlers::handle_final_response,
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
use tracing::{debug, info, warn};

pub(super) async fn recover<'agent, Model, Output>(
    error_str: &str,
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
    state.network_retry_count += 1;
    if state.network_retry_count <= MAX_NETWORK_RETRIES {
        let delay = network_retry_delay(state.network_retry_count);
        info!(
            "NET-001: Transient network error detected (attempt {}/{}), retrying in {:.1}s: {}",
            state.network_retry_count,
            MAX_NETWORK_RETRIES,
            delay.as_secs_f64(),
            error_str
        );
        if state.network_retry_count == 1 {
            context.output.emit_status("⟳ Reconnecting...");
        }
        state.network_retry_in_progress = true;
        tokio::time::sleep(delay).await;
        if context.is_interrupted.load(Acquire) {
            return Ok(Control::Stop);
        }
        if !state.assistant_text.is_empty() {
            handle_final_response(&state.assistant_text, &mut context.session.messages)?;
            state.assistant_text.clear();
        }
        let retry_hook = super::super::retry_compaction_hook(
            &context.token_state,
            context.session.token_tracker.input_tokens,
            context.session.token_tracker.output_tokens,
            context.threshold,
        );
        debug!(
            "API REQUEST (network retry {}/{}) - Provider: {}, Model: {}",
            state.network_retry_count,
            MAX_NETWORK_RETRIES,
            context.session.current_provider_name(),
            context
                .session
                .current_model_id()
                .as_deref()
                .unwrap_or("NONE")
        );
        *stream = agent
            .prompt_streaming_with_history_and_hook(
                "Continue",
                &mut context.session.messages,
                retry_hook,
            )
            .await
            .boxed();
        context.session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text("Continue")),
        });
        state.tool_calls_buffer.clear();
        state.last_tool_name = None;
        state.final_stop_reason = None;
        state.accumulated_reasoning.clear();
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
        debug!("[stream_loop] NET-001: Created new stream for network retry");
        return Ok(Control::Continue);
    }
    warn!(
        "NET-001: Network retry budget exhausted after {} attempts",
        MAX_NETWORK_RETRIES
    );
    context.output.emit_status("✗ Reconnection failed");
    Ok(Control::Next)
}
