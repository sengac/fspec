//! Agent streaming loop with interruption support
//!
//! Handles the main agent streaming loop including token tracking,
//! debug capture, and compaction triggering.
//!
//! Supports two modes:
//! - CLI mode: Uses event_stream for Esc key detection, prints to stdout
//! - NAPI mode: Uses is_interrupted flag (set by JavaScript), sends callbacks
//!
//! Uses rig's StreamingPromptHook to capture per-request token usage and
//! check compaction thresholds before each internal API call.

use super::output::{StreamOutput, TokenInfo};
use super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};
use crate::compaction_threshold::calculate_compaction_threshold;
use crate::interactive_helpers::execute_compaction;
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_core::{CompactionHook, RigAgent, TokenState};
use codelet_tui::{InputQueue, StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use rig::wasm_compat::WasmCompatSend;
use std::sync::atomic::AtomicBool;
// Use Acquire/Release ordering for proper cross-thread synchronization
// - Acquire: Ensures subsequent reads see all writes before the Release store
// - Release: Ensures all writes before the store are visible to Acquire loads
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::interval;

/// Run agent stream with CLI event handling
///
/// This is the CLI-specific entry point that wraps the generic stream function
/// with TUI event handling for Esc key detection.
pub(super) async fn run_agent_stream_with_interruption<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),
    input_queue: &mut InputQueue,
    is_interrupted: Arc<AtomicBool>,
    output: &O,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    run_agent_stream_internal(
        agent,
        prompt,
        session,
        Some(event_stream),
        Some(input_queue),
        is_interrupted,
        None, // CLI mode doesn't use Notify - uses keyboard event stream
        output,
    )
    .await
}

/// Run agent stream for NAPI (no event handling)
///
/// This is the NAPI entry point - JavaScript handles keyboard input and sets
/// is_interrupted via the interrupt() method.
///
/// NAPI-004: The interrupt_notify parameter allows immediate wake-up of the
/// stream loop when interrupt() is called, via tokio::select! with notified().
pub async fn run_agent_stream<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
    output: &O,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    run_agent_stream_internal::<M, O, dyn futures::Stream<Item = TuiEvent> + Unpin + Send>(
        agent,
        prompt,
        session,
        None,
        None,
        is_interrupted,
        Some(interrupt_notify),
        output,
    )
    .await
}

/// Internal generic stream loop
///
/// Core streaming logic shared between CLI and NAPI modes.
/// - When event_stream is Some: Uses tokio::select! with event handling (CLI)
/// - When event_stream is None but interrupt_notify is Some: Uses tokio::select! with Notify (NAPI)
/// - NAPI-004: The interrupt_notify enables immediate ESC response during tool execution
async fn run_agent_stream_internal<M, O, E>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    mut event_stream: Option<&mut E>,
    mut input_queue: Option<&mut InputQueue>,
    is_interrupted: Arc<AtomicBool>,
    interrupt_notify: Option<Arc<Notify>>,
    output: &O,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
    E: futures::Stream<Item = TuiEvent> + Unpin + Send + ?Sized,
{
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;
    use std::time::Instant;
    use uuid::Uuid;

    // CLI-022: Generate request ID for correlation
    let request_id = Uuid::new_v4().to_string();
    let api_start_time = Instant::now();

    // CRITICAL: Add user prompt to message history for persistence (CLI-008)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(prompt)),
    });

    // HOOK-BASED COMPACTION
    let context_window = session.provider_manager().context_window() as u64;
    let threshold = calculate_compaction_threshold(context_window);

    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens,
        cache_read_input_tokens: session.token_tracker.cache_read_input_tokens.unwrap_or(0),
        cache_creation_input_tokens: session
            .token_tracker
            .cache_creation_input_tokens
            .unwrap_or(0),
        compaction_needed: false,
    }));

    let hook = CompactionHook::new(Arc::clone(&token_state), threshold);

    // DEBUG: Log compaction check setup
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                if let Ok(state) = token_state.lock() {
                    manager.capture(
                        "compaction.check",
                        serde_json::json!({
                            "timing": "hook-setup",
                            "inputTokens": state.input_tokens,
                            "cacheReadInputTokens": state.cache_read_input_tokens,
                            "threshold": threshold,
                            "contextWindow": context_window,
                            "messageCount": session.messages.len(),
                        }),
                        None,
                    );
                }
            }
        }
    }

    // CLI-022: Capture api.request event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.request",
                    serde_json::json!({
                        "provider": session.current_provider_name(),
                        "model": session.current_provider_name(),
                        "prompt": prompt,
                        "promptLength": prompt.len(),
                        "messageCount": session.messages.len(),
                    }),
                    Some(codelet_common::debug_capture::CaptureOptions {
                        request_id: Some(request_id.clone()),
                    }),
                );
            }
        }
    }

    // Start streaming with history and hook
    let mut stream = agent
        .prompt_streaming_with_history_and_hook(prompt, &mut session.messages, hook)
        .await;

    // CLI-022: Capture api.response.start event
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                manager.capture(
                    "api.response.start",
                    serde_json::json!({
                        "provider": session.current_provider_name(),
                    }),
                    Some(codelet_common::debug_capture::CaptureOptions {
                        request_id: Some(request_id.clone()),
                    }),
                );
            }
        }
    }

    // Only create status display and interval for CLI mode (unused in NAPI)
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

    // Track assistant response content for adding to messages (CLI-008)
    let mut assistant_text = String::new();
    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
    let mut last_tool_name: Option<String> = None;

    // Store previous cumulative totals for emitting running totals
    let prev_input_tokens = session.token_tracker.input_tokens;
    let prev_output_tokens = session.token_tracker.output_tokens;

    // Track accumulated tokens within THIS run_agent_stream call
    // (which may have multiple API calls due to tool use)
    let mut turn_accumulated_input: u64 = 0;
    let mut turn_accumulated_output: u64 = 0;
    let mut turn_cache_read: u64 = 0;
    let mut turn_cache_creation: u64 = 0;
    // Track current API call's tokens (reset on FinalResponse for next tool call)
    let mut current_api_input: u64 = 0;
    let mut current_api_output: u64 = 0;

    loop {
        // Check interruption at start of each iteration (works for both modes)
        // Use Acquire ordering to synchronize with Release store from interrupt setter
        if is_interrupted.load(Acquire) {
            // Emit interrupted notification
            let queued = if let Some(ref mut iq) = input_queue {
                iq.dequeue_all()
            } else {
                vec![]
            };
            output.emit_interrupted(&queued);

            // Still add partial response to message history
            if !assistant_text.is_empty() {
                handle_final_response(&assistant_text, &mut session.messages)?;
            }

            output.emit_done();
            break;
        }

        // Process next chunk - different based on mode
        let chunk = match (&mut event_stream, &mut status_interval, &status) {
            (Some(es), Some(si), Some(st)) => {
                // CLI mode: Use tokio::select! with event stream and status interval
                tokio::select! {
                    c = stream.next() => Some(c),
                    event = es.next() => {
                        if let Some(TuiEvent::Key(key)) = event {
                            if key.code == KeyCode::Esc {
                                is_interrupted.store(true, Release);
                            }
                        }
                        None // No chunk, loop will check interrupted flag
                    }
                    _ = si.tick() => {
                        let _ = st.format_status();
                        None // No chunk, continue loop
                    }
                }
            }
            _ => {
                // NAPI mode: Use tokio::select! with interrupt notification (NAPI-004)
                // This allows immediate ESC response even during blocking operations
                match &interrupt_notify {
                    Some(notify) => {
                        let interrupt_fut = notify.notified();
                        tokio::select! {
                            c = stream.next() => Some(c),
                            _ = interrupt_fut => None, // Wakes immediately when interrupt() called
                        }
                    }
                    None => {
                        // Fallback for any mode without notify (shouldn't happen in practice)
                        Some(stream.next().await)
                    }
                }
            }
        };

        // Process chunk if we got one
        if let Some(chunk) = chunk {
            match chunk {
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::Text(text),
                ))) => {
                    handle_text_chunk(&text.text, &mut assistant_text, Some(&request_id), output)?;
                }
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(tool_call),
                ))) => {
                    handle_tool_call(
                        &tool_call,
                        &mut session.messages,
                        &mut assistant_text,
                        &mut tool_calls_buffer,
                        &mut last_tool_name,
                        output,
                    )?;
                }
                Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(
                    tool_result,
                )))) => {
                    handle_tool_result(
                        &tool_result,
                        &mut session.messages,
                        &mut tool_calls_buffer,
                        &last_tool_name,
                        output,
                    )?;

                    // Emit intermediate CUMULATIVE token update after tool result
                    // Use accumulated values (already updated by Usage and FinalResponse handlers)
                    output.emit_tokens(&TokenInfo {
                        input_tokens: prev_input_tokens + turn_accumulated_input,
                        output_tokens: prev_output_tokens + turn_accumulated_output,
                        cache_read_input_tokens: Some(turn_cache_read),
                        cache_creation_input_tokens: Some(turn_cache_creation),
                    });
                }
                Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                    // Usage events come from:
                    // 1. MessageStart (input tokens, output=0) - marks start of new API call
                    // 2. MessageDelta (input + output tokens) - streaming updates

                    if usage.output_tokens == 0 {
                        // MessageStart - new API call starting
                        // First, commit previous API call's tokens (if any) to accumulated totals
                        // This handles multi-API-call turns (tool use) where FinalResponse only comes at end
                        turn_accumulated_input += current_api_input;
                        turn_accumulated_output += current_api_output;
                        // Now track the new API call's input tokens
                        current_api_input = usage.input_tokens;
                        current_api_output = 0;
                    } else {
                        // MessageDelta - update current API call's output tokens
                        current_api_output = usage.output_tokens;
                    }

                    turn_cache_read = usage.cache_read_input_tokens.unwrap_or(turn_cache_read);
                    turn_cache_creation = usage.cache_creation_input_tokens.unwrap_or(turn_cache_creation);

                    // Emit CUMULATIVE totals (previous session + accumulated + current API call)
                    output.emit_tokens(&TokenInfo {
                        input_tokens: prev_input_tokens + turn_accumulated_input + current_api_input,
                        output_tokens: prev_output_tokens + turn_accumulated_output + current_api_output,
                        cache_read_input_tokens: Some(turn_cache_read),
                        cache_creation_input_tokens: Some(turn_cache_creation),
                    });
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                    // Get final usage directly from FinalResponse (most accurate source)
                    let usage = final_resp.usage();

                    // For non-Anthropic providers that don't emit Usage, use FinalResponse values
                    if current_api_input == 0 {
                        current_api_input = usage.input_tokens;
                    }
                    current_api_output = usage.output_tokens;

                    // Commit final API call to accumulated totals
                    turn_accumulated_input += current_api_input;
                    turn_accumulated_output += current_api_output;

                    // Update cache tokens from FinalResponse
                    turn_cache_read = usage.cache_read_input_tokens.unwrap_or(turn_cache_read);
                    turn_cache_creation = usage.cache_creation_input_tokens.unwrap_or(turn_cache_creation);

                    // Emit CUMULATIVE token update
                    output.emit_tokens(&TokenInfo {
                        input_tokens: prev_input_tokens + turn_accumulated_input,
                        output_tokens: prev_output_tokens + turn_accumulated_output,
                        cache_read_input_tokens: Some(turn_cache_read),
                        cache_creation_input_tokens: Some(turn_cache_creation),
                    });

                    // CLI-022: Capture api.response.end event
                    if let Ok(manager_arc) = get_debug_capture_manager() {
                        if let Ok(mut manager) = manager_arc.lock() {
                            if manager.is_enabled() {
                                let duration_ms = api_start_time.elapsed().as_millis() as u64;
                                manager.capture(
                                    "api.response.end",
                                    serde_json::json!({
                                        "duration": duration_ms,
                                        "usage": {
                                            "inputTokens": usage.input_tokens,
                                            "outputTokens": usage.output_tokens,
                                            "cacheReadInputTokens": usage.cache_read_input_tokens,
                                            "cacheCreationInputTokens": usage.cache_creation_input_tokens,
                                        },
                                        "responseLength": assistant_text.len(),
                                    }),
                                    Some(codelet_common::debug_capture::CaptureOptions {
                                        request_id: Some(request_id.clone()),
                                    }),
                                );

                                manager.capture(
                                    "token.update",
                                    serde_json::json!({
                                        "inputTokens": usage.input_tokens,
                                        "outputTokens": usage.output_tokens,
                                        "cacheReadInputTokens": usage.cache_read_input_tokens,
                                        "cacheCreationInputTokens": usage.cache_creation_input_tokens,
                                        "totalInputTokens": usage.input_tokens,
                                        "totalOutputTokens": usage.output_tokens,
                                    }),
                                    None,
                                );
                            }
                        }
                    }

                    handle_final_response(&assistant_text, &mut session.messages)?;
                    output.emit_done();
                    break;
                }
                Some(Err(e)) => {
                    // Check if this error is due to compaction hook cancellation
                    // PromptCancelled means the hook cancelled the request because tokens > threshold
                    let error_str = e.to_string();
                    let is_compaction_cancel = error_str.contains("PromptCancelled");

                    // Check if compaction was actually triggered by the hook
                    let compaction_triggered = token_state
                        .lock()
                        .map(|state| state.compaction_needed)
                        .unwrap_or(false);

                    if is_compaction_cancel && compaction_triggered {
                        // This is a compaction cancellation - break to run compaction logic
                        // Don't log as error, this is expected behavior
                        break;
                    }

                    // CLI-022: Capture api.error event (for real errors, not compaction)
                    if let Ok(manager_arc) = get_debug_capture_manager() {
                        if let Ok(mut manager) = manager_arc.lock() {
                            if manager.is_enabled() {
                                manager.capture(
                                    "api.error",
                                    serde_json::json!({
                                        "error": error_str,
                                        "duration": api_start_time.elapsed().as_millis() as u64,
                                    }),
                                    Some(codelet_common::debug_capture::CaptureOptions {
                                        request_id: Some(request_id.clone()),
                                    }),
                                );
                            }
                        }
                    }
                    output.emit_error(&error_str);
                    return Err(anyhow::anyhow!("Agent error: {e}"));
                }
                None => {
                    // Stream ended unexpectedly
                    if !assistant_text.is_empty() {
                        handle_final_response(&assistant_text, &mut session.messages)?;
                    }
                    output.emit_done();
                    break;
                }
                _ => {
                    // Other stream items (ignored)
                }
            }

            // Flush buffered output after processing each chunk
            // This is a no-op for CLI (unbuffered) but triggers batched text emission for NAPI
            // Provides ~10-50ms latency for text streaming while dramatically reducing callback count
            output.flush();
        }
    }

    // Check if hook triggered compaction
    let compaction_needed = token_state
        .lock()
        .map(|state| state.compaction_needed)
        .unwrap_or(false);

    if compaction_needed && !is_interrupted.load(Acquire) {
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

        output.emit_status("\n[Generating summary...]");

        // Execute compaction
        match execute_compaction(session).await {
            Ok(metrics) => {
                // Capture context.update event after compaction
                if let Ok(manager_arc) = get_debug_capture_manager() {
                    if let Ok(mut manager) = manager_arc.lock() {
                        if manager.is_enabled() {
                            manager.capture(
                                "context.update",
                                serde_json::json!({
                                    "type": "compaction",
                                    "originalTokens": metrics.original_tokens,
                                    "compactedTokens": metrics.compacted_tokens,
                                    "compressionRatio": metrics.compression_ratio,
                                }),
                                None,
                            );
                        }
                    }
                }

                output.emit_status(&format!(
                    "[Context compacted: {}→{} tokens, {:.0}% compression]",
                    metrics.original_tokens,
                    metrics.compacted_tokens,
                    metrics.compression_ratio * 100.0
                ));
                output.emit_status("[Please resend your message - context was compacted]\n");

                // NOTE: execute_compaction already sets session.token_tracker.input_tokens
                // to the correct new_total_tokens calculated from compacted messages.
                // We only reset output_tokens and cache metrics here.
                session.token_tracker.output_tokens = 0;
                session.token_tracker.cache_read_input_tokens = None;
                session.token_tracker.cache_creation_input_tokens = None;

                return Ok(());
            }
            Err(e) => {
                // Compaction failed - DO NOT reset token tracker!
                // Keep the high token values so next turn will retry compaction.
                output.emit_status(&format!("Warning: Compaction failed: {e}"));
                output.emit_status("[Context still large - will retry compaction on next turn]\n");

                // Capture compaction failure for debugging
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

                // Return error so caller knows compaction failed
                return Err(anyhow::anyhow!("Compaction failed: {e}"));
            }
        }
    }

    // Update session token tracker with accumulated values from this turn
    // Use turn_accumulated_* which correctly sums all API calls in this turn
    if !is_interrupted.load(Acquire) {
        session.token_tracker.input_tokens += turn_accumulated_input;
        session.token_tracker.output_tokens += turn_accumulated_output;
        // Cache tokens are per-request, not cumulative (use latest values)
        session.token_tracker.cache_read_input_tokens = Some(turn_cache_read);
        session.token_tracker.cache_creation_input_tokens = Some(turn_cache_creation);
    }

    Ok(())
}
