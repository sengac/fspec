//! Post-loop compaction retry handling.
//!
//! When the compaction hook triggers during streaming, this module handles
//! executing the compaction and retrying the stream with compacted context.

use super::output::StreamOutput;
use super::stream_handlers::{handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result};
use super::stream_loop::emit_context_fill_from_usage;
use super::error_classifiers::is_transient_network_error;
use super::recovery_network::{MAX_NETWORK_RETRIES, network_retry_delay};

use crate::interactive_helpers::{compression_ratio, execute_compaction};
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_core::{
    ApiTokenUsage, CompactionHook, RigAgent, StreamingTokenDisplay, TokenState,
};
use codelet_tools::set_tool_progress_callback;
use codelet_tui::InputQueue;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use rig::wasm_compat::WasmCompatSend;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Acquire;
use std::sync::{Arc, Mutex};
use tracing::{debug, trace};

/// Handle post-loop compaction and retry stream.
///
/// Called when `compaction_needed == true` after the main stream loop exits.
/// Executes compaction, then starts a new stream with the compacted context.
#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_compaction_retry<M, O>(
    agent: &RigAgent<M>,
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    token_state: &Arc<Mutex<TokenState>>,
    threshold: u64,
    context_window: u64,
    compaction_in_progress: Arc<AtomicBool>,
    prompt: &str,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    debug!(
        "[stream_loop] ENTERING compaction block - messages_len={}, approx_turns={}",
        session.messages.len(),
        session.messages.len() / 2
    );

    output.emit_compaction_started();
    let total_turns = session.messages.len() as u32 / 2;
    output.emit_compaction_progress("Analyzing context", 0, total_turns.max(1));

    // Capture compaction.triggered event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                if let Ok(state) = token_state.lock() {
                    manager.capture(
                        "compaction.triggered",
                        serde_json::json!({
                            "timing": "hook-triggered",
                            "inputTokens": state.input_tokens,
                            "cacheReadInputTokens": state.cache_read_input_tokens,
                            "threshold": threshold,
                            "contextWindow": context_window,
                        }),
                        None,
                    );
                }
            }
        }
    }

    let original_tokens = session.token_tracker.input_tokens;

    match execute_compaction(session, compaction_in_progress, Some(prompt)).await {
        Ok(()) => {
            let compacted_tokens = session.token_tracker.input_tokens;
            let ratio = compression_ratio(original_tokens, compacted_tokens);

            // Capture context.update event after compaction
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "context.update",
                            serde_json::json!({
                                "type": "compaction",
                                "originalTokens": original_tokens,
                                "compactedTokens": compacted_tokens,
                                "compressionRatio": ratio,
                            }),
                            None,
                        );
                    }
                }
            }

            output.emit_compaction_continuing();
            session.token_tracker.reset_after_compaction();

            // Create fresh hook and token state for the retry
            let retry_token_state = Arc::new(Mutex::new(TokenState {
                input_tokens: session.token_tracker.input_tokens,
                cache_read_input_tokens: 0,
                cache_creation_input_tokens: 0,
                output_tokens: 0,
                compaction_needed: false,
            }));
            let retry_hook = CompactionHook::new(Arc::clone(&retry_token_state), threshold);

            debug!(
                "API REQUEST (retry after compaction) - Provider: {}, Model: {}",
                session.current_provider_name(),
                session.current_model_id().as_deref().unwrap_or("NONE")
            );
            let mut retry_stream = agent
                .prompt_streaming_with_history_and_hook(
                    "Continue",
                    &mut session.messages,
                    retry_hook,
                )
                .await;

            run_retry_stream::<M, O>(
                session,
                output,
                is_interrupted,
                input_queue,
                threshold,
                context_window,
                &mut retry_stream,
            ).await?;

            Ok(())
        }
        Err(e) => {
            output.emit_compaction_failed(&format!("{e} - will retry on next turn"));

            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "compaction.failed",
                            serde_json::json!({
                                "error": e.to_string(),
                                "inputTokens": session.token_tracker.input_tokens,
                            }),
                            None,
                        );
                    }
                }
            }

            Err(anyhow::anyhow!("Compaction failed: {e}"))
        }
    }
}

/// Process the retry stream after successful compaction.
#[allow(clippy::too_many_arguments)]
async fn run_retry_stream<M, O>(
    session: &mut Session,
    output: &O,
    is_interrupted: &Arc<AtomicBool>,
    input_queue: &mut Option<&mut InputQueue>,
    threshold: u64,
    context_window: u64,
    retry_stream: &mut (impl futures::Stream<Item = Result<MultiTurnStreamItem<M::StreamingResponse>, anyhow::Error>> + Unpin + Send),
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    let mut retry_text = String::new();
    let mut retry_tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
    let mut retry_last_tool_name: Option<String> = None;
    // NET-001: Track network retry count for compaction retry stream
    let mut network_retry_count: u32 = 0;
    // NET-001: Track whether we're recovering from a network retry (for UX feedback)
    let mut network_retry_in_progress = false;
    let mut retry_display = StreamingTokenDisplay::new(
        session.token_tracker.input_tokens,
        0,
        0,
        0,
    );

    loop {
        if is_interrupted.load(Acquire) {
            let queued = if let Some(ref mut iq) = input_queue {
                iq.dequeue_all()
            } else {
                vec![]
            };
            output.emit_interrupted(&queued);
            if !retry_text.is_empty() {
                handle_final_response(&retry_text, &mut session.messages)?;
            }
            output.emit_done_with_stop_reason(None);
            break;
        }

        match retry_stream.next().await {
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(text),
            ))) => {
                if network_retry_in_progress {
                    output.emit_status("✓ Reconnected");
                    network_retry_in_progress = false;
                }
                network_retry_count = 0;
                handle_text_chunk(&text.text, &mut retry_text, None, output)?;
                if let Some(update) = retry_display.record_chunk(&text.text) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ToolCall(tool_call),
            ))) => {
                if network_retry_in_progress {
                    output.emit_status("✓ Reconnected");
                    network_retry_in_progress = false;
                }
                network_retry_count = 0;
                handle_tool_call(
                    &tool_call,
                    &mut session.messages,
                    &mut retry_text,
                    &mut retry_tool_calls_buffer,
                    &mut retry_last_tool_name,
                    output,
                )?;
            }
            Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::ReasoningDelta { reasoning, .. },
            ))) => {
                // NET-001: Reset network retry counter on successful data receipt
                if network_retry_in_progress {
                    output.emit_status("✓ Reconnected");
                    network_retry_in_progress = false;
                }
                network_retry_count = 0;

                output.emit_thinking(&reasoning);
                if let Some(update) = retry_display.record_chunk(&reasoning) {
                    output.emit_tokens(&update.into());
                }
            }
            Some(Ok(MultiTurnStreamItem::StreamUserItem(
                StreamedUserContent::ToolResult(tool_result),
            ))) => {
                handle_tool_result(
                    &tool_result,
                    &mut session.messages,
                    &mut retry_tool_calls_buffer,
                    &retry_last_tool_name,
                    output,
                )?;
            }
            Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                if usage.output_tokens == 0 {
                    retry_display.start_new_segment(&usage);
                } else if let Some(update) = retry_display.update_from_usage(&usage) {
                    output.emit_tokens(&update.into());
                    let fill_usage = ApiTokenUsage::new(
                        update.input_tokens,
                        update.cache_read_tokens,
                        update.cache_creation_tokens,
                        usage.output_tokens,
                    ).with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
                    emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);
                }
            }
            Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                let retry_stop_reason = final_resp.stop_reason().map(String::from);
                let usage = final_resp.usage();

                let retry_final = if !retry_display.has_authoritative_output() && usage.input_tokens > 0 {
                    trace!(
                        "OpenAI-compatible provider (retry): extracted tokens from FinalResponse - input={}, output={}, cache_read={:?}",
                        usage.input_tokens, usage.output_tokens, usage.cache_read_input_tokens
                    );
                    retry_display.update_from_final_response(&usage)
                } else {
                    retry_display.current()
                };

                output.emit_tokens(&retry_final.into());
                let fill_usage = ApiTokenUsage::new(
                    retry_final.input_tokens,
                    retry_final.cache_read_tokens,
                    retry_final.cache_creation_tokens,
                    usage.output_tokens,
                ).with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
                emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);

                if !is_interrupted.load(Acquire) {
                    let retry_usage = ApiTokenUsage::new(
                        retry_final.input_tokens,
                        retry_final.cache_read_tokens,
                        retry_final.cache_creation_tokens,
                        0,
                    );
                    session.token_tracker.update_from_usage(&retry_usage, retry_final.output_tokens);
                }

                handle_final_response(&retry_text, &mut session.messages)?;
                output.emit_done_with_stop_reason(retry_stop_reason);
                break;
            }
            Some(Err(e)) => {
                let error_str = e.to_string();
                // NET-001: Retry transient network errors in compaction retry stream
                if is_transient_network_error(&error_str) {
                    network_retry_count += 1;
                    if network_retry_count <= MAX_NETWORK_RETRIES {
                        let delay = network_retry_delay(network_retry_count);
                        tracing::info!(
                            "NET-001: Network error in compaction retry (attempt {}/{}), retrying in {:.1}s: {}",
                            network_retry_count,
                            MAX_NETWORK_RETRIES,
                            delay.as_secs_f64(),
                            error_str
                        );
                        if network_retry_count == 1 {
                            output.emit_status("⟳ Reconnecting...");
                        }
                        network_retry_in_progress = true;
                        tokio::time::sleep(delay).await;
                        if is_interrupted.load(Acquire) {
                            break;
                        }
                        // The compaction retry stream cannot be restarted from here
                        // (we don't have access to the agent), so we just continue
                        // polling — the stream may recover on the next poll_next().
                        // If it yields None or another error, the subsequent handler
                        // will deal with it.
                        continue;
                    }
                    tracing::warn!(
                        "NET-001: Network retry budget exhausted in compaction retry after {} attempts",
                        MAX_NETWORK_RETRIES
                    );
                    output.emit_status("✗ Reconnection failed");
                }
                output.emit_error(&error_str);
                return Err(anyhow::anyhow!("Retry error after compaction: {e}"));
            }
            None => {
                if !retry_text.is_empty() {
                    handle_final_response(&retry_text, &mut session.messages)?;
                }
                output.emit_done_with_stop_reason(None);
                break;
            }
            _ => {}
        }
        output.flush();
    }

    // Clear tool progress callback
    set_tool_progress_callback(None);

    Ok(())
}
