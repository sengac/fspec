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
use codelet_tui::{InputQueue, StatusDisplay, TuiEvent};
use futures::StreamExt;
use rig::{
    agent::MultiTurnStreamItem,
    completion::{CompletionModel, GetTokenUsage},
    message::Message,
    streaming::StreamedUserContent,
    wasm_compat::WasmCompatSend,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Acquire},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{sync::Notify, time::interval};

#[allow(clippy::too_many_arguments)]
pub(super) async fn run<Model, Output, Events>(
    agent: RigAgent<Model>,
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
    session: &mut Session,
    mut event_stream: Option<&mut Events>,
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
    Events: futures::Stream<Item = TuiEvent> + Unpin + Send + ?Sized,
{
    codelet_tools::tool_attribution::configure_tool_server(
        agent.tool_server_handle(),
        session_id.to_string(),
    )
    .await?;
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
    let hook = CompactionHook::new(Arc::clone(&token_state), preparation.threshold);
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
    let request_history_len = session.messages.len();
    let mut stream: AgentStream<'_, Model> = agent
        .prompt_streaming_with_history_and_hook(effective_prompt, &mut session.messages, hook)
        .await
        .boxed();
    session.messages.push(Message::User {
        content: build_user_content_with_images(effective_prompt, images),
    });
    super::telemetry::response_start(session, &request_id);
    let status = if event_stream.is_some() {
        Some(StatusDisplay::new())
    } else {
        None
    };
    let mut status_interval = if event_stream.is_some() {
        Some(interval(Duration::from_secs(1)))
    } else {
        None
    };
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
        cli_mode: event_stream.is_some(),
        request_id,
        api_start_time,
        preparer: preparation.preparer,
        threshold: preparation.threshold,
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
        let chunk = super::poll::next::<Model, _, Events>(
            &mut stream,
            &mut context,
            &state,
            &mut event_stream,
            &mut status_interval,
            &status,
        )
        .await?;
        if let Some(chunk) = chunk {
            let control = match chunk {
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(item))) => {
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
