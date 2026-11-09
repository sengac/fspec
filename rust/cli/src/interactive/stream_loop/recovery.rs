use super::{
    super::{
        error_classifiers::{
            is_context_overflow_error, is_image_content_error, is_stall_timeout_error,
            is_truncated_tool_call_error,
        },
        output::StreamOutput,
    },
    state::{AgentStream, Context, Control, State},
};
use codelet_core::RigAgent;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    message::Message,
    wasm_compat::WasmCompatSend,
};
use tracing::{debug, info, warn};

pub(super) async fn handle<'agent, Model, Output>(
    error: anyhow::Error,
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
    state.input_observation = Default::default();
    if crate::compactor_sub_agent::has_terminal_overflow_owner(context.session_id)
        && is_context_overflow_error(&error)
    {
        if context.session.messages.len() == context.request_history_len + 1
            && state.assistant_text.is_empty()
            && state.accumulated_reasoning.is_empty()
            && state.turn_tool_infos.is_empty()
        {
            context
                .session
                .messages
                .truncate(context.request_history_len);
        }
        context.session.block_compaction_inference(
            "terminal overflow requires outer recovery reconciliation".into(),
        );
        return Err(error.context("Agent error"));
    }
    if super::super::request_failure::block_terminal_request_failure(context.session, &error) {
        context.output.emit_error(&error.to_string());
        return Err(error.context("Agent error"));
    }
    debug!(
        "[stream_loop] STREAM ERROR RECEIVED: error={}, type={:?}",
        error,
        std::any::type_name_of_val(&error)
    );
    if super::recovery_compaction::recover(&error, agent, stream, context, state).await? {
        return Ok(Control::Continue);
    }
    let error_str = error.to_string();
    if is_stall_timeout_error(&error_str) {
        return Err(error.context("Agent error"));
    }
    if is_image_content_error(&error_str) {
        warn!("Received image content error from API, attempting to sanitize conversation history");
        if let Some(last_msg) = context.session.messages.last() {
            if matches!(last_msg, Message::User { .. }) {
                context.session.messages.pop();
                info!("Popped last user message before image sanitization");
            }
        }
        let sanitized =
            super::super::recovery_image::sanitize_image_content(&mut context.session.messages);
        if sanitized {
            info!("Sanitized image content from conversation history — session can continue");
            context.output.emit_error(&format!(
                "{error_str}\n\n[Images removed from conversation history to recover session]"
            ));
            return Ok(Control::Stop);
        }
    }
    if is_truncated_tool_call_error(&error_str) {
        super::recovery_truncation::recover(&error_str, agent, stream, context, state).await?;
        return Ok(Control::Continue);
    }
    if super::super::provider_recovery::should_retry_network_error(
        context.session.current_provider_name(),
        error.as_ref(),
    ) {
        match super::recovery_network::recover(&error_str, agent, stream, context, state).await? {
            Control::Next => {}
            control => return Ok(control),
        }
    }
    super::terminal_error::handle(error, context)
}
