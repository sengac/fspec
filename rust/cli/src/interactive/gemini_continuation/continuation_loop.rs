use super::super::done_early_exit::{
    apply_finish_with_summary, decide_tool_result_early_exit, DONE_EARLY_EXIT_STOP_REASON,
};
use super::super::error_classifiers::{
    classify_compaction_branch, extract_prompt_cancelled, CompactionBranch,
};
use super::super::interruption::{record_tool_cancellations, CancellationReason};
use super::super::output::StreamOutput;
use super::super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};
use super::state::ContinuationState;
use super::{
    append_continuation_prompt, capture_continuation_event, BoxedStream, GeminiContinuationResult,
};
use crate::interactive_helpers::reconcile_session_messages;
use crate::session::Session;
use anyhow::Result;
use codelet_core::{
    CompactionHook, ContinuationStrategy, GeminiTurnCompletionFacade, RigAgent, TokenState,
    TurnCompletionFacade,
};
use codelet_tools::set_tool_progress_callback;
use codelet_tui::InputQueue;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use rig::wasm_compat::WasmCompatSend;
use std::sync::atomic::{AtomicBool, Ordering::Acquire};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_continuation_loop<'stream, M, O>(
    agent: &'stream RigAgent<M>,
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    parent_token_state: &Arc<Mutex<TokenState>>,
    threshold: u64,
    model_id: &str,
    stream: &mut BoxedStream<'stream, M::StreamingResponse>,
    mut state: ContinuationState,
    session_id: Uuid,
) -> Result<GeminiContinuationResult>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    let mut input_observation = codelet_core::compaction::InputUsageObservation::default();
    loop {
        if is_interrupted.load(Acquire) {
            output.acknowledge_interruption();
            let queued = input_queue
                .as_mut()
                .map(|queue| queue.dequeue_all())
                .unwrap_or_default();
            record_tool_cancellations(
                session,
                &mut state.tool_calls_buffer,
                output,
                CancellationReason::UserInterruption,
            )
            .await?;
            state.finish_history(session, session_id)?;
            set_tool_progress_callback(session_id, None);
            super::super::interruption::publish_settlement(
                output,
                vec![
                    super::super::output::StreamEvent::Interrupted(queued),
                    super::super::output::StreamEvent::Done(None),
                ],
                false,
            )
            .await?;
            return Ok(GeminiContinuationResult::Finished {
                display: state.display,
            });
        }

        match stream.next().await {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ResponseAttemptStarted
                | StreamedAssistantContent::ResponseAttemptAborted,
            ))) => {
                input_observation = Default::default();
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(
                chunk,
            )))) => {
                handle_text_chunk(&chunk.text, &mut state.text, None, output)?;
                if let Some(update) = state.display.record_chunk(&chunk.text) {
                    output.emit_tokens(&update.into());
                }
                record_tool_cancellations(
                    session,
                    &mut state.tool_calls_buffer,
                    output,
                    CancellationReason::Unspecified,
                )
                .await?;
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall(tool_call),
            ))) => {
                handle_tool_call(
                    &tool_call,
                    &mut session.messages,
                    &mut state.text,
                    &mut state.tool_calls_buffer,
                    &mut state.last_tool_name,
                    output,
                )?;
                state.tool_calls += 1;
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ReasoningDelta { reasoning, .. },
            ))) => {
                output.emit_thinking(&reasoning);
                if let Some(update) = state.display.record_chunk(&reasoning) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(
                tool_result,
            )))) => {
                handle_tool_result(
                    &tool_result,
                    &mut session.messages,
                    &mut state.tool_calls_buffer,
                    &state.last_tool_name,
                    output,
                )?;
                if let Some(summary) = decide_tool_result_early_exit(|| {
                    codelet_tools::take_done_acceptance(session_id)
                }) {
                    record_tool_cancellations(
                        session,
                        &mut state.tool_calls_buffer,
                        output,
                        CancellationReason::AcceptedCompletion,
                    )
                    .await?;
                    state.finish_history(session, session_id)?;
                    apply_finish_with_summary(session, session_id, &summary, output);
                    set_tool_progress_callback(session_id, None);
                    output.emit_done_with_stop_reason(Some(DONE_EARLY_EXIT_STOP_REASON.into()));
                    return Ok(GeminiContinuationResult::Finished {
                        display: state.display,
                    });
                }
            }
            Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                input_observation.observe(&mut session.token_tracker, &usage);
                if usage.output_tokens == 0 {
                    state.display.start_new_segment(&usage);
                } else if let Some(update) = state.display.update_from_usage(&usage) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                let physical = final_resp
                    .request_context()
                    .map_or(final_resp.usage(), |context| {
                        context.usage.unwrap_or_default()
                    });
                input_observation.finish(&mut session.token_tracker, &physical);
                state.finalize(&final_resp);
                output.emit_tokens(&state.display.current().into());
                if state.needs_recovery(parent_token_state) {
                    let policy = state.recover(session, parent_token_state, output)?;
                    return Ok(GeminiContinuationResult::CompactionNeeded(policy));
                }
                let strategy = GeminiTurnCompletionFacade
                    .continuation_strategy(&state.text, &session.messages);
                if let ContinuationStrategy::FullLoop { prompt } = strategy {
                    capture_continuation_event(
                        model_id,
                        prompt,
                        "nested_empty_response_after_tool",
                    );
                    handle_final_response(&state.text, &mut session.messages)?;
                    state.text.clear();
                    append_continuation_prompt(session, prompt);
                    state.start_nested();
                    let hook = CompactionHook::new(Arc::clone(&state.token_state), threshold);
                    let raw_nested = agent
                        .prompt_streaming_with_history_and_hook(prompt, &mut session.messages, hook)
                        .await;
                    *stream = Box::pin(raw_nested);
                    continue;
                }
                return Ok(GeminiContinuationResult::Completed {
                    text: state.text,
                    display: state.display,
                    usage: final_resp.usage(),
                    tool_calls: state.tool_calls,
                    stop_reason: final_resp.stop_reason().map(String::from),
                });
            }
            Some(Err(error)) => {
                state.needs_recovery(parent_token_state);
                let branch = classify_compaction_branch(&error, parent_token_state);
                if matches!(branch, CompactionBranch::Recover { .. }) {
                    if let Some(history) = extract_prompt_cancelled(&error) {
                        reconcile_session_messages(&mut session.messages, history);
                    }
                    let policy = state.recover(session, parent_token_state, output)?;
                    return Ok(GeminiContinuationResult::CompactionNeeded(policy));
                }
                set_tool_progress_callback(session_id, None);
                output.emit_error(&error.to_string());
                return Err(anyhow::anyhow!("Gemini continuation error: {error}"));
            }
            None => {
                if state.needs_recovery(parent_token_state) {
                    let policy = state.recover(session, parent_token_state, output)?;
                    return Ok(GeminiContinuationResult::CompactionNeeded(policy));
                }
                state.finish_history(session, session_id)?;
                set_tool_progress_callback(session_id, None);
                output.emit_done_with_stop_reason(None);
                return Ok(GeminiContinuationResult::Finished {
                    display: state.display,
                });
            }
            _ => {}
        }
        output.flush();
    }
}
