use super::{
    super::{
        context_projection::ContextProjection,
        multimodal::{build_user_content_with_images, BridgeImage},
        output::StreamOutput,
    },
    state::{AgentStream, Context, Control, State},
};
use crate::{compaction_cancellation::CompactionCancellation, session::Session};
use codelet_core::{CompactionHook, RigAgent, TokenState};
use codelet_tui::InputQueue;
use futures::StreamExt;
use rig::{
    agent::MultiTurnStreamItem,
    completion::{CompletionModel, GetTokenUsage},
    message::Message,
    streaming::{StreamedAssistantContent, StreamedUserContent},
    wasm_compat::WasmCompatSend,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Acquire},
        Arc, Mutex,
    },
    time::Instant,
};
use tokio::sync::Notify;

#[allow(clippy::too_many_arguments)]
pub(super) async fn run<Model, Output>(
    agent: RigAgent<Model>,
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
    session: &mut Session,
    input_queue: Option<&mut InputQueue>,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    interrupt_notify: Option<Arc<Notify>>,
    output: &Output,
    session_id: uuid::Uuid,
) -> anyhow::Result<()>
where
    Model: CompletionModel,
    Model::StreamingResponse: WasmCompatSend + GetTokenUsage,
    Output: StreamOutput,
{
    codelet_tools::tool_attribution::configure_tool_server(
        agent.tool_server_handle(),
        session_id.to_string(),
    )
    .await?;
    // BUG-516: a manual /compact runs the compactor sub-agent round, never
    // an ordinary inference turn, so the BUG-214 gate below is not relaxed.
    if session.take_manual_compaction_request() {
        let cancellation =
            CompactionCancellation::new(is_interrupted.clone(), interrupt_notify.clone());
        return super::manual_compaction::run(
            &agent,
            session,
            output,
            session_id,
            &compaction_in_progress,
            &cancellation,
        )
        .await;
    }
    session.ensure_compaction_inference_ready()?;
    let compaction_cancellation =
        CompactionCancellation::new(is_interrupted.clone(), interrupt_notify.clone());
    let agent = session
        .configure_agent(agent)
        .with_settlement_session(session_id);
    let progressive_output = super::super::progressive_output::ProgressiveOutput::new(
        output,
        session.provider_manager().runtime_options().streaming != Some(false),
    );
    let output = &progressive_output;
    let request_id = uuid::Uuid::new_v4().to_string();
    let api_start_time = Instant::now();
    super::super::continue_state::emit_continue_state(
        session,
        output,
        super::super::output::ContinueStateReason::TurnStart,
    );
    let preparation = super::preflight::prepare(
        &agent,
        prompt,
        session,
        output,
        session_id,
        &compaction_in_progress,
        &compaction_cancellation,
    )
    .await?;
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: session.token_tracker.output_tokens,
        compaction_needed: false,
    }));
    let hook = CompactionHook::new(Arc::clone(&token_state), preparation.compaction_trigger);
    let active_tool_call_id = super::telemetry::start(
        session,
        prompt,
        output,
        &request_id,
        session_id,
        &preparation,
        &token_state,
    );
    let effective_prompt = if preparation.compaction_just_ran {
        "Continue"
    } else {
        prompt
    };
    if preparation.compaction_just_ran && session.admitted_input_pending().is_some() {
        crate::session::admission::persist_continuation(session_id, effective_prompt)
            .map_err(anyhow::Error::msg)?;
    }
    let admitted = session.detach_admitted_input(preparation.compaction_just_ran);
    let request_history_len = session.messages.len();
    // MODEL-015: the turn's effort message goes right before the new user
    // message; the turn keeps the conversation's top-level effort.
    let agent = crate::session::effort::configure_turn_effort(
        &mut session.effort_log,
        agent,
        &session.messages,
    );
    let mut stream: AgentStream<'_, Model> = agent
        .prompt_streaming_with_history_and_hook(effective_prompt, &mut session.messages, hook)
        .await
        .boxed();
    session
        .messages
        .push(admitted.unwrap_or_else(|| Message::User {
            content: build_user_content_with_images(effective_prompt, images),
        }));
    super::telemetry::response_start(session, &request_id);
    let mut state = State::new(session, output);
    let context_projection = ContextProjection {
        session_id,
        model: session.current_model_id().unwrap_or_default(),
        capacity: preparation.context_window,
        threshold: preparation.threshold,
    };
    context_projection.emit(output, None, None);
    let mut context = Context {
        session,
        prompt,
        output,
        session_id,
        is_interrupted,
        compaction_in_progress,
        interrupt_notify,
        compaction_cancellation,
        input_queue,
        request_id,
        api_start_time,
        preparer: preparation.preparer,
        threshold: preparation.threshold,
        compaction_trigger: preparation.compaction_trigger,
        usable_window: preparation.usable_window,
        context_window: preparation.context_window,
        token_state,
        active_tool_call_id,
        request_history_len,
        context_projection,
    };
    loop {
        if context.is_interrupted.load(Acquire) {
            drop(stream);
            super::interrupted::settle(&mut context, &mut state).await?;
            break;
        }
        let chunk = super::poll::next::<Model, _>(&mut stream, &mut context, &state).await?;
        if let Some(chunk) = chunk {
            let control = match chunk {
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(item))) => {
                    // BUG-546: the first attempt start after tool results is
                    // the step boundary: deliver pending steers there, before
                    // the next request is sent. The replaced stream's attempt
                    // start is not reported; the new stream reports its own.
                    if matches!(item, StreamedAssistantContent::ResponseAttemptStarted)
                        && std::mem::take(&mut state.steer_boundary_pending)
                        && super::steer::deliver(&agent, &mut stream, &mut context, &mut state)
                            .await?
                    {
                        continue;
                    }
                    super::assistant::handle(item, &mut context, &mut state)?;
                    Control::Next
                }
                Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(
                    result,
                )))) => {
                    match super::tool_result::handle(result, &mut context, &mut state)? {
                        Control::Next => {}
                        Control::Stop => break,
                        Control::Continue => continue,
                    }
                    if let Some(summary) =
                        super::super::done_early_exit::decide_tool_result_early_exit(|| {
                            codelet_tools::take_done_acceptance(context.session_id)
                        })
                    {
                        drop(stream);
                        super::tool_result::accepted(&mut context, &mut state, &summary).await?;
                        break;
                    }
                    // BUG-546: not here. The same response may still stream
                    // tool calls; a steer waits for the step boundary.
                    state.steer_boundary_pending = true;
                    Control::Next
                }
                Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                    super::usage::observed(usage, &mut context, &mut state);
                    Control::Next
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(response))) => {
                    super::final_response::handle(
                        response,
                        &agent,
                        &mut stream,
                        &mut context,
                        &mut state,
                    )
                    .await?
                }
                Some(Err(error)) => {
                    super::recovery::handle(error, &agent, &mut stream, &mut context, &mut state)
                        .await?
                }
                None => {
                    super::stream_end::handle(&mut context, &mut state)?;
                    Control::Stop
                }
                _ => {
                    tracing::debug!("[stream_loop] Unknown stream item received (ignored)");
                    Control::Next
                }
            };
            match control {
                Control::Next => {}
                Control::Continue => continue,
                Control::Stop => break,
            }
            output.flush();
        }
    }
    super::finish::finish(&mut context, &state).await
}
