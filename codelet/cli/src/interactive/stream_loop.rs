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

use super::output::{ContextFillInfo, StreamOutput, TokenInfo};
use super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};
use crate::compaction_threshold::calculate_usable_context;
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
use std::error::Error as StdError;
use std::sync::atomic::AtomicBool;
// Use Acquire/Release ordering for proper cross-thread synchronization
// - Acquire: Ensures subsequent reads see all writes before the Release store
// - Release: Ensures all writes before the store are visible to Acquire loads
use std::sync::atomic::Ordering::{Acquire, Release};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::interval;
use tracing::error;

/// TUI-031: Tokens per second tracker with time-window and EMA smoothing
/// All tok/s calculation is done in Rust for single source of truth
struct TokPerSecTracker {
    /// Samples of (timestamp, cumulative_tokens) for time-window calculation
    samples: Vec<(Instant, u64)>,
    /// Cumulative tokens generated in this turn
    cumulative_tokens: u64,
    /// EMA-smoothed rate for stable display
    smoothed_rate: Option<f64>,
    /// Last time we emitted a tok/s update (for throttling)
    last_emit_time: Option<Instant>,
}

impl TokPerSecTracker {
    /// Time window for rate calculation (3 seconds)
    const TIME_WINDOW: Duration = Duration::from_secs(3);
    /// EMA alpha (0.3 = 30% new value, 70% old value)
    const EMA_ALPHA: f64 = 0.3;
    /// Minimum time between display updates (500ms)
    const DISPLAY_THROTTLE: Duration = Duration::from_millis(500);
    /// Minimum time span for stable rate calculation (100ms)
    const MIN_TIME_SPAN: Duration = Duration::from_millis(100);

    fn new() -> Self {
        Self {
            samples: Vec::new(),
            cumulative_tokens: 0,
            smoothed_rate: None,
            last_emit_time: None,
        }
    }

    /// Record a text chunk and calculate the smoothed tok/s rate
    /// Returns Some(rate) if display should be updated, None if throttled
    fn record_chunk(&mut self, text: &str) -> Option<f64> {
        let now = Instant::now();

        // Estimate tokens (~4 chars per token, rounded up)
        let chunk_tokens = (text.len() + 3) / 4;
        self.cumulative_tokens += chunk_tokens as u64;

        // Add sample
        self.samples.push((now, self.cumulative_tokens));

        // Remove samples older than TIME_WINDOW
        let cutoff = now - Self::TIME_WINDOW;
        self.samples.retain(|(ts, _)| *ts >= cutoff);

        // Need at least 2 samples for rate calculation
        if self.samples.len() < 2 {
            return None;
        }

        let oldest = self.samples.first().unwrap();
        let newest = self.samples.last().unwrap();
        let token_delta = newest.1 - oldest.1;
        let time_delta = newest.0.duration_since(oldest.0);

        // Need at least MIN_TIME_SPAN for stable rate
        if time_delta < Self::MIN_TIME_SPAN {
            return None;
        }

        let raw_rate = token_delta as f64 / time_delta.as_secs_f64();

        // Apply EMA smoothing
        self.smoothed_rate = Some(match self.smoothed_rate {
            Some(prev) => Self::EMA_ALPHA * raw_rate + (1.0 - Self::EMA_ALPHA) * prev,
            None => raw_rate,
        });

        // Check throttling
        let should_emit = match self.last_emit_time {
            Some(last) => now.duration_since(last) >= Self::DISPLAY_THROTTLE,
            None => true,
        };

        if should_emit {
            self.last_emit_time = Some(now);
            self.smoothed_rate
        } else {
            None
        }
    }

    /// Get current smoothed rate without recording (for final emit)
    fn current_rate(&self) -> Option<f64> {
        self.smoothed_rate
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

    // CRITICAL: Add user prompt to message history for persistence (CLI-008)
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(prompt)),
    });

    // HOOK-BASED COMPACTION (CTX-002: Optimized compaction trigger)
    let context_window = session.provider_manager().context_window() as u64;
    let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
    // CTX-002: Use usable_context (context_window - output_reservation) instead of 90% threshold
    let threshold = calculate_usable_context(context_window, max_output_tokens);

    // TUI-033: Helper to emit context fill percentage after token updates
    // CTX-002: Uses simple sum (input + cache_read + output) for token calculation
    let emit_context_fill = |output: &O,
                             input_tokens: u64,
                             cache_read_tokens: u64,
                             output_tokens: u64,
                             threshold: u64,
                             context_window: u64| {
        // CTX-002: Simple sum of all token types
        let total_tokens = input_tokens + cache_read_tokens + output_tokens;
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

    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: session.token_tracker.input_tokens,
        cache_read_input_tokens: session.token_tracker.cache_read_input_tokens.unwrap_or(0),
        cache_creation_input_tokens: session
            .token_tracker
            .cache_creation_input_tokens
            .unwrap_or(0),
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
    let prev_cache_creation = session.token_tracker.cache_creation_input_tokens.unwrap_or(0);

    // CTX-003: Track CURRENT CONTEXT tokens (overwritten with each API call, not accumulated)
    // Anthropic's input_tokens represents TOTAL context per call (absolute, not incremental)
    let mut current_api_input: u64 = 0;
    let mut current_api_output: u64 = 0;
    let mut turn_cache_read: u64 = 0;
    let mut turn_cache_creation: u64 = 0;

    // TUI-031: Track CUMULATIVE output tokens across all API calls within this turn
    // Initialize to previous output so display doesn't flash to 0 at start of new turn
    // This value grows throughout the session and never decreases
    let mut turn_cumulative_output: u64 = prev_output_tokens;

    // TUI-031: Tokens per second tracker (time-window + EMA smoothing)
    let mut tok_per_sec_tracker = TokPerSecTracker::new();

    // Emit initial token state at start of prompt so display shows current session state
    // (prevents flash to 0 when starting new prompt)
    output.emit_tokens(&TokenInfo {
        input_tokens: prev_input_tokens,
        output_tokens: prev_output_tokens,
        cache_read_input_tokens: Some(prev_cache_read),
        cache_creation_input_tokens: Some(prev_cache_creation),
        tokens_per_second: None,
    });
    emit_context_fill(
        output,
        prev_input_tokens,
        prev_cache_read,
        prev_output_tokens,
        threshold,
        context_window,
    );

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

                    // TUI-031: Track tok/s and emit update if not throttled
                    if let Some(rate) = tok_per_sec_tracker.record_chunk(&text.text) {
                        let display_output = turn_cumulative_output + current_api_output;
                        output.emit_tokens(&TokenInfo {
                            input_tokens: current_api_input,
                            output_tokens: display_output,
                            cache_read_input_tokens: Some(turn_cache_read),
                            cache_creation_input_tokens: Some(turn_cache_creation),
                            tokens_per_second: Some(rate),
                        });
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

                    // TUI-031: Emit CUMULATIVE output tokens after tool result
                    let display_output = turn_cumulative_output + current_api_output;
                    output.emit_tokens(&TokenInfo {
                        input_tokens: current_api_input,
                        output_tokens: display_output,
                        cache_read_input_tokens: Some(turn_cache_read),
                        cache_creation_input_tokens: Some(turn_cache_creation),
                        tokens_per_second: tok_per_sec_tracker.current_rate(),
                    });
                    // TUI-033: Emit context fill percentage
                    emit_context_fill(
                        output,
                        current_api_input,
                        turn_cache_read,
                        display_output,
                        threshold,
                        context_window,
                    );
                }
                Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                    // Usage events come from:
                    // 1. MessageStart (input tokens, output=0) - marks start of new API call
                    // 2. MessageDelta (input + output tokens) - streaming updates

                    if usage.output_tokens == 0 {
                        // MessageStart - new API call starting
                        // Reset current API output but keep cumulative for display
                        current_api_input = usage.input_tokens;
                        current_api_output = 0;
                    } else {
                        // MessageDelta - update current API call's output tokens
                        current_api_output = usage.output_tokens;
                    }

                    turn_cache_read = usage.cache_read_input_tokens.unwrap_or(turn_cache_read);
                    turn_cache_creation = usage
                        .cache_creation_input_tokens
                        .unwrap_or(turn_cache_creation);

                    // TUI-031: Emit CUMULATIVE output tokens for display (never decreases within turn)
                    // Input tokens show current context size, output tokens show cumulative generation
                    let display_output = turn_cumulative_output + current_api_output;
                    output.emit_tokens(&TokenInfo {
                        input_tokens: current_api_input,
                        output_tokens: display_output,
                        cache_read_input_tokens: Some(turn_cache_read),
                        cache_creation_input_tokens: Some(turn_cache_creation),
                        tokens_per_second: tok_per_sec_tracker.current_rate(),
                    });
                    // TUI-033: Emit context fill percentage
                    emit_context_fill(
                        output,
                        current_api_input,
                        turn_cache_read,
                        display_output,
                        threshold,
                        context_window,
                    );
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                    // Get final usage directly from FinalResponse (most accurate source)
                    let usage = final_resp.usage();

                    // For non-Anthropic providers that don't emit Usage, use FinalResponse values
                    if current_api_input == 0 {
                        current_api_input = usage.input_tokens;
                    }
                    current_api_output = usage.output_tokens;

                    // TUI-031: Accumulate output tokens for this turn
                    // This happens when FinalResponse is received (end of an API call)
                    turn_cumulative_output += current_api_output;

                    // Update cache tokens from FinalResponse
                    turn_cache_read = usage.cache_read_input_tokens.unwrap_or(turn_cache_read);
                    turn_cache_creation = usage
                        .cache_creation_input_tokens
                        .unwrap_or(turn_cache_creation);

                    // TUI-031: Emit CUMULATIVE output tokens for display
                    // Input tokens show current context size, output tokens show cumulative generation
                    // Clear tok/s on final response (streaming is done)
                    output.emit_tokens(&TokenInfo {
                        input_tokens: current_api_input,
                        output_tokens: turn_cumulative_output,
                        cache_read_input_tokens: Some(turn_cache_read),
                        cache_creation_input_tokens: Some(turn_cache_creation),
                        tokens_per_second: None, // Streaming done, hide tok/s
                    });
                    // TUI-033: Emit context fill percentage
                    emit_context_fill(
                        output,
                        current_api_input,
                        turn_cache_read,
                        turn_cumulative_output,
                        threshold,
                        context_window,
                    );

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
                output.emit_status("[Continuing with compacted context...]\n");

                // NOTE: execute_compaction already sets session.token_tracker.input_tokens
                // to the correct new_total_tokens calculated from compacted messages.
                // We only reset output_tokens and cache metrics here.
                session.token_tracker.output_tokens = 0;
                session.token_tracker.cache_read_input_tokens = None;
                session.token_tracker.cache_creation_input_tokens = None;

                // Re-add the user's original prompt to session.messages so the agent
                // can continue processing it with the compacted context
                session.messages.push(Message::User {
                    content: OneOrMany::one(UserContent::text(prompt)),
                });

                // Create fresh hook and token state for the retry
                let retry_token_state = Arc::new(Mutex::new(TokenState {
                    input_tokens: session.token_tracker.input_tokens,
                    cache_read_input_tokens: session
                        .token_tracker
                        .cache_read_input_tokens
                        .unwrap_or(0),
                    cache_creation_input_tokens: session
                        .token_tracker
                        .cache_creation_input_tokens
                        .unwrap_or(0),
                    // CTX-002: Include output tokens for accurate total calculation
                    output_tokens: session.token_tracker.output_tokens,
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

                // TUI-031: Track tokens for retry loop (same pattern as main loop)
                let mut retry_api_input: u64 = 0;
                let mut retry_api_output: u64 = 0;
                let mut retry_cumulative_output: u64 = 0;
                let mut retry_cache_read: u64 = 0;
                let mut retry_cache_creation: u64 = 0;
                let mut retry_tok_tracker = TokPerSecTracker::new();

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

                            // TUI-031: Track tok/s and emit update if not throttled
                            if let Some(rate) = retry_tok_tracker.record_chunk(&text.text) {
                                let display_output = retry_cumulative_output + retry_api_output;
                                output.emit_tokens(&TokenInfo {
                                    input_tokens: retry_api_input,
                                    output_tokens: display_output,
                                    cache_read_input_tokens: Some(retry_cache_read),
                                    cache_creation_input_tokens: Some(retry_cache_creation),
                                    tokens_per_second: Some(rate),
                                });
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

                            // TUI-031: Emit CUMULATIVE output tokens after tool result
                            let display_output = retry_cumulative_output + retry_api_output;
                            output.emit_tokens(&TokenInfo {
                                input_tokens: retry_api_input,
                                output_tokens: display_output,
                                cache_read_input_tokens: Some(retry_cache_read),
                                cache_creation_input_tokens: Some(retry_cache_creation),
                                tokens_per_second: retry_tok_tracker.current_rate(),
                            });
                            emit_context_fill(
                                output,
                                retry_api_input,
                                retry_cache_read,
                                display_output,
                                threshold,
                                context_window,
                            );
                        }
                        Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                            // TUI-031: Track tokens with cumulative output (same pattern as main loop)
                            if usage.output_tokens == 0 {
                                retry_api_input = usage.input_tokens;
                                retry_api_output = 0;
                            } else {
                                retry_api_output = usage.output_tokens;
                            }
                            retry_cache_read = usage.cache_read_input_tokens.unwrap_or(retry_cache_read);
                            retry_cache_creation = usage.cache_creation_input_tokens.unwrap_or(retry_cache_creation);

                            let display_output = retry_cumulative_output + retry_api_output;
                            output.emit_tokens(&TokenInfo {
                                input_tokens: retry_api_input,
                                output_tokens: display_output,
                                cache_read_input_tokens: Some(retry_cache_read),
                                cache_creation_input_tokens: Some(retry_cache_creation),
                                tokens_per_second: retry_tok_tracker.current_rate(),
                            });
                            // TUI-033: Emit context fill percentage
                            emit_context_fill(
                                output,
                                retry_api_input,
                                retry_cache_read,
                                display_output,
                                threshold,
                                context_window,
                            );
                        }
                        Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                            // TUI-031: Accumulate output tokens from FinalResponse
                            let usage = final_resp.usage();
                            if retry_api_input == 0 {
                                retry_api_input = usage.input_tokens;
                            }
                            retry_api_output = usage.output_tokens;
                            retry_cumulative_output += retry_api_output;
                            retry_cache_read = usage.cache_read_input_tokens.unwrap_or(retry_cache_read);
                            retry_cache_creation = usage.cache_creation_input_tokens.unwrap_or(retry_cache_creation);

                            output.emit_tokens(&TokenInfo {
                                input_tokens: retry_api_input,
                                output_tokens: retry_cumulative_output,
                                cache_read_input_tokens: Some(retry_cache_read),
                                cache_creation_input_tokens: Some(retry_cache_creation),
                                tokens_per_second: None, // Streaming done, hide tok/s
                            });
                            emit_context_fill(
                                output,
                                retry_api_input,
                                retry_cache_read,
                                retry_cumulative_output,
                                threshold,
                                context_window,
                            );

                            handle_final_response(&retry_assistant_text, &mut session.messages)?;
                            output.emit_done();
                            break;
                        }
                        Some(Err(e)) => {
                            output.emit_error(&e.to_string());
                            return Err(anyhow::anyhow!("Agent error after compaction: {e}"));
                        }
                        None => {
                            if !retry_assistant_text.is_empty() {
                                handle_final_response(
                                    &retry_assistant_text,
                                    &mut session.messages,
                                )?;
                            }
                            output.emit_done();
                            break;
                        }
                        _ => {}
                    }
                    output.flush();
                }

                // TUI-031: Update session state after retry completes (mirrors end-of-function logic)
                if !is_interrupted.load(Acquire) {
                    session.token_tracker.input_tokens = retry_api_input;
                    session.token_tracker.output_tokens = retry_cumulative_output;
                    session.token_tracker.cumulative_billed_input += retry_api_input;
                    session.token_tracker.cumulative_billed_output += retry_api_output;
                    session.token_tracker.cache_read_input_tokens = Some(retry_cache_read);
                    session.token_tracker.cache_creation_input_tokens = Some(retry_cache_creation);
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

    // CTX-003: Update session token tracker with BOTH current context AND cumulative billing
    // Anthropic's input_tokens represents the TOTAL context size (absolute, not incremental).
    //
    // Two distinct metrics:
    // - input_tokens: CURRENT context size (for display and threshold checks)
    // - output_tokens: CUMULATIVE output tokens across all API calls (TUI-031)
    // - cumulative_billed_input/output: Sum of all API calls (for billing analytics)
    //
    // This matches CompactionHook behavior which correctly OVERWRITES (not accumulates) for context.
    if !is_interrupted.load(Acquire) {
        // Overwrite current context values (for display and threshold checks)
        session.token_tracker.input_tokens = current_api_input;
        // TUI-031: Save CUMULATIVE output tokens so next turn continues from correct value
        session.token_tracker.output_tokens = turn_cumulative_output;
        // Accumulate for billing analytics (sum of all API calls in session)
        session.token_tracker.cumulative_billed_input += current_api_input;
        session.token_tracker.cumulative_billed_output += current_api_output;
        // Cache tokens are per-request, not cumulative (use latest values)
        session.token_tracker.cache_read_input_tokens = Some(turn_cache_read);
        session.token_tracker.cache_creation_input_tokens = Some(turn_cache_creation);
    }

    Ok(())
}
