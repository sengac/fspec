//! Agent streaming loop with interruption support
//!
//! Handles the main agent streaming loop including token tracking,
//! debug capture, and compaction triggering.

use super::estimate_tokens;
use super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};
use crate::interactive_helpers::execute_compaction;
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_core::RigAgent;
use codelet_tui::{InputQueue, StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::CompletionModel;
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
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
{
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;
    use std::time::Instant;
    use uuid::Uuid;

    // CLI-022: Generate request ID for correlation
    let request_id = Uuid::new_v4().to_string();
    let api_start_time = Instant::now();

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

    // CRITICAL: Add user prompt to message history for persistence (CLI-008)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(prompt)),
    });

    // CRITICAL FIX: Pass conversation history to rig for context persistence (CLI-008)
    let mut stream = agent
        .prompt_streaming_with_history(prompt, &mut session.messages)
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

    // CLI-010: Track token usage for compaction
    let mut turn_input_tokens: u64 = 0;
    let mut turn_output_tokens: u64 = 0;
    let mut turn_cache_read_tokens: Option<u64> = None;
    let mut turn_cache_creation_tokens: Option<u64> = None;

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
                        // CLI-010: Extract token usage from FinalResponse
                        let usage = final_resp.usage();

                        // Use API values if available, otherwise estimate (matches codelet fallback pattern)
                        turn_input_tokens = if usage.input_tokens > 0 {
                            usage.input_tokens
                        } else {
                            estimate_tokens(prompt)
                        };

                        turn_output_tokens = if usage.output_tokens > 0 {
                            usage.output_tokens
                        } else {
                            estimate_tokens(&assistant_text)
                        };

                        // PROV-006: Cache token extraction from Anthropic API
                        //
                        // REQUEST-SIDE (WORKING): ClaudeProvider.create_rig_agent() uses additional_params
                        // to transform system prompts to array format with cache_control metadata.
                        // This enables Anthropic's prompt caching on outgoing requests.
                        //
                        // RESPONSE-SIDE (NOW WORKING): Patched rig-core exposes cache tokens in Usage.
                        // The patch adds cache_read_input_tokens and cache_creation_input_tokens fields
                        // to rig's generic Usage struct, and extracts them from MessageStart SSE events.
                        turn_cache_read_tokens = usage.cache_read_input_tokens;
                        turn_cache_creation_tokens = usage.cache_creation_input_tokens;

                        // CLI-022: Capture api.response.end event with token usage
                        if let Ok(manager_arc) = get_debug_capture_manager() {
                            if let Ok(mut manager) = manager_arc.lock() {
                                if manager.is_enabled() {
                                    let duration_ms = api_start_time.elapsed().as_millis() as u64;
                                    manager.capture(
                                        "api.response.end",
                                        serde_json::json!({
                                            "duration": duration_ms,
                                            "usage": {
                                                "inputTokens": turn_input_tokens,
                                                "outputTokens": turn_output_tokens,
                                                "cacheReadInputTokens": turn_cache_read_tokens,
                                                "cacheCreationInputTokens": turn_cache_creation_tokens,
                                            },
                                            "responseLength": assistant_text.len(),
                                        }),
                                        Some(codelet_common::debug_capture::CaptureOptions {
                                            request_id: Some(request_id.clone()),
                                        }),
                                    );

                                    // CLI-022: Capture token.update event
                                    // Note: totalInputTokens = inputTokens since we REPLACE (not accumulate)
                                    manager.capture(
                                        "token.update",
                                        serde_json::json!({
                                            "inputTokens": turn_input_tokens,
                                            "outputTokens": turn_output_tokens,
                                            "cacheReadInputTokens": turn_cache_read_tokens,
                                            "cacheCreationInputTokens": turn_cache_creation_tokens,
                                            "totalInputTokens": turn_input_tokens,
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

    // CLI-010: After stream completes, update tokens and check compaction
    if !is_interrupted.load(Ordering::Relaxed) {
        // Replace token tracker with API usage values (matches TypeScript runner.ts:1143)
        // TypeScript: inputTokens: usage.inputTokens ?? tokenTracker.inputTokens
        // The API reports total input tokens for the current context, not incremental
        session.token_tracker.input_tokens = turn_input_tokens;
        session.token_tracker.output_tokens = turn_output_tokens;
        if let Some(cache_read) = turn_cache_read_tokens {
            let current = session.token_tracker.cache_read_input_tokens.unwrap_or(0);
            session.token_tracker.cache_read_input_tokens = Some(current + cache_read);
        }
        if let Some(cache_create) = turn_cache_creation_tokens {
            let current = session
                .token_tracker
                .cache_creation_input_tokens
                .unwrap_or(0);
            session.token_tracker.cache_creation_input_tokens = Some(current + cache_create);
        }

        // CTX-002: Removed eager turn creation - now done lazily during compaction
        // Turns are created from session.messages inside execute_compaction() following TypeScript implementation

        // Check if compaction should trigger
        // CLI-015: Use model-specific context window instead of hardcoded value
        // CLI-020: Apply autocompact buffer to leave headroom after compaction
        // FIX: Use message estimation instead of rig's aggregated_usage, because rig
        // accumulates tokens across all tool call iterations in a multi-turn agent call.
        // For compaction, we need the CURRENT context size, not accumulated API usage.
        use crate::compaction_threshold::calculate_compaction_threshold;
        use crate::interactive_helpers::estimate_message_tokens;
        let context_window = session.provider_manager().context_window() as u64;
        let threshold = calculate_compaction_threshold(context_window);
        let effective = estimate_message_tokens(&session.messages);

        if effective > threshold {
            // CLI-022: Capture compaction.triggered event
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "compaction.triggered",
                            serde_json::json!({
                                "effectiveTokens": effective,
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
                    // CLI-022: Capture context.update event after compaction
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
                        "[Context compacted: {}→{} tokens, {:.0}% compression]\n",
                        metrics.original_tokens,
                        metrics.compacted_tokens,
                        metrics.compression_ratio * 100.0
                    );
                }
                Err(e) => {
                    eprintln!("Warning: Compaction failed: {e}");
                }
            }
        }
    }

    Ok(())
}
