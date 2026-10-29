use super::{
    super::output::StreamOutput,
    state::{AgentStream, Context, State},
};
use codelet_tui::{StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use futures::StreamExt;
use rig::{agent::MultiTurnStreamItem, completion::CompletionModel};
use std::{sync::atomic::Ordering::Release, time::Duration};
use tokio::time::Interval;

pub(super) async fn next<Model, Output, Events>(
    stream: &mut AgentStream<'_, Model>,
    context: &mut Context<'_, Output>,
    state: &State,
    event_stream: &mut Option<&mut Events>,
    status_interval: &mut Option<Interval>,
    status: &Option<StatusDisplay>,
) -> anyhow::Result<
    Option<Option<Result<MultiTurnStreamItem<Model::StreamingResponse>, anyhow::Error>>>,
>
where
    Model: CompletionModel,
    Output: StreamOutput,
    Events: futures::Stream<Item = TuiEvent> + Unpin + Send + ?Sized,
{
    let effective_stall_timeout = if state.tool_execution_in_progress {
        Duration::from_secs(86400)
    } else {
        super::super::recovery_stall::stall_timeout_duration()
    };
    let chunk = match (event_stream, status_interval, status) {
        (Some(events), Some(status_interval), Some(status)) => {
            tokio::select! {
                chunk = stream.next() => Some(chunk),
                event = events.next() => {
                    if let Some(TuiEvent::Key(key)) = event {
                        if key.code == KeyCode::Esc {
                            context.is_interrupted.store(true, Release);
                        }
                    }
                    None
                }
                _ = status_interval.tick() => {
                    let _ = status.format_status();
                    None
                }
                _ = super::super::provider_recovery::wait_for_stall_deadline(
                    &context.session.provider_session, context.session.current_provider_name(), effective_stall_timeout,
                ) => return Err(stalled(context, state, effective_stall_timeout)?),
            }
        }
        _ => match &context.interrupt_notify {
            Some(notify) => {
                let interrupt_fut = notify.notified();
                tokio::select! {
                    chunk = stream.next() => Some(chunk),
                    _ = interrupt_fut => None,
                    _ = super::super::provider_recovery::wait_for_stall_deadline(
                        &context.session.provider_session, context.session.current_provider_name(), effective_stall_timeout,
                    ) => return Err(stalled(context, state, effective_stall_timeout)?),
                }
            }
            None => {
                let next = tokio::select! {
                    chunk = stream.next() => Ok(chunk),
                    _ = super::super::provider_recovery::wait_for_stall_deadline(
                        &context.session.provider_session, context.session.current_provider_name(), effective_stall_timeout,
                    ) => Err(()),
                };
                match next {
                    Ok(chunk) => Some(chunk),
                    Err(()) => return Err(stalled(context, state, effective_stall_timeout)?),
                }
            }
        },
    };
    Ok(chunk)
}

fn stalled<Output: StreamOutput>(
    context: &mut Context<'_, Output>,
    state: &State,
    timeout: Duration,
) -> anyhow::Result<anyhow::Error> {
    let message = super::super::recovery_stall::build_stall_timeout_message(timeout.as_secs());
    tracing::warn!("AMGR-016: {}", message);
    context.output.emit_error(&message);
    if !state.assistant_text.is_empty() {
        super::super::stream_handlers::handle_final_response(
            &state.assistant_text,
            &mut context.session.messages,
        )?;
    }
    context
        .output
        .emit_done_with_stop_reason(Some("stall_timeout".to_owned()));
    Ok(anyhow::anyhow!("{message}"))
}
