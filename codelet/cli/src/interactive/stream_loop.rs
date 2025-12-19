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

use super::output::StreamOutput;
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
        output,
    )
    .await
}

/// Run agent stream for NAPI (no event handling)
///
/// This is the NAPI entry point - JavaScript handles keyboard input and sets
/// is_interrupted via the interrupt() method.
pub async fn run_agent_stream<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
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
        output,
    )
    .await
}

/// Internal generic stream loop
///
/// Core streaming logic shared between CLI and NAPI modes.
/// - When event_stream is Some: Uses tokio::select! with event handling (CLI)
/// - When event_stream is None: Direct stream processing with interrupt check (NAPI)
async fn run_agent_stream_internal<M, O, E>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    mut event_stream: Option<&mut E>,
    mut input_queue: Option<&mut InputQueue>,
    is_interrupted: Arc<AtomicBool>,
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
    let mut turn_output_tokens: u64 = 0;

    loop {
        // Check interruption at start of each iteration (works for both modes)
        if is_interrupted.load(Ordering::Relaxed) {
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
                                is_interrupted.store(true, Ordering::Relaxed);
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
                // NAPI mode: Direct stream processing (no event handling)
                Some(stream.next().await)
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
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                    // Output tokens come from FinalResponse
                    let usage = final_resp.usage();
                    turn_output_tokens = usage.output_tokens;

                    // CLI-022: Capture api.response.end event
                    if let Ok(manager_arc) = get_debug_capture_manager() {
                        if let Ok(mut manager) = manager_arc.lock() {
                            if manager.is_enabled() {
                                if let Ok(state) = token_state.lock() {
                                    let duration_ms = api_start_time.elapsed().as_millis() as u64;
                                    manager.capture(
                                        "api.response.end",
                                        serde_json::json!({
                                            "duration": duration_ms,
                                            "usage": {
                                                "inputTokens": state.input_tokens,
                                                "outputTokens": turn_output_tokens,
                                                "cacheReadInputTokens": state.cache_read_input_tokens,
                                                "cacheCreationInputTokens": state.cache_creation_input_tokens,
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
                                            "inputTokens": state.input_tokens,
                                            "outputTokens": turn_output_tokens,
                                            "cacheReadInputTokens": state.cache_read_input_tokens,
                                            "cacheCreationInputTokens": state.cache_creation_input_tokens,
                                            "totalInputTokens": state.input_tokens,
                                            "totalOutputTokens": turn_output_tokens,
                                        }),
                                        None,
                                    );
                                }
                            }
                        }
                    }

                    handle_final_response(&assistant_text, &mut session.messages)?;
                    output.emit_done();
                    break;
                }
                Some(Err(e)) => {
                    // CLI-022: Capture api.error event
                    if let Ok(manager_arc) = get_debug_capture_manager() {
                        if let Ok(mut manager) = manager_arc.lock() {
                            if manager.is_enabled() {
                                manager.capture(
                                    "api.error",
                                    serde_json::json!({
                                        "error": e.to_string(),
                                        "duration": api_start_time.elapsed().as_millis() as u64,
                                    }),
                                    Some(codelet_common::debug_capture::CaptureOptions {
                                        request_id: Some(request_id.clone()),
                                    }),
                                );
                            }
                        }
                    }
                    output.emit_error(&e.to_string());
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
        }
    }

    // Check if hook triggered compaction
    let compaction_needed = token_state
        .lock()
        .map(|state| state.compaction_needed)
        .unwrap_or(false);

    if compaction_needed && !is_interrupted.load(Ordering::Relaxed) {
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
            }
            Err(e) => {
                output.emit_status(&format!("Warning: Compaction failed: {e}"));
            }
        }

        // Reset token tracker after compaction
        session.token_tracker.input_tokens = 0;
        session.token_tracker.output_tokens = 0;
        session.token_tracker.cache_read_input_tokens = None;
        session.token_tracker.cache_creation_input_tokens = None;

        return Ok(());
    }

    // Update tokens from hook's TokenState (accumulate, don't overwrite)
    if !is_interrupted.load(Ordering::Relaxed) {
        if let Ok(state) = token_state.lock() {
            session.token_tracker.input_tokens += state.input_tokens;
            session.token_tracker.output_tokens += turn_output_tokens;
            // Cache tokens are per-request, not cumulative
            session.token_tracker.cache_read_input_tokens = Some(state.cache_read_input_tokens);
            session.token_tracker.cache_creation_input_tokens =
                Some(state.cache_creation_input_tokens);
        }
    }

    Ok(())
}
