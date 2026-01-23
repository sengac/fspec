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

use super::output::{ContextFillInfo, StreamOutput};
use super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};
use crate::compaction_threshold::calculate_usable_context;
use crate::interactive_helpers::execute_compaction;
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_common::token_estimator::count_tokens;
use codelet_core::{ApiTokenUsage, CompactionHook, RigAgent, TokenState, ensure_thought_signatures, GeminiTurnCompletionFacade, TurnCompletionFacade, ContinuationStrategy, StreamingTokenDisplay};
use codelet_tools::set_tool_progress_callback;
use codelet_tui::{InputQueue, StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::streaming::{StreamedAssistantContent, StreamedUserContent};
use rig::wasm_compat::WasmCompatSend;
use std::error::Error as StdError;
use std::sync::atomic::AtomicBool;
// Use Acquire/Release ordering for proper cross-thread synchronization
// - Acquire: Ensures subsequent reads see all writes before the Release store
// - Release: Ensures all writes before the store are visible to Acquire loads
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::time::interval;
use tracing::{error, info, trace};

/// Check if an error indicates the prompt/context is too long
fn is_prompt_too_long_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();
    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}

/// CMPCT-002: Check if an error indicates compaction was cancelled by the hook
/// This is used to detect when the CompactionHook cancels a request due to token threshold
fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains("PromptCancelled")
}

/// CMPCT-002: Signal that compaction is needed by setting the flag in token state
/// This allows the post-loop compaction logic to detect and handle it
fn signal_compaction_needed(token_state: &Arc<Mutex<TokenState>>) {
    if let Ok(mut state) = token_state.lock() {
        state.compaction_needed = true;
    }
}

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
#[allow(clippy::too_many_arguments)]
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

    // HOOK-BASED COMPACTION (CTX-002: Optimized compaction trigger)
    let context_window = session.provider_manager().context_window() as u64;
    let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
    // CTX-002: Use usable_context (context_window - output_reservation) instead of 90% threshold
    let threshold = calculate_usable_context(context_window, max_output_tokens);

    // CTX-005: PRE-PROMPT COMPACTION CHECK
    // Before adding the new prompt, estimate if current context + new prompt would exceed threshold.
    // This prevents "prompt is too long" API errors when resuming a session at high context fill.
    // The hook only checks AFTER API responses, but we need to check BEFORE the first API call.
    let prompt_tokens = count_tokens(prompt) as u64;
    let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
    let estimated_total = current_tokens + prompt_tokens;

    if estimated_total > threshold && !session.messages.is_empty() {
        trace!(
            "Pre-prompt compaction triggered: estimated {} > threshold {}",
            estimated_total, threshold
        );
        output.emit_status("\n[Context near limit, generating summary...]");

        match execute_compaction(session).await {
            Ok(metrics) => {
                output.emit_status(&format!(
                    "[Context compacted: {}→{} tokens, {:.0}% compression]",
                    metrics.original_tokens,
                    metrics.compacted_tokens,
                    metrics.compression_ratio * 100.0
                ));
                output.emit_status("[Continuing with compacted context...]\n");

                // CMPCT-001: Reset output and cache metrics after compaction
                session.token_tracker.reset_after_compaction();
            }
            Err(e) => {
                // Log but continue - the API might still work, or will fail with clear error
                error!("Pre-prompt compaction failed: {}", e);
                output.emit_status(&format!("[Compaction failed: {e}, continuing anyway...]"));
            }
        }
    }

    // CRITICAL: Add user prompt to message history for persistence (CLI-008)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(prompt)),
    });

    // GEMINI-THINK: For Gemini preview models with thinking enabled, ensure thought signatures
    // are present on function calls in the active loop. Without this, Gemini 2.5/3 preview
    // models return 400 errors or stop responding after tool calls.
    // See: https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/core/geminiChat.ts
    if let Some(model_id) = session.current_model_id() {
        ensure_thought_signatures(&mut session.messages, &model_id);
    } else {
        // Fallback to provider name check for backwards compatibility
        let provider = session.current_provider_name();
        if provider == "gemini" {
            // Use a generic model that enables preparation for all Gemini models
            ensure_thought_signatures(&mut session.messages, "gemini-2.5-preview");
        }
    }

    // TUI-033: Helper to emit context fill percentage after token updates
    // PROV-001: Uses ApiTokenUsage.total_context() for consistent calculation
    let emit_context_fill_from_usage = |output: &O,
                                        usage: &ApiTokenUsage,
                                        threshold: u64,
                                        context_window: u64| {
        let total_tokens = usage.total_context();
        // Calculate fill percentage (can exceed 100% near compaction)
        let fill_percentage = if threshold > 0 {
            ((total_tokens as f64 / threshold as f64) * 100.0) as u32
        } else {
            0
        };
        output.emit_context_fill(&ContextFillInfo {
            fill_percentage,
            effective_tokens: total_tokens,
            threshold,
            context_window,
        });
    };

    // PROV-001: session.token_tracker.input_tokens stores TOTAL context (input + cache_read + cache_creation)
    // Initialize cache values to 0 to avoid double-counting in TokenState::total()
    // During streaming, on_stream_completion_response_finish will update with actual API values
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens, // Already includes cache
        cache_read_input_tokens: 0,                       // Don't double count
        cache_creation_input_tokens: 0,                   // Don't double count
        // CTX-002: Include output tokens in TokenState for accurate total calculation
        output_tokens: session.token_tracker.output_tokens,
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

    // TOOL-011: Set up tool progress callback for streaming bash output
    // Get the progress emitter from output - this returns an Arc<dyn StreamOutput>
    // that can be captured by the 'static callback closure.
    //
    // KEY INSIGHT: tokio::select! waits for ONE branch to COMPLETE. When stream.next()
    // is executing and a tool runs inside it, the entire tool execution happens within
    // that single poll. Even though spawned tasks send to a channel, by the time select!
    // could check the channel, stream.next() has already returned Ready(ToolResult).
    //
    // SOLUTION: The callback emits DIRECTLY through StreamOutput, bypassing the channel.
    // This works because the callback is called from a spawned tokio task inside the tool,
    // which runs on the tokio runtime and can make I/O calls (print for CLI, or
    // ThreadsafeFunction::call for NAPI which is NonBlocking).
    if let Some(emitter) = output.progress_emitter() {
        set_tool_progress_callback(Some(Arc::new(move |chunk: &str, is_stderr: bool| {
            emitter.emit_tool_progress("", "bash", chunk, is_stderr);
        })));
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

    // Track previous session state for initial display
    let prev_input_tokens = session.token_tracker.input_tokens;
    let prev_output_tokens = session.token_tracker.output_tokens;
    let prev_cache_read = session.token_tracker.cache_read_input_tokens.unwrap_or(0);
    let prev_cache_creation = session
        .token_tracker
        .cache_creation_input_tokens
        .unwrap_or(0);

    // STREAMING-DISPLAY: Use StreamingTokenDisplay to track tokens during streaming
    // This encapsulates:
    // - Output token tracking (estimated vs authoritative)
    // - Tok/s rate calculation with EMA smoothing
    // - Display throttling to prevent UI flicker
    let mut streaming_display = StreamingTokenDisplay::new(
        prev_input_tokens,
        prev_output_tokens,
        prev_cache_read,
        prev_cache_creation,
    );

    // Emit initial token state at start of prompt so display shows current session state
    // (prevents flash to 0 when starting new prompt)
    // PROV-001: prev_input_tokens ALREADY contains total context (stored that way)
    trace!(
        "Initial token emit: prev_input_tokens={}, prev_output_tokens={}, cache_read={}, cache_creation={}",
        prev_input_tokens, prev_output_tokens, prev_cache_read, prev_cache_creation
    );
    output.emit_tokens(&streaming_display.current().into());
    // CTX-004: For initial state, use 0 for output since no new output yet in this turn
    // PROV-001: Create initial usage with prev_input_tokens as raw (cache already counted in it)
    let initial_usage = ApiTokenUsage::new(prev_input_tokens, 0, 0, 0);
    emit_context_fill_from_usage(output, &initial_usage, threshold, context_window);

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
                // NOTE: Tool progress is emitted directly via progress_emitter callback,
                // not through tokio::select! because select! can't interleave during stream.next()
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
                // NOTE: Tool progress is emitted directly via progress_emitter callback,
                // not through tokio::select! because select! can't interleave during stream.next()
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

                    // STREAMING-DISPLAY: Track chunk and emit if not throttled
                    if let Some(update) = streaming_display.record_chunk(&text.text) {
                        output.emit_tokens(&update.into());
                    }
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
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ReasoningDelta { reasoning, .. },
                ))) => {
                    // TOOL-010: Emit thinking/reasoning content from extended thinking
                    output.emit_thinking(&reasoning);

                    // STREAMING-DISPLAY: Track thinking chunk and emit if not throttled
                    if let Some(update) = streaming_display.record_chunk(&reasoning) {
                        output.emit_tokens(&update.into());
                    }
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

                    // PROV-001: Don't emit token updates after tool results
                    // This is when a new API segment is about to start, and the next
                    // MessageStart may have very different input values (cache growing).
                    // Emitting here causes "bouncing" in the display.
                    // Token updates only occur during text streaming and at FinalResponse.
                }
                Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                    // Usage events come from:
                    // 1. MessageStart (input tokens, output=0) - marks start of new API call (Anthropic)
                    // 2. MessageDelta (input + output tokens) - streaming updates (Anthropic)
                    // 3. Gemini: Every SSE chunk with usage_metadata (may have output > 0 from start)
                    //
                    // PROV-001: For Anthropic, only emit on MessageDelta (output > 0), NOT on
                    // MessageStart which causes "bouncing" during tool loops.
                    // For Gemini: Always update input_tokens since they don't have separate start/delta events.

                    if usage.output_tokens == 0 {
                        // MessageStart - new API call starting (Anthropic pattern)
                        // STREAMING-DISPLAY: Start new segment (accumulates previous output)
                        streaming_display.start_new_segment(&usage);
                    } else {
                        // MessageDelta - authoritative usage data available
                        // STREAMING-DISPLAY: Update from usage and emit
                        if let Some(update) = streaming_display.update_from_usage(&usage) {
                            output.emit_tokens(&update.into());
                            // CTX-004: Context fill uses CURRENT API values
                            let fill_usage = ApiTokenUsage::new(
                                update.input_tokens,
                                update.cache_read_tokens,
                                update.cache_creation_tokens,
                                usage.output_tokens, // Current segment output for fill calculation
                            );
                            emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);
                        }
                    }
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                    // Get usage from FinalResponse
                    let usage = final_resp.usage();

                    // PROV-002: OpenAI-compatible providers (including Z.AI) don't emit Usage events
                    // during streaming - they only return usage in FinalResponse.
                    // STREAMING-DISPLAY: update_from_final_response handles this case
                    let final_update = if !streaming_display.has_authoritative_output() && usage.input_tokens > 0 {
                        // OpenAI-compatible path: no Usage events during streaming
                        trace!(
                            "OpenAI-compatible provider: extracted tokens from FinalResponse - input={}, output={}, cache_read={:?}",
                            usage.input_tokens, usage.output_tokens, usage.cache_read_input_tokens
                        );
                        streaming_display.update_from_final_response(&usage)
                    } else {
                        // Anthropic/Gemini path: already have authoritative values from Usage events
                        // Just get current display values
                        streaming_display.current()
                    };

                    // Emit final token update
                    output.emit_tokens(&final_update.into());
                    // CTX-004: Context fill uses current values
                    let fill_usage = ApiTokenUsage::new(
                        final_update.input_tokens,
                        final_update.cache_read_tokens,
                        final_update.cache_creation_tokens,
                        usage.output_tokens,
                    );
                    emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);

                    // CLI-022: Capture api.response.end event
                    // PROV-001: Capture both aggregated (for billing) and display (for UI debugging) values
                    if let Ok(manager_arc) = get_debug_capture_manager() {
                        if let Ok(mut manager) = manager_arc.lock() {
                            if manager.is_enabled() {
                                let duration_ms = api_start_time.elapsed().as_millis() as u64;
                                manager.capture(
                                    "api.response.end",
                                    serde_json::json!({
                                        "duration": duration_ms,
                                        // Aggregated usage from FinalResponse (sum of all API segments - for billing)
                                        "aggregatedUsage": {
                                            "inputTokens": usage.input_tokens,
                                            "outputTokens": usage.output_tokens,
                                            "cacheReadInputTokens": usage.cache_read_input_tokens,
                                            "cacheCreationInputTokens": usage.cache_creation_input_tokens,
                                            "totalInputTokens": usage.input_tokens
                                                + usage.cache_read_input_tokens.unwrap_or(0)
                                                + usage.cache_creation_input_tokens.unwrap_or(0),
                                        },
                                        // Display usage (last segment's values - what UI shows)
                                        "displayUsage": {
                                            "inputTokens": final_update.input_tokens,
                                            "outputTokens": final_update.output_tokens,
                                            "cacheReadInputTokens": final_update.cache_read_tokens,
                                            "cacheCreationInputTokens": final_update.cache_creation_tokens,
                                            "totalInputTokens": final_update.total_input(),
                                        },
                                        "responseLength": assistant_text.len(),
                                    }),
                                    Some(codelet_common::debug_capture::CaptureOptions {
                                        request_id: Some(request_id.clone()),
                                    }),
                                );

                                // PROV-001: token.update uses DISPLAY values (what user sees)
                                // These are the last segment's values, not aggregated
                                manager.capture(
                                    "token.update",
                                    serde_json::json!({
                                        "inputTokens": final_update.input_tokens,
                                        "outputTokens": final_update.output_tokens,
                                        "cacheReadInputTokens": final_update.cache_read_tokens,
                                        "cacheCreationInputTokens": final_update.cache_creation_tokens,
                                        "totalInputTokens": final_update.total_input(),
                                        "totalOutputTokens": final_update.output_tokens,
                                    }),
                                    None,
                                );
                            }
                        }
                    }

                    // GEMINI-TURN: Check if Gemini model returned empty response after tool call
                    // and needs a continuation prompt to nudge it to respond with the results.
                    // 
                    // The facade returns a ContinuationStrategy that tells us HOW to continue:
                    // - None: Response is complete, proceed normally
                    // - FullLoop: Re-run the full agentic loop (handles tool calls in continuation)
                    let provider_name = session.current_provider_name();
                    let model_id = session.current_model_id().unwrap_or_default();
                    
                    if provider_name == "gemini" {
                        let turn_completion = GeminiTurnCompletionFacade;
                        if turn_completion.requires_turn_completion_check(&model_id) {
                            let strategy = turn_completion.continuation_strategy(&assistant_text, &session.messages);
                            
                            if let ContinuationStrategy::FullLoop { prompt: continuation_prompt } = strategy {
                                // Log that we need a continuation prompt
                                info!(
                                    "GEMINI-TURN: Empty response after tool call detected for model {}, using FullLoop strategy",
                                    model_id
                                );
                                
                                // Capture continuation event for debugging
                                if let Ok(manager_arc) = get_debug_capture_manager() {
                                    if let Ok(mut manager) = manager_arc.lock() {
                                        if manager.is_enabled() {
                                            manager.capture(
                                                "gemini.continuation",
                                                serde_json::json!({
                                                    "reason": "empty_response_after_tool",
                                                    "strategy": "FullLoop",
                                                    "model": model_id,
                                                    "prompt": continuation_prompt,
                                                }),
                                                None,
                                            );
                                        }
                                    }
                                }
                                
                                // Handle final response (add empty assistant text to history)
                                handle_final_response(&assistant_text, &mut session.messages)?;
                                
                                // GEMINI-TURN-002: Use recursive full loop for continuation
                                // This allows the continuation to handle tool calls properly,
                                // unlike the previous inline approach that only handled text.
                                //
                                // We add the continuation prompt to messages, update session state,
                                // and DON'T emit done - the outer loop will continue.
                                
                                // Add the continuation as a new user message
                                session.messages.push(rig::message::Message::User {
                                    content: rig::OneOrMany::one(rig::message::UserContent::text(continuation_prompt)),
                                });
                                
                                // Prepare history again for Gemini (add thought signatures)
                                if let Some(model_id) = session.current_model_id() {
                                    ensure_thought_signatures(&mut session.messages, &model_id);
                                }
                                
                                // CMPCT-001: Update display values before recursion (no billing accumulation yet)
                                // Use current values from streaming_display
                                let current_display = streaming_display.current();
                                let turn_usage = ApiTokenUsage::new(
                                    current_display.input_tokens,
                                    current_display.cache_read_tokens,
                                    current_display.cache_creation_tokens,
                                    0,
                                );
                                session.token_tracker.update_display_only(&turn_usage, current_display.output_tokens);
                                
                                // Create a new hook and token state for the continuation
                                let continuation_token_state = Arc::new(Mutex::new(TokenState {
                                    input_tokens: session.token_tracker.input_tokens,
                                    cache_read_input_tokens: current_display.cache_read_tokens,
                                    cache_creation_input_tokens: current_display.cache_creation_tokens,
                                    output_tokens: current_display.output_tokens,
                                    compaction_needed: false,
                                }));
                                let continuation_hook = CompactionHook::new(Arc::clone(&continuation_token_state), threshold);
                                
                                // Start a new FULL stream for the continuation
                                // This stream can handle tool calls, unlike the previous simple approach
                                let mut continuation_stream = agent
                                    .prompt_streaming_with_history_and_hook(
                                        continuation_prompt,
                                        &mut session.messages,
                                        continuation_hook,
                                    )
                                    .await;
                                
                                // Track continuation state
                                let mut continuation_text = String::new();
                                let mut continuation_tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
                                let mut continuation_last_tool_name: Option<String> = None;
                                
                                // STREAMING-DISPLAY: Create continuation display tracker
                                let mut continuation_display = StreamingTokenDisplay::new(
                                    current_display.input_tokens,
                                    current_display.output_tokens,
                                    current_display.cache_read_tokens,
                                    current_display.cache_creation_tokens,
                                );
                                
                                // Process the continuation stream - FULL loop with tool support
                                loop {
                                    // Check interruption
                                    if is_interrupted.load(Acquire) {
                                        let queued = if let Some(ref mut iq) = input_queue {
                                            iq.dequeue_all()
                                        } else {
                                            vec![]
                                        };
                                        output.emit_interrupted(&queued);
                                        if !continuation_text.is_empty() {
                                            handle_final_response(&continuation_text, &mut session.messages)?;
                                        }
                                        
                                        // CMPCT-001: Update token tracker with billing accumulation on interrupt
                                        let cont_final = continuation_display.current();
                                        let cont_usage = ApiTokenUsage::new(
                                            cont_final.input_tokens,
                                            cont_final.cache_read_tokens,
                                            cont_final.cache_creation_tokens,
                                            0,
                                        );
                                        session.token_tracker.update_from_usage(&cont_usage, cont_final.output_tokens);
                                        
                                        // Clear tool progress callback before returning
                                        set_tool_progress_callback(None);
                                        output.emit_done();
                                        return Ok(());
                                    }
                                    
                                    match continuation_stream.next().await {
                                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                                            StreamedAssistantContent::Text(text),
                                        ))) => {
                                            handle_text_chunk(&text.text, &mut continuation_text, None, output)?;
                                            // STREAMING-DISPLAY: Record chunk in continuation display
                                            if let Some(update) = continuation_display.record_chunk(&text.text) {
                                                output.emit_tokens(&update.into());
                                            }
                                        }
                                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                                            StreamedAssistantContent::ToolCall(tool_call),
                                        ))) => {
                                            // GEMINI-TURN-002: Handle tool calls in continuation
                                            handle_tool_call(
                                                &tool_call,
                                                &mut session.messages,
                                                &mut continuation_text,
                                                &mut continuation_tool_calls_buffer,
                                                &mut continuation_last_tool_name,
                                                output,
                                            )?;
                                        }
                                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                                            StreamedAssistantContent::ReasoningDelta { reasoning, .. },
                                        ))) => {
                                            output.emit_thinking(&reasoning);
                                            // STREAMING-DISPLAY: Record thinking chunk
                                            if let Some(update) = continuation_display.record_chunk(&reasoning) {
                                                output.emit_tokens(&update.into());
                                            }
                                        }
                                        Some(Ok(MultiTurnStreamItem::StreamUserItem(
                                            StreamedUserContent::ToolResult(tool_result),
                                        ))) => {
                                            // GEMINI-TURN-002: Handle tool results in continuation
                                            handle_tool_result(
                                                &tool_result,
                                                &mut session.messages,
                                                &mut continuation_tool_calls_buffer,
                                                &continuation_last_tool_name,
                                                output,
                                            )?;
                                        }
                                        Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                                            // STREAMING-DISPLAY: Update from usage event
                                            if usage.output_tokens == 0 {
                                                continuation_display.start_new_segment(&usage);
                                            } else if let Some(update) = continuation_display.update_from_usage(&usage) {
                                                output.emit_tokens(&update.into());
                                            }
                                        }
                                        Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                                            // Get usage from FinalResponse
                                            let usage = final_resp.usage();

                                            // STREAMING-DISPLAY: Update from final response if needed
                                            let cont_final = if !continuation_display.has_authoritative_output() && usage.input_tokens > 0 {
                                                continuation_display.update_from_final_response(&usage)
                                            } else {
                                                continuation_display.current()
                                            };

                                            // GEMINI-TURN-002: Check if we need ANOTHER continuation
                                            // This handles the case where multiple tool calls happen in sequence
                                            let nested_strategy = turn_completion.continuation_strategy(
                                                &continuation_text,
                                                &session.messages,
                                            );
                                            
                                            if let ContinuationStrategy::FullLoop { prompt: nested_prompt } = nested_strategy {
                                                info!(
                                                    "GEMINI-TURN: Nested empty response detected, continuing again"
                                                );
                                                
                                                // Capture nested continuation for debugging
                                                if let Ok(manager_arc) = get_debug_capture_manager() {
                                                    if let Ok(mut manager) = manager_arc.lock() {
                                                        if manager.is_enabled() {
                                                            manager.capture(
                                                                "gemini.continuation",
                                                                serde_json::json!({
                                                                    "reason": "nested_empty_response_after_tool",
                                                                    "strategy": "FullLoop",
                                                                    "model": model_id,
                                                                    "prompt": nested_prompt,
                                                                }),
                                                                None,
                                                            );
                                                        }
                                                    }
                                                }
                                                
                                                // Add continuation text to history
                                                handle_final_response(&continuation_text, &mut session.messages)?;
                                                continuation_text.clear();
                                                
                                                // Add nested continuation prompt
                                                session.messages.push(rig::message::Message::User {
                                                    content: rig::OneOrMany::one(rig::message::UserContent::text(nested_prompt)),
                                                });
                                                
                                                // Prepare history again
                                                if let Some(model_id) = session.current_model_id() {
                                                    ensure_thought_signatures(&mut session.messages, &model_id);
                                                }
                                                
                                                // Create new stream for nested continuation
                                                let nested_token_state = Arc::new(Mutex::new(TokenState {
                                                    input_tokens: cont_final.input_tokens,
                                                    cache_read_input_tokens: cont_final.cache_read_tokens,
                                                    cache_creation_input_tokens: cont_final.cache_creation_tokens,
                                                    output_tokens: cont_final.output_tokens,
                                                    compaction_needed: false,
                                                }));
                                                let nested_hook = CompactionHook::new(Arc::clone(&nested_token_state), threshold);
                                                
                                                continuation_stream = agent
                                                    .prompt_streaming_with_history_and_hook(
                                                        nested_prompt,
                                                        &mut session.messages,
                                                        nested_hook,
                                                    )
                                                    .await;
                                                
                                                // Reset for next iteration - create new display tracker
                                                continuation_tool_calls_buffer.clear();
                                                continuation_last_tool_name = None;
                                                continuation_display = StreamingTokenDisplay::new(
                                                    cont_final.input_tokens,
                                                    cont_final.output_tokens,
                                                    cont_final.cache_read_tokens,
                                                    cont_final.cache_creation_tokens,
                                                );
                                                continue;
                                            }
                                            
                                            // Normal completion - add text to history and exit
                                            handle_final_response(&continuation_text, &mut session.messages)?;
                                            
                                            // CMPCT-001: Update token tracker with billing accumulation on normal completion
                                            let cont_usage = ApiTokenUsage::new(
                                                cont_final.input_tokens,
                                                cont_final.cache_read_tokens,
                                                cont_final.cache_creation_tokens,
                                                0,
                                            );
                                            session.token_tracker.update_from_usage(&cont_usage, cont_final.output_tokens);
                                            
                                            break;
                                        }
                                        Some(Err(e)) => {
                                            // CMPCT-002: Check if this is a compaction cancellation using helper
                                            if is_compaction_cancelled(&e) {
                                                // CMPCT-002: Handle compaction gracefully during Gemini continuation
                                                // Instead of returning an error, we:
                                                // 1. Save partial text to session history
                                                // 2. Update token tracker with cumulative billing
                                                // 3. Set compaction_needed flag
                                                // 4. Break out to let post-loop compaction logic handle it
                                                info!(
                                                    "Compaction triggered during Gemini continuation - handling gracefully"
                                                );
                                                
                                                // Save any partial text accumulated during continuation
                                                if !continuation_text.is_empty() {
                                                    handle_final_response(&continuation_text, &mut session.messages)?;
                                                    info!("Saved {} chars of partial continuation text", continuation_text.len());
                                                }
                                                
                                                // Update token tracker with current display values
                                                let cont_err_final = continuation_display.current();
                                                let cont_err_usage = ApiTokenUsage::new(
                                                    cont_err_final.input_tokens,
                                                    cont_err_final.cache_read_tokens,
                                                    cont_err_final.cache_creation_tokens,
                                                    0,
                                                );
                                                session.token_tracker.update_from_usage(&cont_err_usage, cont_err_final.output_tokens);
                                                
                                                // Set compaction_needed flag so post-loop logic handles it
                                                signal_compaction_needed(&token_state);
                                                
                                                output.emit_status("\n[Context limit reached during continuation, compacting...]");
                                                
                                                // Clear tool progress callback before breaking
                                                set_tool_progress_callback(None);
                                                
                                                // Break out of continuation loop - outer code will handle compaction
                                                // Note: We break from the continuation loop but NOT from the main stream loop
                                                // The main loop's post-processing will detect compaction_needed and handle it
                                                break;
                                            }
                                            
                                            // Non-compaction error - return error as before
                                            set_tool_progress_callback(None);
                                            output.emit_error(&e.to_string());
                                            return Err(anyhow::anyhow!("Gemini continuation error: {e}"));
                                        }
                                        None => {
                                            // Stream ended unexpectedly - update token tracker before exiting
                                            if !continuation_text.is_empty() {
                                                handle_final_response(&continuation_text, &mut session.messages)?;
                                            }
                                            
                                            // CMPCT-001: Update token tracker with current display values
                                            let cont_end_final = continuation_display.current();
                                            let cont_end_usage = ApiTokenUsage::new(
                                                cont_end_final.input_tokens,
                                                cont_end_final.cache_read_tokens,
                                                cont_end_final.cache_creation_tokens,
                                                0,
                                            );
                                            session.token_tracker.update_from_usage(&cont_end_usage, cont_end_final.output_tokens);
                                            
                                            break;
                                        }
                                        _ => {}
                                    }
                                    output.flush();
                                }
                                
                                // CMPCT-002: Check if we broke from continuation loop due to compaction
                                // If so, don't return - break from main stream loop to run compaction
                                let compaction_during_continuation = token_state
                                    .lock()
                                    .map(|state| state.compaction_needed)
                                    .unwrap_or(false);
                                
                                if compaction_during_continuation {
                                    // Don't return Ok() - break from main stream loop
                                    // The post-loop compaction logic will handle it
                                    break;
                                }
                                
                                // Normal continuation completion - clear callback and return
                                set_tool_progress_callback(None);
                                output.emit_done();
                                return Ok(());
                            }
                        }
                    }

                    // Normal case: add assistant text to history and finish
                    handle_final_response(&assistant_text, &mut session.messages)?;
                    output.emit_done();
                    break;
                }
                Some(Err(e)) => {
                    // CMPCT-002: Check if this error is due to compaction hook cancellation
                    // using the helper function for DRY compliance
                    let is_compaction_cancel = is_compaction_cancelled(&e);

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

                    // Check if this is a "prompt is too long" error from the API
                    let error_str = e.to_string();
                    let is_prompt_too_long = is_prompt_too_long_error(&error_str);

                    if is_prompt_too_long && !session.messages.is_empty() {
                        info!("Received 'prompt is too long' error, triggering recovery compaction");
                        output.emit_status("\n[Context exceeded limit, triggering emergency compaction...]");

                        // Pop the last user message we added at the start of this function
                        if let Some(last_msg) = session.messages.last() {
                            if matches!(last_msg, rig::message::Message::User { .. }) {
                                session.messages.pop();
                                info!("Popped last user message from context");
                            }
                        }

                        // Set compaction_needed flag so the post-loop logic handles it
                        signal_compaction_needed(&token_state);

                        break;
                    }

                    // NAPI-008: Log error with full details (include in message for TypeScript layer)
                    error!(
                        "API error received from provider: {} (messages={}, provider={})",
                        error_str,
                        session.messages.len(),
                        session.current_provider_name()
                    );
                    // Log the full error chain for debugging
                    let err_ref: &dyn StdError = e.as_ref();
                    let mut source = err_ref.source();
                    while let Some(cause) = source {
                        error!("Caused by: {}", cause);
                        source = cause.source();
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
                    // Stream ended
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

            // TOOL-011: Tool progress is emitted directly via progress_emitter callback
            // This ensures streaming happens in real-time during tool execution
        }
    }

    // TOOL-011: Clear the tool progress callback
    set_tool_progress_callback(None);

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
                output.emit_status("[Continuing with compacted context...]\n");

                // CMPCT-001: Reset output and cache metrics after compaction
                // NOTE: execute_compaction already sets session.token_tracker.input_tokens
                // to the correct new_total_tokens calculated from compacted messages.
                session.token_tracker.reset_after_compaction();

                // Re-add the user's original prompt to session.messages so the agent
                // can continue processing it with the compacted context
                session.messages.push(Message::User {
                    content: OneOrMany::one(UserContent::text(prompt)),
                });

                // Create fresh hook and token state for the retry
                // PROV-001: After compaction, input_tokens is the new estimated total
                // Cache values were reset to None above, so they're 0 here
                // This prevents double-counting in TokenState::total()
                let retry_token_state = Arc::new(Mutex::new(TokenState {
                    input_tokens: session.token_tracker.input_tokens,
                    cache_read_input_tokens: 0, // Reset after compaction
                    cache_creation_input_tokens: 0, // Reset after compaction
                    output_tokens: 0, // Fresh start after compaction
                    compaction_needed: false,
                }));
                let retry_hook = CompactionHook::new(Arc::clone(&retry_token_state), threshold);

                // Start new stream with compacted context
                let mut retry_stream = agent
                    .prompt_streaming_with_history_and_hook(
                        prompt,
                        &mut session.messages,
                        retry_hook,
                    )
                    .await;

                // Reset tracking for this retry
                let mut retry_assistant_text = String::new();
                let mut retry_tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
                let mut retry_last_tool_name: Option<String> = None;

                // STREAMING-DISPLAY: Create retry display tracker (fresh start after compaction)
                let mut retry_display = StreamingTokenDisplay::new(
                    session.token_tracker.input_tokens,
                    0, // Fresh start after compaction
                    0, // Cache reset after compaction
                    0, // Cache reset after compaction
                );

                // Process retry stream
                loop {
                    if is_interrupted.load(Acquire) {
                        let queued = if let Some(ref mut iq) = input_queue {
                            iq.dequeue_all()
                        } else {
                            vec![]
                        };
                        output.emit_interrupted(&queued);
                        if !retry_assistant_text.is_empty() {
                            handle_final_response(&retry_assistant_text, &mut session.messages)?;
                        }
                        output.emit_done();
                        break;
                    }

                    match retry_stream.next().await {
                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(text),
                        ))) => {
                            handle_text_chunk(&text.text, &mut retry_assistant_text, None, output)?;

                            // STREAMING-DISPLAY: Track chunk and emit if not throttled
                            if let Some(update) = retry_display.record_chunk(&text.text) {
                                output.emit_tokens(&update.into());
                            }
                        }
                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ToolCall(tool_call),
                        ))) => {
                            handle_tool_call(
                                &tool_call,
                                &mut session.messages,
                                &mut retry_assistant_text,
                                &mut retry_tool_calls_buffer,
                                &mut retry_last_tool_name,
                                output,
                            )?;
                        }
                        Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::ReasoningDelta { reasoning, .. },
                        ))) => {
                            // TOOL-010: Emit thinking/reasoning content from extended thinking
                            output.emit_thinking(&reasoning);
                            
                            // STREAMING-DISPLAY: Track thinking chunk
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

                            // PROV-001: Don't emit token updates after tool results
                        }
                        Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                            // STREAMING-DISPLAY: Update from usage event
                            if usage.output_tokens == 0 {
                                retry_display.start_new_segment(&usage);
                            } else if let Some(update) = retry_display.update_from_usage(&usage) {
                                output.emit_tokens(&update.into());
                                // CTX-004: Context fill uses CURRENT API values
                                let fill_usage = ApiTokenUsage::new(
                                    update.input_tokens,
                                    update.cache_read_tokens,
                                    update.cache_creation_tokens,
                                    usage.output_tokens,
                                );
                                emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);
                            }
                        }
                        Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                            // Get usage from FinalResponse
                            let usage = final_resp.usage();

                            // STREAMING-DISPLAY: Update from final response if needed
                            let retry_final = if !retry_display.has_authoritative_output() && usage.input_tokens > 0 {
                                trace!(
                                    "OpenAI-compatible provider (retry): extracted tokens from FinalResponse - input={}, output={}, cache_read={:?}",
                                    usage.input_tokens, usage.output_tokens, usage.cache_read_input_tokens
                                );
                                retry_display.update_from_final_response(&usage)
                            } else {
                                retry_display.current()
                            };

                            // Emit final token update
                            output.emit_tokens(&retry_final.into());
                            let fill_usage = ApiTokenUsage::new(
                                retry_final.input_tokens,
                                retry_final.cache_read_tokens,
                                retry_final.cache_creation_tokens,
                                usage.output_tokens,
                            );
                            emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);

                            // TUI-031: Update session state after retry completes
                            if !is_interrupted.load(Acquire) {
                                let retry_usage = ApiTokenUsage::new(
                                    retry_final.input_tokens,
                                    retry_final.cache_read_tokens,
                                    retry_final.cache_creation_tokens,
                                    0,
                                );
                                session.token_tracker.update_from_usage(&retry_usage, retry_final.output_tokens);
                            }

                            handle_final_response(&retry_assistant_text, &mut session.messages)?;
                            output.emit_done();
                            break;
                        }
                        Some(Err(e)) => {
                            output.emit_error(&e.to_string());
                            return Err(anyhow::anyhow!("Retry error after compaction: {e}"));
                        }
                        None => {
                            if !retry_assistant_text.is_empty() {
                                handle_final_response(&retry_assistant_text, &mut session.messages)?;
                            }
                            output.emit_done();
                            break;
                        }
                        _ => {}
                    }
                    output.flush();
                }

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

    // CMPCT-001: Update session token tracker with BOTH current context AND cumulative billing
    // Uses the consolidated update_from_usage method to reduce code duplication
    if !is_interrupted.load(Acquire) {
        let final_display = streaming_display.current();
        let final_usage = ApiTokenUsage::new(
            final_display.input_tokens,
            final_display.cache_read_tokens,
            final_display.cache_creation_tokens,
            0,
        );
        tracing::debug!(
            "CMPCT-001: Before update: cumulative_billed_input={}, final_display.input_tokens={}",
            session.token_tracker.cumulative_billed_input,
            final_display.input_tokens
        );
        session.token_tracker.update_from_usage(&final_usage, final_display.output_tokens);
        tracing::debug!(
            "CMPCT-001: After update: cumulative_billed_input={}, cumulative_billed_output={}",
            session.token_tracker.cumulative_billed_input,
            session.token_tracker.cumulative_billed_output
        );
    }

    Ok(())
}
