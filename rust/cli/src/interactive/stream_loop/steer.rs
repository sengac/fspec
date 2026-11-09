//! RIG-016: deliver steered input at the running turn's next safe boundary.
//!
//! A steer waits in the session's inbox until every tool call of the current
//! step has its result; the stream is then restarted with the steer as the
//! next user message over the history that already holds those results, so
//! tool-call history stays valid and no in-flight generation is altered.
//!
//! BUG-546: rig runs each tool call as soon as it streams, so a tool result
//! does not mean the step is over: the same response may still stream more
//! calls. The driver therefore calls [`deliver`] only at the next
//! ResponseAttemptStarted after tool results, which rig yields once the
//! response has finished and before it sends the next request. Delivering
//! after the first result dropped the remaining calls (and Codex refused the
//! replay of calls without an outcome).

use super::{
    super::output::StreamOutput,
    state::{AgentStream, Context, State},
};
use codelet_core::{CompactionHook, RigAgent};
use futures::StreamExt;
use rig::{
    completion::{CompletionModel, GetTokenUsage},
    message::Message,
    wasm_compat::WasmCompatSend,
};
use std::sync::Arc;

/// Whether every tool call streamed in this step has settled.
fn at_safe_boundary(state: &State) -> bool {
    !state.tool_execution_in_progress
        && state
            .turn_tool_infos
            .iter()
            .all(|info| info.output.is_some())
}

/// Restart the stream with pending steers when the step is at a safe
/// boundary. Returns whether a steer was delivered.
pub(super) async fn deliver<'agent, Model, Output>(
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
    if !at_safe_boundary(state) || !codelet_common::input_inbox::has_steers(context.session_id) {
        return Ok(false);
    }
    let steers = codelet_common::input_inbox::take_steers(context.session_id);
    let text = steers
        .into_iter()
        .map(|entry| entry.text)
        .collect::<Vec<_>>()
        .join("\n\n");
    tracing::info!(session = %context.session_id, "[RIG-016] delivering steered input");
    let hook = CompactionHook::new(Arc::clone(&context.token_state), context.threshold);
    *stream = agent
        .prompt_streaming_with_history_and_hook(&text, context.session.inference_messages(), hook)
        .await
        .boxed();
    if let Err(error) = crate::session::admission::persist_continuation(context.session_id, &text) {
        tracing::warn!(session = %context.session_id, "[RIG-016] steer not persisted: {error}");
    }
    context.session.messages.push(Message::user(text));
    state.tool_calls_buffer.clear();
    state.last_tool_name = None;
    state.final_stop_reason = None;
    state.steer_boundary_pending = false;
    Ok(true)
}
