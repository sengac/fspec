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
use codelet_common::token_estimator::count_tokens;
use codelet_core::{ApiTokenUsage, CompactionHook, RigAgent, TokenState, ensure_thought_signatures, GeminiTurnCompletionFacade, TurnCompletionFacade, ContinuationStrategy};
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
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::time::interval;
use tracing::{error, info};

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

/// TUI-031: Tokens per second tracker with time-window and EMA smoothing
/// All tok/s calculation is done in Rust for single source of truth
struct TokPerSecTracker {
    /// Samples of (timestamp, cumulative_tokens) for time-window calculation
    samples: Vec<(Instant, u64)>,
    /// Cumulative tokens generated in this turn (tiktoken estimate)
    cumulative_tokens: u64,
    /// EMA-smoothed rate for stable display
    smoothed_rate: Option<f64>,
    /// Last time we emitted a tok/s update (for throttling)
    last_emit_time: Option<Instant>,
}

impl TokPerSecTracker {
    /// Time window for rate calculation (1 second) - PROV-002: Reduced for faster response
    /// When chunk flow changes, we want the rate to reflect it immediately.
    /// A shorter window means "stale" tokens exit the window faster.
    const TIME_WINDOW: Duration = Duration::from_secs(1);
    /// Minimum time between display updates (200ms) - PROV-002: Faster UI feedback
    const DISPLAY_THROTTLE: Duration = Duration::from_millis(200);
    /// Minimum time span for stable rate calculation (50ms)
    const MIN_TIME_SPAN: Duration = Duration::from_millis(50);

    fn new() -> Self {
        // Create new TokPerSecTracker
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

        // Use tiktoken-rs for accurate token counting
        let chunk_tokens = count_tokens(text);
        self.cumulative_tokens += chunk_tokens as u64;

        // Add sample
        self.samples.push((now, self.cumulative_tokens));

        // Remove samples older than TIME_WINDOW
        let cutoff = now - Self::TIME_WINDOW;
        self.samples.retain(|(ts, _)| *ts >= cutoff);

        // Need at least 2 samples for rate calculation
        let (Some(oldest), Some(newest)) = (self.samples.first(), self.samples.last()) else {
            return None;
        };
        if self.samples.len() < 2 {
            return None;
        }
        let token_delta = newest.1 - oldest.1;
        let time_delta = newest.0.duration_since(oldest.0);

        // Need at least MIN_TIME_SPAN for stable rate
        if time_delta < Self::MIN_TIME_SPAN {
            return None;
        }

        let raw_rate = token_delta as f64 / time_delta.as_secs_f64();

        // PROV-002: Adaptive EMA that responds quickly to rate changes
        // The core problem: EMA creates inertia, so tok/s stays "stuck" when chunk flow changes
        // Solution: Use adaptive alpha based on how much the rate is changing
        // - If rate is stable (small delta): use low alpha (0.1) for smoothing
        // - If rate is changing rapidly (large delta): use high alpha (0.9) to respond quickly
        let adaptive_alpha = match self.smoothed_rate {
            Some(prev) => {
                let rate_delta = (raw_rate - prev).abs();
                let relative_change = rate_delta / (prev.max(1.0)); // Avoid division by zero

                // Adaptive alpha: larger change = less smoothing (more responsiveness)
                // Clamp alpha between 0.1 (max smoothing) and 0.9 (min smoothing)
                let alpha = (relative_change * 2.0).clamp(0.1, 0.9);
                alpha
            }
            None => 1.0, // First reading: 100% weight to raw_rate
        };

        let (_prev_rate, new_rate) = match self.smoothed_rate {
            Some(prev) => (Some(prev), adaptive_alpha * raw_rate + (1.0 - adaptive_alpha) * prev),
            None => (None, raw_rate),
        };
        self.smoothed_rate = Some(new_rate);

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

    /// Calculate display tokens for UI during streaming
    /// 
    /// PROV-002: Handles the provider difference in token reporting:
    /// - Anthropic/Gemini: emit Usage events during streaming, so turn_usage has real values
    /// - OpenAI-compatible (Z.AI, OpenAI): only report usage in FinalResponse
    /// 
    /// For OpenAI-compatible providers, we use:
    /// - prev_input_tokens: the session's last known input (reasonable approximation)
    /// - cumulative_tokens: tiktoken estimate of generated output
    /// 
    /// IMPORTANT: Always use the MAX of tiktoken estimate and actual API value to prevent
    /// the display from "going backwards" when actual < estimate.
    /// 
    /// Returns (display_input, display_output) tuple for TokenInfo creation.
    fn calculate_display_tokens(
        &self,
        turn_usage: &ApiTokenUsage,
        turn_cumulative_output: u64,
        prev_input_tokens: u64,
    ) -> (u64, u64) {
        // Input: use actual if available, otherwise fall back to previous session value
        let display_input = if turn_usage.input_tokens > 0 {
            turn_usage.input_tokens
        } else {
            prev_input_tokens
        };

        // Output: use MAX of tiktoken estimate and actual API value
        // This prevents the display from "going backwards" when:
        // 1. Tiktoken estimate during streaming is higher than actual API value
        // 2. A new API segment starts (tool call) and output resets
        let tiktoken_output = turn_cumulative_output + self.cumulative_tokens;
        let actual_output = turn_cumulative_output + turn_usage.output_tokens;
        let display_output = tiktoken_output.max(actual_output);

        (display_input, display_output)
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
        info!(
            "[CTX-005] Pre-prompt compaction triggered: estimated {} > threshold {}",
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

                // Reset output and cache metrics after compaction
                session.token_tracker.output_tokens = 0;
                session.token_tracker.cache_read_input_tokens = None;
                session.token_tracker.cache_creation_input_tokens = None;
            }
            Err(e) => {
                // Log but continue - the API might still work, or will fail with clear error
                error!("[CTX-005] Pre-prompt compaction failed: {}", e);
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
        set_tool_progress_callback(Some(Arc::new(move |chunk: &str| {
            emitter.emit_tool_progress("", "bash", chunk);
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

    // PROV-001: Track current turn's token usage with ApiTokenUsage for DRY calculations
    // - input_tokens: raw (non-cached) input from current API call
    // - cache_read/creation: initialized from previous session to avoid display reset
    // - output_tokens: current API call's output (not cumulative)
    let mut turn_usage = ApiTokenUsage::new(0, prev_cache_read, prev_cache_creation, 0);

    // TUI-031: Track CUMULATIVE output tokens across all API calls within this turn
    // Initialize to previous output so display doesn't flash to 0 at start of new turn
    // This value grows throughout the session and never decreases
    let mut turn_cumulative_output: u64 = prev_output_tokens;

    // TUI-031: Tokens per second tracker (time-window + EMA smoothing)
    let mut tok_per_sec_tracker = TokPerSecTracker::new();

    // Emit initial token state at start of prompt so display shows current session state
    // (prevents flash to 0 when starting new prompt)
    // PROV-001: prev_input_tokens ALREADY contains total context (stored that way)
    info!(
        "[PROV-001] Initial emit: prev_input_tokens={}, prev_output_tokens={}, cache_read={}, cache_creation={}",
        prev_input_tokens, prev_output_tokens, prev_cache_read, prev_cache_creation
    );
    output.emit_tokens(&TokenInfo {
        input_tokens: prev_input_tokens, // Already total (includes cache)
        output_tokens: prev_output_tokens,
        cache_read_input_tokens: Some(prev_cache_read),
        cache_creation_input_tokens: Some(prev_cache_creation),
        tokens_per_second: None,
    });
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

                    // TUI-031: Track tok/s and emit update if not throttled
                    if let Some(rate) = tok_per_sec_tracker.record_chunk(&text.text) {
                        // PROV-002: Use unified method for display token calculation
                        let (display_input, display_output) = tok_per_sec_tracker
                            .calculate_display_tokens(&turn_usage, turn_cumulative_output, prev_input_tokens);
                        let display_usage = ApiTokenUsage::new(
                            display_input,
                            turn_usage.cache_read_input_tokens,
                            turn_usage.cache_creation_input_tokens,
                            display_output,
                        );
                        output.emit_tokens(&TokenInfo::from_usage(display_usage, Some(rate)));
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

                    // PROV-002: Track tok/s for thinking content too (both Anthropic and Z.AI stream thinking)
                    if let Some(rate) = tok_per_sec_tracker.record_chunk(&reasoning) {
                        // PROV-002: Use unified method for display token calculation
                        let (display_input, display_output) = tok_per_sec_tracker
                            .calculate_display_tokens(&turn_usage, turn_cumulative_output, prev_input_tokens);
                        let display_usage = ApiTokenUsage::new(
                            display_input,
                            turn_usage.cache_read_input_tokens,
                            turn_usage.cache_creation_input_tokens,
                            display_output,
                        );
                        output.emit_tokens(&TokenInfo::from_usage(display_usage, Some(rate)));
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

                    // Always update input_tokens when provided (needed for Gemini which sends
                    // input tokens with every usage event, not just at start)
                    if usage.input_tokens > 0 && turn_usage.input_tokens == 0 {
                        turn_usage.input_tokens = usage.input_tokens;
                    }

                    if usage.output_tokens == 0 {
                        // MessageStart - new API call starting (Anthropic pattern)
                        // PROV-001 FIX: Accumulate previous segment's output BEFORE resetting
                        // In multi-turn tool loops, FinalResponse only comes at the very end.
                        // Without this, turn_cumulative_output stays at 0 during tool calls.
                        turn_cumulative_output += turn_usage.output_tokens;

                        // Update internal tracking but DON'T emit to display
                        turn_usage.input_tokens = usage.input_tokens;
                        turn_usage.output_tokens = 0;
                        // Update cache tokens silently
                        turn_usage.update_cache(
                            usage.cache_read_input_tokens,
                            usage.cache_creation_input_tokens,
                        );
                    } else {
                        // MessageDelta - update current API call's output tokens
                        turn_usage.output_tokens = usage.output_tokens;

                        // Update cache tokens
                        turn_usage.update_cache(
                            usage.cache_read_input_tokens,
                            usage.cache_creation_input_tokens,
                        );

                        // PROV-002 FIX: Use unified method for display token calculation
                        // This ensures consistency between text chunk emissions and Usage event emissions.
                        // Previously, text chunks used tok_per_sec_tracker.cumulative_tokens (tiktoken estimate)
                        // while Usage events used turn_usage.output_tokens (actual API value).
                        // This caused tokens to "go backwards" when a Usage event arrived with a smaller
                        // actual value than the tiktoken estimate.
                        let (display_input, display_output) = tok_per_sec_tracker
                            .calculate_display_tokens(&turn_usage, turn_cumulative_output, prev_input_tokens);
                        let display_usage = ApiTokenUsage::new(
                            display_input,
                            turn_usage.cache_read_input_tokens,
                            turn_usage.cache_creation_input_tokens,
                            display_output,
                        );
                        output.emit_tokens(&TokenInfo::from_usage(
                            display_usage,
                            tok_per_sec_tracker.current_rate(),
                        ));
                        // CTX-004: Context fill uses CURRENT API output only (not cumulative)
                        emit_context_fill_from_usage(output, &turn_usage, threshold, context_window);
                    }
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                    // Get usage from FinalResponse
                    let usage = final_resp.usage();

                    // PROV-002: OpenAI-compatible providers (including Z.AI) don't emit Usage events
                    // during streaming - they only return usage in FinalResponse. For these providers,
                    // turn_usage will still be at 0 when we reach here, so we need to extract tokens
                    // from the FinalResponse usage directly.
                    //
                    // Detection: If turn_usage.input_tokens is 0 but FinalResponse has usage, this
                    // is an OpenAI-compatible provider that doesn't emit streaming Usage events.
                    if turn_usage.input_tokens == 0 && usage.input_tokens > 0 {
                        // OpenAI-compatible path: extract tokens from FinalResponse
                        turn_usage.input_tokens = usage.input_tokens;
                        turn_usage.output_tokens = usage.output_tokens;
                        // PROV-002: Extract cached tokens from FinalResponse (Z.AI/OpenAI)
                        if let Some(cached) = usage.cache_read_input_tokens {
                            turn_usage.cache_read_input_tokens = cached;
                        }
                        // PROV-002 FIX: ADD to cumulative, don't replace!
                        // turn_cumulative_output already has prev_output_tokens from initialization.
                        // We need to add this turn's output to it, not replace it.
                        turn_cumulative_output += usage.output_tokens;
                        
                        info!(
                            "[PROV-002] OpenAI-compatible provider: extracted tokens from FinalResponse - input={}, output={}, cache_read={:?}",
                            usage.input_tokens, usage.output_tokens, usage.cache_read_input_tokens
                        );
                    } else {
                        // PROV-001 FIX: FinalResponse.usage() contains AGGREGATED values (sum of all
                        // segments in multi-turn tool loops). For DISPLAY, we want the LAST segment's
                        // values (current context size), which are already in turn_usage from Usage events.
                        //
                        // DON'T overwrite turn_usage.input_tokens or cache values with aggregated values!
                        // They would show 290k (sum of 5 segments) instead of ~60k (actual context).
                        //
                        // For output: add the last segment's output to cumulative total.
                        // The aggregated output in FinalResponse is for billing, not display.
                        turn_cumulative_output += turn_usage.output_tokens;
                    }

                    // TUI-031: Emit CUMULATIVE output tokens for display
                    // PROV-002: Use MAX of tiktoken estimate and actual to prevent going backwards
                    let tiktoken_output = turn_cumulative_output + tok_per_sec_tracker.cumulative_tokens;
                    let display_output = tiktoken_output.max(turn_cumulative_output);
                    let display_usage = ApiTokenUsage::new(
                        turn_usage.input_tokens,
                        turn_usage.cache_read_input_tokens,
                        turn_usage.cache_creation_input_tokens,
                        display_output,
                    );
                    output.emit_tokens(&TokenInfo::from_usage(display_usage, tok_per_sec_tracker.current_rate()));
                    // CTX-004: Context fill uses CURRENT API output only
                    emit_context_fill_from_usage(output, &turn_usage, threshold, context_window);

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
                                            "inputTokens": turn_usage.input_tokens,
                                            "outputTokens": turn_cumulative_output,
                                            "cacheReadInputTokens": turn_usage.cache_read_input_tokens,
                                            "cacheCreationInputTokens": turn_usage.cache_creation_input_tokens,
                                            "totalInputTokens": display_usage.total_input(),
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
                                        "inputTokens": turn_usage.input_tokens,
                                        "outputTokens": turn_cumulative_output,
                                        "cacheReadInputTokens": turn_usage.cache_read_input_tokens,
                                        "cacheCreationInputTokens": turn_usage.cache_creation_input_tokens,
                                        "totalInputTokens": display_usage.total_input(),
                                        "totalOutputTokens": turn_cumulative_output,
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
                                
                                // Update session token tracker with current values before recursion
                                session.token_tracker.input_tokens = turn_usage.total_input();
                                session.token_tracker.output_tokens = turn_cumulative_output;
                                session.token_tracker.cache_read_input_tokens = Some(turn_usage.cache_read_input_tokens);
                                session.token_tracker.cache_creation_input_tokens = Some(turn_usage.cache_creation_input_tokens);
                                
                                // Create a new hook and token state for the continuation
                                let continuation_token_state = Arc::new(Mutex::new(TokenState {
                                    input_tokens: session.token_tracker.input_tokens,
                                    cache_read_input_tokens: turn_usage.cache_read_input_tokens,
                                    cache_creation_input_tokens: turn_usage.cache_creation_input_tokens,
                                    output_tokens: turn_cumulative_output,
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
                                let mut continuation_usage = ApiTokenUsage::new(
                                    turn_usage.input_tokens,
                                    turn_usage.cache_read_input_tokens,
                                    turn_usage.cache_creation_input_tokens,
                                    0,
                                );
                                let mut continuation_cumulative_output = turn_cumulative_output;
                                
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
                                        
                                        // Update session token tracker before returning
                                        continuation_cumulative_output += continuation_usage.output_tokens;
                                        session.token_tracker.input_tokens = continuation_usage.total_input();
                                        session.token_tracker.output_tokens = continuation_cumulative_output;
                                        session.token_tracker.cumulative_billed_input += continuation_usage.input_tokens;
                                        session.token_tracker.cumulative_billed_output += continuation_usage.output_tokens;
                                        session.token_tracker.cache_read_input_tokens = Some(continuation_usage.cache_read_input_tokens);
                                        session.token_tracker.cache_creation_input_tokens = Some(continuation_usage.cache_creation_input_tokens);
                                        
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
                                            // PROV-002: Track tok/s for thinking content too (both Anthropic and Z.AI stream thinking)
                                            output.emit_thinking(&reasoning);
                                            
                                            // Note: continuation loop doesn't have its own tok_per_sec_tracker
                                            // We still emit the thinking content for display, just without tok/s
                                            // This is acceptable since continuation is for Gemini's empty-response workaround
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
                                            // Update token tracking (same pattern as main loop)
                                            if usage.input_tokens > 0 && continuation_usage.input_tokens == 0 {
                                                continuation_usage.input_tokens = usage.input_tokens;
                                            }
                                            if usage.output_tokens == 0 {
                                                continuation_cumulative_output += continuation_usage.output_tokens;
                                                continuation_usage.input_tokens = usage.input_tokens;
                                                continuation_usage.output_tokens = 0;
                                                continuation_usage.update_cache(
                                                    usage.cache_read_input_tokens,
                                                    usage.cache_creation_input_tokens,
                                                );
                                            } else {
                                                continuation_usage.output_tokens = usage.output_tokens;
                                                continuation_usage.update_cache(
                                                    usage.cache_read_input_tokens,
                                                    usage.cache_creation_input_tokens,
                                                );
                                            }
                                        }
                                        Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                                            // Get usage from FinalResponse
                                            let usage = final_resp.usage();

                                            // PROV-002: OpenAI-compatible providers only return usage in FinalResponse
                                            if continuation_usage.input_tokens == 0 && usage.input_tokens > 0 {
                                                continuation_usage.input_tokens = usage.input_tokens;
                                                continuation_usage.output_tokens = usage.output_tokens;
                                            }

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
                                                continuation_cumulative_output += continuation_usage.output_tokens;
                                                let nested_token_state = Arc::new(Mutex::new(TokenState {
                                                    input_tokens: continuation_usage.total_input(),
                                                    cache_read_input_tokens: continuation_usage.cache_read_input_tokens,
                                                    cache_creation_input_tokens: continuation_usage.cache_creation_input_tokens,
                                                    output_tokens: continuation_cumulative_output,
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
                                                
                                                // Reset for next iteration
                                                continuation_tool_calls_buffer.clear();
                                                continuation_last_tool_name = None;
                                                continuation_usage = ApiTokenUsage::new(
                                                    continuation_usage.input_tokens,
                                                    continuation_usage.cache_read_input_tokens,
                                                    continuation_usage.cache_creation_input_tokens,
                                                    0,
                                                );
                                                continue;
                                            }
                                            
                                            // Normal completion - add text to history and exit
                                            continuation_cumulative_output += continuation_usage.output_tokens;
                                            handle_final_response(&continuation_text, &mut session.messages)?;
                                            
                                            // Update session token tracker (including billing)
                                            session.token_tracker.input_tokens = continuation_usage.total_input();
                                            session.token_tracker.output_tokens = continuation_cumulative_output;
                                            session.token_tracker.cumulative_billed_input += continuation_usage.input_tokens;
                                            session.token_tracker.cumulative_billed_output += continuation_usage.output_tokens;
                                            session.token_tracker.cache_read_input_tokens = Some(continuation_usage.cache_read_input_tokens);
                                            session.token_tracker.cache_creation_input_tokens = Some(continuation_usage.cache_creation_input_tokens);
                                            
                                            break;
                                        }
                                        Some(Err(e)) => {
                                            // Check if this is a compaction cancellation
                                            let error_str = e.to_string();
                                            let is_compaction_cancel = error_str.contains("PromptCancelled");
                                            
                                            if is_compaction_cancel {
                                                // Compaction needed during continuation - this is complex to handle
                                                // For now, log and return error. Future: trigger compaction and retry.
                                                error!(
                                                    "Compaction triggered during Gemini continuation - not yet supported"
                                                );
                                            }
                                            
                                            // Clear tool progress callback before returning
                                            set_tool_progress_callback(None);
                                            output.emit_error(&e.to_string());
                                            return Err(anyhow::anyhow!("Gemini continuation error: {e}"));
                                        }
                                        None => {
                                            // Stream ended unexpectedly - update token tracker before exiting
                                            if !continuation_text.is_empty() {
                                                handle_final_response(&continuation_text, &mut session.messages)?;
                                            }
                                            
                                            // Update session token tracker even on unexpected end (including billing)
                                            continuation_cumulative_output += continuation_usage.output_tokens;
                                            session.token_tracker.input_tokens = continuation_usage.total_input();
                                            session.token_tracker.output_tokens = continuation_cumulative_output;
                                            session.token_tracker.cumulative_billed_input += continuation_usage.input_tokens;
                                            session.token_tracker.cumulative_billed_output += continuation_usage.output_tokens;
                                            session.token_tracker.cache_read_input_tokens = Some(continuation_usage.cache_read_input_tokens);
                                            session.token_tracker.cache_creation_input_tokens = Some(continuation_usage.cache_creation_input_tokens);
                                            
                                            break;
                                        }
                                        _ => {}
                                    }
                                    output.flush();
                                }
                                
                                // Clear tool progress callback before returning
                                set_tool_progress_callback(None);
                                
                                // Done with continuation
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

                    // Check if this is a "prompt is too long" error from the API
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
                        if let Ok(mut state) = token_state.lock() {
                            state.compaction_needed = true;
                        }

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

                // PROV-001: Track tokens for retry loop using ApiTokenUsage (DRY)
                let mut retry_usage = ApiTokenUsage::default();
                let mut retry_cumulative_output: u64 = 0;
                let mut retry_tok_tracker = TokPerSecTracker::new();
                // PROV-002: Capture prev input tokens for OpenAI-compatible fallback during streaming
                let retry_prev_input_tokens = session.token_tracker.input_tokens;

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
                                // PROV-002: Use unified method for display token calculation
                                let (display_input, display_output) = retry_tok_tracker
                                    .calculate_display_tokens(&retry_usage, retry_cumulative_output, retry_prev_input_tokens);
                                let display_usage = ApiTokenUsage::new(
                                    display_input,
                                    retry_usage.cache_read_input_tokens,
                                    retry_usage.cache_creation_input_tokens,
                                    display_output,
                                );
                                output.emit_tokens(&TokenInfo::from_usage(display_usage, Some(rate)));
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
                            
                            // PROV-002: Track tok/s for thinking content too (both Anthropic and Z.AI stream thinking)
                            if let Some(rate) = retry_tok_tracker.record_chunk(&reasoning) {
                                // PROV-002: Use unified method for display token calculation
                                let (display_input, display_output) = retry_tok_tracker
                                    .calculate_display_tokens(&retry_usage, retry_cumulative_output, retry_prev_input_tokens);
                                let display_usage = ApiTokenUsage::new(
                                    display_input,
                                    retry_usage.cache_read_input_tokens,
                                    retry_usage.cache_creation_input_tokens,
                                    display_output,
                                );
                                output.emit_tokens(&TokenInfo::from_usage(display_usage, Some(rate)));
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

                            // Always update input_tokens when provided (needed for Gemini which sends
                            // input tokens with every usage event, not just at start)
                            if usage.input_tokens > 0 && retry_usage.input_tokens == 0 {
                                retry_usage.input_tokens = usage.input_tokens;
                            }

                            if usage.output_tokens == 0 {
                                // MessageStart - new API call starting (Anthropic pattern)
                                // PROV-001 FIX: Accumulate previous segment's output BEFORE resetting
                                // In multi-turn tool loops, FinalResponse only comes at the very end.
                                // Without this, retry_cumulative_output stays at 0 during tool calls.
                                retry_cumulative_output += retry_usage.output_tokens;

                                // Update internal tracking but DON'T emit to display
                                retry_usage.input_tokens = usage.input_tokens;
                                retry_usage.output_tokens = 0;
                                // Update cache tokens silently
                                retry_usage.update_cache(
                                    usage.cache_read_input_tokens,
                                    usage.cache_creation_input_tokens,
                                );
                            } else {
                                // MessageDelta - update current API call's output tokens
                                retry_usage.output_tokens = usage.output_tokens;

                                // Update cache tokens
                                retry_usage.update_cache(
                                    usage.cache_read_input_tokens,
                                    usage.cache_creation_input_tokens,
                                );

                                // PROV-002 FIX: Use unified method for display token calculation
                                let (display_input, display_output) = retry_tok_tracker
                                    .calculate_display_tokens(&retry_usage, retry_cumulative_output, retry_prev_input_tokens);
                                let display_usage = ApiTokenUsage::new(
                                    display_input,
                                    retry_usage.cache_read_input_tokens,
                                    retry_usage.cache_creation_input_tokens,
                                    display_output,
                                );
                                output.emit_tokens(&TokenInfo::from_usage(
                                    display_usage,
                                    retry_tok_tracker.current_rate(),
                                ));
                                // CTX-004: Context fill uses CURRENT API output only
                                emit_context_fill_from_usage(
                                    output,
                                    &retry_usage,
                                    threshold,
                                    context_window,
                                );
                            }
                        }
                        Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                            // Get usage from FinalResponse
                            let usage = final_resp.usage();

                            // PROV-002: OpenAI-compatible providers (including Z.AI) don't emit Usage events
                            // during streaming - they only return usage in FinalResponse.
                            if retry_usage.input_tokens == 0 && usage.input_tokens > 0 {
                                // OpenAI-compatible path: extract tokens from FinalResponse
                                retry_usage.input_tokens = usage.input_tokens;
                                retry_usage.output_tokens = usage.output_tokens;
                                // PROV-002: Extract cached tokens from FinalResponse (Z.AI/OpenAI)
                                if let Some(cached) = usage.cache_read_input_tokens {
                                    retry_usage.cache_read_input_tokens = cached;
                                }
                                // PROV-002 FIX: ADD to cumulative, don't replace!
                                retry_cumulative_output += usage.output_tokens;
                            } else {
                                // PROV-001 FIX: FinalResponse.usage() contains AGGREGATED values
                                retry_cumulative_output += retry_usage.output_tokens;
                            }

                            // PROV-002: Use MAX of tiktoken estimate and actual to prevent going backwards
                            let tiktoken_output = retry_cumulative_output + retry_tok_tracker.cumulative_tokens;
                            let display_output = tiktoken_output.max(retry_cumulative_output);
                            let display_usage = ApiTokenUsage::new(
                                retry_usage.input_tokens,
                                retry_usage.cache_read_input_tokens,
                                retry_usage.cache_creation_input_tokens,
                                display_output,
                            );
                            output
                                .emit_tokens(&TokenInfo::from_usage(display_usage, retry_tok_tracker.current_rate()));
                            // CTX-004: Context fill uses CURRENT API output only
                            emit_context_fill_from_usage(
                                output,
                                &retry_usage,
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
                    // TOOL-011: Tool progress is emitted directly via progress_emitter callback
                }

                // TUI-031: Update session state after retry completes
                // PROV-001: Use ApiTokenUsage.total_input() for consistent calculation
                if !is_interrupted.load(Acquire) {
                    session.token_tracker.input_tokens = retry_usage.total_input();
                    session.token_tracker.output_tokens = retry_cumulative_output;
                    session.token_tracker.cumulative_billed_input += retry_usage.input_tokens;
                    session.token_tracker.cumulative_billed_output += retry_usage.output_tokens;
                    session.token_tracker.cache_read_input_tokens =
                        Some(retry_usage.cache_read_input_tokens);
                    session.token_tracker.cache_creation_input_tokens =
                        Some(retry_usage.cache_creation_input_tokens);
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
    // PROV-001: Use ApiTokenUsage.total_input() for consistent calculation
    //
    // Two distinct metrics:
    // - input_tokens: TOTAL context size for display (uses turn_usage.total_input())
    // - output_tokens: CUMULATIVE output tokens across all API calls (TUI-031)
    // - cumulative_billed_input/output: Sum of all API calls (for billing analytics)
    if !is_interrupted.load(Acquire) {
        // PROV-001: Store TOTAL context for display and threshold checks
        session.token_tracker.input_tokens = turn_usage.total_input();
        // TUI-031: Save CUMULATIVE output tokens so next turn continues from correct value
        session.token_tracker.output_tokens = turn_cumulative_output;
        // Accumulate for billing analytics (raw uncached input, not total context)
        // PROV-001 DEBUG: Log values before accumulation
        tracing::debug!(
            "PROV-001: Before accumulation: cumulative_billed_input={}, turn_usage.input_tokens={}",
            session.token_tracker.cumulative_billed_input,
            turn_usage.input_tokens
        );
        session.token_tracker.cumulative_billed_input += turn_usage.input_tokens;
        session.token_tracker.cumulative_billed_output += turn_usage.output_tokens;
        // PROV-001 DEBUG: Log values after accumulation
        tracing::debug!(
            "PROV-001: After accumulation: cumulative_billed_input={}, cumulative_billed_output={}",
            session.token_tracker.cumulative_billed_input,
            session.token_tracker.cumulative_billed_output
        );
        // Cache tokens are per-request, not cumulative (use latest values)
        session.token_tracker.cache_read_input_tokens = Some(turn_usage.cache_read_input_tokens);
        session.token_tracker.cache_creation_input_tokens =
            Some(turn_usage.cache_creation_input_tokens);
    }

    Ok(())
}
