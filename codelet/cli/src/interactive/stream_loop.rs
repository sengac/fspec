//! Agent streaming loop with interruption support
//!
//! Handles the main agent streaming loop including token tracking,
//! debug capture, and compaction triggering.
//!
//! Uses rig's StreamingPromptHook to capture per-request token usage and
//! check compaction thresholds before each internal API call, matching
//! TypeScript's approach exactly.

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
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::interval;

pub(super) async fn run_agent_stream_with_interruption<M>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session, // CLI-010: Need full session for token tracking and compaction
    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),
    input_queue: &mut InputQueue,
    is_interrupted: Arc<AtomicBool>,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
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

    // HOOK-BASED COMPACTION (matches TypeScript runner.ts:829-836 exactly)
    // Uses rig's StreamingPromptHook to capture per-request API token usage
    // and check thresholds before each internal API call (including tool iterations).
    //
    // The hook:
    // - on_completion_call: Called BEFORE each API call - checks threshold, cancels if exceeded
    // - on_stream_completion_response_finish: Called AFTER each API call - captures per-request usage
    //
    // This matches TypeScript which checks shouldTriggerCompaction() before each streamText() call.
    let context_window = session.provider_manager().context_window() as u64;
    let threshold = calculate_compaction_threshold(context_window);

    // Initialize TokenState with values from last API call (from session.token_tracker)
    // This is what TypeScript does - it uses tokenTracker values from the previous call
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens,
        cache_read_input_tokens: session.token_tracker.cache_read_input_tokens.unwrap_or(0),
        cache_creation_input_tokens: session.token_tracker.cache_creation_input_tokens.unwrap_or(0),
        compaction_needed: false,
    }));

    let hook = CompactionHook::new(Arc::clone(&token_state), threshold);

    // DEBUG: Log compaction check setup
    if let Ok(manager_arc) = get_debug_capture_manager() {
        if let Ok(mut manager) = manager_arc.lock() {
            if manager.is_enabled() {
                let state = token_state.lock().unwrap();
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

    // CRITICAL FIX: Pass conversation history AND hook to rig for context persistence (CLI-008)
    // The hook captures per-request token usage and triggers compaction when needed
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

    let status = StatusDisplay::new();
    let mut status_interval = interval(Duration::from_secs(1));

    // Track assistant response content for adding to messages (CLI-008)
    let mut assistant_text = String::new();
    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();

    // Track last tool call for debug logging
    let mut last_tool_name: Option<String> = None;

    // CLI-010: Track output tokens (input/cache tokens come from hook's TokenState)
    let mut turn_output_tokens: u64 = 0;

    loop {
        tokio::select! {
            // Agent streaming
            chunk = stream.next() => {
                if is_interrupted.load(Ordering::Relaxed) {
                    // Interrupted - show queued inputs
                    println!("\n⚠️ Agent interrupted");
                    let queued = input_queue.dequeue_all();
                    if queued.is_empty() {
                        println!("Queued inputs: (none)");
                    } else {
                        println!("Queued inputs:\n{}", queued.join("\n\n"));
                    }
                    break;
                }

                match chunk {
                    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text)))) => {
                        handle_text_chunk(
                            &text.text,
                            &mut assistant_text,
                            Some(&request_id),
                        )?;
                    }
                    Some(Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(tool_call)))) => {
                        handle_tool_call(
                            &tool_call,
                            &mut session.messages,
                            &mut assistant_text,
                            &mut tool_calls_buffer,
                            &mut last_tool_name,
                        )?;
                    }
                    Some(Ok(MultiTurnStreamItem::StreamUserItem(StreamedUserContent::ToolResult(tool_result)))) => {
                        handle_tool_result(
                            &tool_result,
                            &mut session.messages,
                            &mut tool_calls_buffer,
                            &last_tool_name,
                        )?;
                    }
                    Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                        // Output tokens come from FinalResponse (aggregated across all tool iterations)
                        let usage = final_resp.usage();
                        turn_output_tokens = usage.output_tokens;

                        // CLI-022: Capture api.response.end event with token usage
                        // Get per-request values from hook's TokenState
                        if let Ok(manager_arc) = get_debug_capture_manager() {
                            if let Ok(mut manager) = manager_arc.lock() {
                                if manager.is_enabled() {
                                    let state = token_state.lock().unwrap();
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

                                    // CLI-022: Capture token.update event
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

                        handle_final_response(
                            &assistant_text,
                            &mut session.messages,
                        )?;
                        // Stream complete
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
                        return Err(anyhow::anyhow!("Agent error: {e}"));
                    }
                    _ => {
                        // Other stream items
                    }
                }
            }

            // Terminal events
            event = event_stream.next() => {
                match event {
                    Some(TuiEvent::Key(key)) if key.code == KeyCode::Esc => {
                        is_interrupted.store(true, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }

            // Status display updates
            _ = status_interval.tick() => {
                // Update status (in real implementation, would render to UI)
                // For now, just track elapsed time
                let _ = status.format_status();
            }
        }
    }

    // Check if hook triggered compaction (cancelled before API call could proceed)
    let compaction_needed = {
        let state = token_state.lock().unwrap();
        state.compaction_needed
    };

    if compaction_needed && !is_interrupted.load(Ordering::Relaxed) {
        // Capture compaction.triggered event
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    let state = token_state.lock().unwrap();
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

        println!("\n[Generating summary...]");
        std::io::stdout().flush()?;

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

                println!(
                    "[Context compacted: {}→{} tokens, {:.0}% compression]",
                    metrics.original_tokens,
                    metrics.compacted_tokens,
                    metrics.compression_ratio * 100.0
                );
                println!("[Please resend your message - context was compacted]\n");
            }
            Err(e) => {
                eprintln!("Warning: Compaction failed: {e}");
            }
        }

        // Reset token tracker after compaction
        session.token_tracker.input_tokens = 0;
        session.token_tracker.output_tokens = 0;
        session.token_tracker.cache_read_input_tokens = None;
        session.token_tracker.cache_creation_input_tokens = None;

        return Ok(());
    }

    // CLI-010: After stream completes, update tokens from hook's TokenState
    // The hook captures per-request values in on_stream_completion_response_finish
    if !is_interrupted.load(Ordering::Relaxed) {
        // Get final token values from hook state (per-request, not accumulated)
        let state = token_state.lock().unwrap();
        session.token_tracker.input_tokens = state.input_tokens;
        session.token_tracker.output_tokens = turn_output_tokens;
        session.token_tracker.cache_read_input_tokens = Some(state.cache_read_input_tokens);
        session.token_tracker.cache_creation_input_tokens = Some(state.cache_creation_input_tokens);
    }

    Ok(())
}
