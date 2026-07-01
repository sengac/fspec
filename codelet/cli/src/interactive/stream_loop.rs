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

use super::message_helpers::add_assistant_tool_calls_message;
use super::output::{ContextFillInfo, StreamOutput};
use super::stream_handlers::{
    handle_final_response, handle_text_chunk, handle_tool_call, handle_tool_result,
};
use crate::compaction_threshold::{resolve_compaction_threshold, CompactionThresholdConfig};
use crate::interactive_helpers::{
    convert_messages_to_turns, execute_compaction, inject_synthetic_tool_results_for_orphans,
    reconcile_session_messages,
};
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::annotation_detector::{
    detect_annotations, ToolCallInfo, TurnContext,
};
use codelet_core::{
    ensure_thought_signatures, ApiTokenUsage, CompactionHook, RigAgent, StreamingTokenDisplay,
    TokenState,
};
use codelet_tools::set_tool_progress_callback;
use codelet_tui::{InputQueue, StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::message::UserContent;
use rig::one_or_many::OneOrMany;
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
use tracing::{debug, error, info, trace, warn};

/// Detect structural annotations from a completed turn's tool calls and store
/// them in `session.annotations`. Rotates current into previous for next turn.
fn process_turn_annotations(
    session: &mut Session,
    current: &mut Vec<ToolCallInfo>,
    previous: &mut Vec<ToolCallInfo>,
) {
    if current.is_empty() {
        return;
    }
    let ctx = TurnContext {
        current_tool_calls: current,
        previous_tool_calls: if previous.is_empty() {
            None
        } else {
            Some(previous)
        },
    };
    let annotations = detect_annotations(&ctx);
    if !annotations.is_empty() {
        let msg_idx = session.messages.len().saturating_sub(1);
        session.annotations.insert(msg_idx, annotations);
    }
    *previous = std::mem::take(current);
}

// Error classifiers moved to error_classifiers.rs
use super::error_classifiers::{
    classify_compaction_branch, extract_prompt_cancelled, is_image_content_error,
    is_prompt_too_long_error, is_stall_timeout_error, is_transient_network_error,
    is_truncated_tool_call_error, CompactionBranch,
};

// Image recovery moved to recovery_image.rs
use super::recovery_image::sanitize_image_content;

// Truncation recovery moved to recovery_truncation.rs
use super::recovery_truncation::{
    build_truncation_budget_exhausted_message, build_truncation_recovery_message,
    MAX_TRUNCATION_RETRIES,
};

// Thinking recovery moved to recovery_thinking.rs
use super::recovery_thinking::{
    build_thinking_budget_exhausted_message, build_thinking_exhaustion_recovery_message,
    downgrade_thinking_level, is_thinking_exhaustion, MAX_THINKING_EXHAUSTION_RETRIES,
    THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD, THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
};

// Network recovery moved to recovery_network.rs
use super::recovery_network::{network_retry_delay, MAX_NETWORK_RETRIES};

// Stall timeout recovery moved to recovery_stall.rs
use super::recovery_stall::{build_stall_timeout_message, stall_timeout_duration};

// CMPCT-027: Compaction retry circuit breaker — bound cascaded compaction
// rounds per user turn. See `recovery_compaction::MAX_COMPACTION_RETRIES`.
use super::recovery_compaction::MAX_COMPACTION_RETRIES;

/// Emit context fill information from API token usage.
/// Extracted as a standalone function so it can be shared with the in-loop
/// compaction restart (`in_loop_compaction_restart!()` macro) and related
/// recovery handlers.
pub(super) fn emit_context_fill_from_usage<O: StreamOutput>(
    output: &O,
    usage: &ApiTokenUsage,
    threshold: u64,
    context_window: u64,
) {
    let total_tokens = usage.total_context();
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
}

// CMPCT-002 / CMPCT-023: `signal_compaction_needed(...)` used to live here.
// As of the compaction-recovery unification, all production entry paths
// (B, C, D) set `compaction_needed` through
// `recovery_compaction::begin_compaction_recovery`. Tests that need to
// flip the flag should do so directly on `TokenState::compaction_needed`.

/// Run agent stream with CLI event handling
///
/// This is the CLI-specific entry point that wraps the generic stream function
/// with TUI event handling for Esc key detection.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_agent_stream_with_interruption<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),
    input_queue: &mut InputQueue,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    output: &O,
    session_id: uuid::Uuid,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    run_agent_stream_internal(
        agent,
        prompt,
        None, // No images for CLI mode
        session,
        Some(event_stream),
        Some(input_queue),
        is_interrupted,
        compaction_in_progress,
        None, // CLI mode doesn't use Notify - uses keyboard event stream
        output,
        session_id,
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
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_stream<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
    output: &O,
    session_id: uuid::Uuid,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    run_agent_stream_internal::<M, O, dyn futures::Stream<Item = TuiEvent> + Unpin + Send>(
        agent,
        prompt,
        None, // No images for standard API
        session,
        None,
        None,
        is_interrupted,
        compaction_in_progress,
        Some(interrupt_notify),
        output,
        session_id,
    )
    .await
}

/// BRIDGE-007: Run agent stream with multimodal support (images)
///
/// Same as run_agent_stream but accepts optional images for multimodal input.
/// Called from NAPI agent_loop when bridge input includes images.
#[allow(clippy::too_many_arguments)]
pub async fn run_agent_stream_with_images<M, O>(
    agent: RigAgent<M>,
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
    session: &mut Session,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    interrupt_notify: Arc<Notify>,
    output: &O,
    session_id: uuid::Uuid,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
{
    run_agent_stream_internal::<M, O, dyn futures::Stream<Item = TuiEvent> + Unpin + Send>(
        agent,
        prompt,
        images,
        session,
        None,
        None,
        is_interrupted,
        compaction_in_progress,
        Some(interrupt_notify),
        output,
        session_id,
    )
    .await
}

// Multimodal content building moved to multimodal.rs
use super::multimodal::{build_user_content_with_images, BridgeImage};

/// Internal generic stream loop
///
/// Core streaming logic shared between CLI and NAPI modes.
/// - When event_stream is Some: Uses tokio::select! with event handling (CLI)
/// - When event_stream is None but interrupt_notify is Some: Uses tokio::select! with Notify (NAPI)
/// - NAPI-004: The interrupt_notify enables immediate ESC response during tool execution
/// - BRIDGE-007: Now supports optional images for multimodal input
#[allow(clippy::too_many_arguments)]
async fn run_agent_stream_internal<M, O, E>(
    agent: RigAgent<M>,
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
    session: &mut Session,
    mut event_stream: Option<&mut E>,
    mut input_queue: Option<&mut InputQueue>,
    is_interrupted: Arc<AtomicBool>,
    compaction_in_progress: Arc<AtomicBool>,
    interrupt_notify: Option<Arc<Notify>>,
    output: &O,
    session_id: uuid::Uuid,
) -> Result<()>
where
    M: CompletionModel,
    M::StreamingResponse: WasmCompatSend + GetTokenUsage,
    O: StreamOutput,
    E: futures::Stream<Item = TuiEvent> + Unpin + Send + ?Sized,
{
    use rig::message::Message;
    use std::time::Instant;
    use uuid::Uuid;

    // CLI-022: Generate request ID for correlation
    let request_id = Uuid::new_v4().to_string();
    let api_start_time = Instant::now();

    // HOOK-BASED COMPACTION (CTX-002: Optimized compaction trigger)
    let context_window = session.provider_manager().context_window() as u64;
    let max_output_tokens = session.provider_manager().max_output_tokens() as u64;
    // CTX-007: Resolve per-model compaction threshold through priority chain:
    // 1. User-configured override > 2. Built-in model family default > 3. Legacy formula
    let model_id = session.current_model_id();
    let user_config = session
        .provider_manager()
        .compaction_threshold_override()
        .map(|(t, v)| CompactionThresholdConfig::from_type_value(t, v));
    let threshold = resolve_compaction_threshold(
        context_window,
        max_output_tokens,
        model_id.as_deref(),
        user_config.as_ref(),
    );

    // DIAG: Log compaction parameters for debugging
    debug!(
        "DIAG stream_loop: model={:?}, context_window={}, max_output={}, threshold={}, input_tokens={}, output_tokens={}",
        session.current_model_id(),
        context_window,
        max_output_tokens,
        threshold,
        session.token_tracker.input_tokens,
        session.token_tracker.output_tokens
    );

    // CTX-005: PRE-PROMPT COMPACTION CHECK
    let mut compaction_just_ran = false;

    // Before adding the new prompt, estimate if current context + new prompt would exceed threshold.
    // This prevents "prompt is too long" API errors when resuming a session at high context fill.
    // The hook only checks AFTER API responses, but we need to check BEFORE the first API call.
    // PROV-005: Also check for actual conversation turns - system messages alone can't be compacted.
    let prompt_tokens = count_tokens(prompt) as u64;
    let current_tokens = session.token_tracker.input_tokens + session.token_tracker.output_tokens;
    let estimated_total = current_tokens + prompt_tokens;

    // PROV-005: Check if there are actual conversation turns to compact
    // convert_messages_to_turns returns empty if there are only system messages (no user+assistant pairs)
    let has_turns_to_compact = !convert_messages_to_turns(&session.messages).is_empty();

    // DIAG: Log pre-prompt compaction decision
    debug!(
        "DIAG pre-prompt check: prompt_tokens={}, current_tokens={}, estimated_total={}, threshold={}, has_turns={}, will_compact={}",
        prompt_tokens,
        current_tokens,
        estimated_total,
        threshold,
        has_turns_to_compact,
        estimated_total > threshold && has_turns_to_compact
    );

    if estimated_total > threshold && has_turns_to_compact {
        trace!(
            "Pre-prompt compaction triggered: estimated {} > threshold {}",
            estimated_total,
            threshold
        );
        // UX-002: Use structured compaction event instead of string status
        output.emit_compaction_started();

        // UX-002: Emit progress for automatic compaction
        let total_turns = session.messages.len() as u32 / 2; // Approximate turn count
        output.emit_compaction_progress("Analyzing context", 0, total_turns.max(1));

        match execute_compaction(session, compaction_in_progress.clone(), Some(prompt)).await {
            Ok(()) => {
                output.emit_compaction_continuing();
                session.token_tracker.reset_after_compaction();
                compaction_just_ran = true;
            }
            Err(e) => {
                // Log but continue - the API might still work, or will fail with clear error
                error!("Pre-prompt compaction failed: {}", e);
                // UX-002: Use structured compaction failed event
                output.emit_compaction_failed(&format!("{e}, continuing anyway"));
            }
        }
    }

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
                        "model": session.current_model_id().unwrap_or_else(|| session.current_provider_name().to_string()),
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
    // BUG-149: Track the active tool_call_id so the progress callback can emit
    // the REAL provider id instead of an empty string. The TUI folds progress
    // into a card by EXACT tool_call_id match; an empty id matches no card and
    // is silently dropped, so live output never streams. Tool execution is
    // serial within a turn (tool_execution_in_progress flag), so a single
    // active id is unambiguous. Set on ToolCall, cleared on ToolResult.
    let active_tool_call_id: std::sync::Arc<std::sync::Mutex<Option<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    if let Some(emitter) = output.progress_emitter() {
        // RPC-398: Register under the REAL per-session id that BashTool emits
        // with (threaded down from the caller that built create_rig_agent /
        // BashTool::new). Previously this used Uuid::nil() (BUG-126 CLI mode)
        // which never matched the emit key, so nothing streamed. Preserves
        // BUG-126 exact-match isolation — no global fallback.
        let active_tool_call_id_cb = active_tool_call_id.clone();
        set_tool_progress_callback(
            session_id,
            Some(Arc::new(move |chunk: &str, is_stderr: bool| {
                // BUG-149: emit the active tool_call_id. Falls back to the empty
                // string if no tool is active (stray emit) — the TUI drops that,
                // preserving prior behaviour without panicking.
                let id = active_tool_call_id_cb
                    .lock()
                    .ok()
                    .and_then(|g| g.clone())
                    .unwrap_or_default();
                emitter.emit_tool_progress(&id, "bash", chunk, is_stderr);
            })),
        );
    }

    // After compaction, the original prompt is embedded in the compaction instruction.
    // Use a synthetic prompt so rig doesn't duplicate it.
    let effective_prompt = if compaction_just_ran {
        "Continue"
    } else {
        prompt
    };

    // Start streaming with history and hook
    let mut stream = agent
        .prompt_streaming_with_history_and_hook(effective_prompt, &mut session.messages, hook)
        .await;

    // FIXED: Add user message to history AFTER rig clones it (CLI-008, BRIDGE-007)
    // This ensures: (1) LLM sees prompt once, (2) persistence still works
    // Previously this was done BEFORE the call above, causing duplication because
    // rig's build() concatenates chat_history + prompt, and the prompt was already in history.
    session.messages.push(Message::User {
        content: build_user_content_with_images(effective_prompt, images),
    });

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
    // PROV-041: Accumulate reasoning/thinking content for preservation on exhaustion
    let mut accumulated_reasoning = String::new();
    // PROV-039: Track stop_reason from FinalResponse for truncation detection
    let mut final_stop_reason: Option<String> = None;
    // PROV-040: Track consecutive truncation retries to prevent infinite loops
    let mut truncation_retry_count: u32 = 0;
    // PROV-041: Track consecutive thinking exhaustion retries to prevent infinite loops
    let mut thinking_exhaustion_retry_count: u32 = 0;
    // NET-001: Track consecutive network error retries to prevent infinite loops
    let mut network_retry_count: u32 = 0;
    // CMPCT-027: Track consecutive compaction retry rounds to prevent infinite
    // cascades. Incremented BEFORE each in-loop post-compaction restart
    // (Paths B/C/D). When it exceeds `MAX_COMPACTION_RETRIES` the loop
    // returns a structured budget-exhausted error.
    let mut compaction_retry_count: u32 = 0;
    // NET-001: Track whether we're recovering from a network retry (for UX feedback)
    let mut network_retry_in_progress = false;
    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
    let mut last_tool_name: Option<String> = None;
    // AMGR-016-FIX: Track whether a tool is currently executing inside rig's multi-turn stream.
    // Between a ToolCall chunk and the corresponding ToolResult chunk, stream.next() is blocked
    // running the tool. The stall timeout must be disabled during this window because tools
    // (DeepSearch, Bash, AgentManager await_idle) can legitimately take minutes.
    let mut tool_execution_in_progress = false;

    let mut turn_tool_infos: Vec<ToolCallInfo> = Vec::new();
    let mut previous_turn_tool_infos: Vec<ToolCallInfo> = Vec::new();

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

    // CMPCT-027: In-loop compaction-restart block shared by Paths B, C, and D.
    //
    // Caller contract:
    // - `begin_compaction_recovery` MUST have been called upstream (either in
    //   this match arm for Paths B/C, or inside `handle_gemini_continuation`
    //   for Path D) so partial assistant text has been saved, the trailing
    //   User prompt has been popped (if applicable), and the compaction
    //   lifecycle start events have been emitted.
    // - `compaction_in_progress: Arc<AtomicBool>` is in scope.
    //
    // Behavior:
    // 1. Circuit breaker: increment `compaction_retry_count`, return a
    //    structured `build_compaction_budget_exhausted_message` error if the
    //    budget is exceeded.
    // 2. Run the DAG-condensation compaction plus its debug capture events
    //    via `execute_compaction_and_capture_events`.
    // 3. Reset the outer `token_state` in place (not a new Arc) so the branch
    //    classifier continues to see the same state and the new
    //    `CompactionHook` watches the same Arc.
    // 4. Re-issue the stream with a `"Continue"` prompt — the caller is
    //    responsible for the following `continue;` so control stays in the
    //    primary loop and the same error cascade governs the retry.
    // 5. Reset per-turn locals so a subsequent FinalResponse / truncation /
    //    image handler sees a fresh state machine.
    //
    // CMPCT-028: The macro now takes an explicit `CompactionRecoveryPolicy`
    // expression. The retry prompt string is obtained via
    // `compaction_retry_prompt(policy)` instead of being hardcoded to
    // "Continue", so when partial Assistant text was preserved by
    // `flush_partial_state_before_compaction` the agent receives a resume
    // prompt that references the preserved work instead of the ambiguous
    // "Continue" signal. Path A (pre-prompt compaction) is unchanged because
    // it cannot have preserved partial text.
    //
    // Followed by `continue;` at each call site (Paths B, C, D).
    macro_rules! in_loop_compaction_restart {
        ($policy:expr) => {{
            compaction_retry_count += 1;
            if compaction_retry_count > MAX_COMPACTION_RETRIES {
                let msg = super::recovery_compaction::build_compaction_budget_exhausted_message(
                    MAX_COMPACTION_RETRIES,
                );
                warn!(
                    "CMPCT-027: Compaction retry budget exhausted after {} attempts",
                    MAX_COMPACTION_RETRIES
                );
                output.emit_error(&msg);
                return Err(anyhow::anyhow!(
                    "Compaction retry budget exhausted after {} attempts",
                    MAX_COMPACTION_RETRIES
                ));
            }

            super::recovery_compaction::execute_compaction_and_capture_events(
                session,
                compaction_in_progress.clone(),
                prompt,
                threshold,
                context_window,
                &token_state,
                output,
            )
            .await?;

            // Reset outer token_state in-place so the new hook watches the
            // same Arc. Cleared flag + fresh counters let the next error
            // classifier pass correctly (classify_compaction_branch reads
            // this same Arc).
            if let Ok(mut state) = token_state.lock() {
                state.compaction_needed = false;
                state.input_tokens = session.token_tracker.input_tokens;
                state.cache_read_input_tokens = 0;
                state.cache_creation_input_tokens = 0;
                state.output_tokens = 0;
            }
            let new_hook = CompactionHook::new(Arc::clone(&token_state), threshold);

            // CMPCT-028: pick the retry prompt from the caller-supplied
            // policy and log the selection so operators can audit which
            // branch fired. The string returned by `compaction_retry_prompt`
            // is static (`&'static str`), so forwarding it into rig is
            // zero-cost.
            let selected_policy: super::recovery_compaction::CompactionRecoveryPolicy = $policy;
            let retry_prompt =
                super::recovery_compaction::compaction_retry_prompt(selected_policy);
            debug!(
                policy = ?selected_policy,
                retry_prompt = retry_prompt,
                compaction_retry_count = compaction_retry_count,
                max_compaction_retries = MAX_COMPACTION_RETRIES,
                "[stream_loop] CMPCT-028: issuing in-loop post-compaction retry stream with policy-selected prompt"
            );

            debug!(
                "API REQUEST (compaction retry {}/{}) - Provider: {}, Model: {}",
                compaction_retry_count,
                MAX_COMPACTION_RETRIES,
                session.current_provider_name(),
                session.current_model_id().as_deref().unwrap_or("NONE")
            );

            // Re-issue the stream in place. Same `stream` binding, so the
            // enclosing loop's `match stream.next().await` keeps running.
            stream = agent
                .prompt_streaming_with_history_and_hook(
                    retry_prompt,
                    &mut session.messages,
                    new_hook,
                )
                .await;

            // Reset per-turn streaming state.
            tool_calls_buffer.clear();
            last_tool_name = None;
            final_stop_reason = None;
            accumulated_reasoning.clear();
            turn_tool_infos.clear();
            tool_execution_in_progress = false;

            streaming_display = StreamingTokenDisplay::new(
                session.token_tracker.input_tokens,
                0,
                0,
                0,
            );
        }};
    }

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

            // PROV-039: Propagate stop_reason even on interruption
            output.emit_done_with_stop_reason(final_stop_reason.take());
            break;
        }

        // AMGR-016: Stall timeout duration — resets on every received chunk.
        // If no streaming data (tokens, tool_calls, usage, etc.) arrives for this
        // duration, the stream is considered stalled and we abort.
        //
        // AMGR-016-FIX: When a tool is executing (between ToolCall and ToolResult chunks),
        // stream.next() blocks on the tool — no chunks are produced. Tools like DeepSearch
        // (300s), Bash, and AgentManager await_idle can legitimately take minutes. The stall
        // timeout must be disabled during tool execution to prevent false positives.
        let effective_stall_timeout = if tool_execution_in_progress {
            // Tool is running inside rig — disable stall timeout (use 24h as effectively infinite)
            Duration::from_secs(86400)
        } else {
            stall_timeout_duration()
        };

        // Process next chunk - different based on mode
        let chunk = match (&mut event_stream, &mut status_interval, &status) {
            (Some(es), Some(si), Some(st)) => {
                // CLI mode: Use tokio::select! with event stream, status interval, and stall timeout
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
                    _ = tokio::time::sleep(effective_stall_timeout) => {
                        // AMGR-016: CLI mode stall timeout — same behavior as NAPI mode.
                        // No data received for effective_stall_timeout duration — terminal error.
                        let stall_msg = build_stall_timeout_message(effective_stall_timeout.as_secs());
                        warn!("AMGR-016: {}", stall_msg);
                        output.emit_error(&stall_msg);

                        if !assistant_text.is_empty() {
                            handle_final_response(&assistant_text, &mut session.messages)?;
                        }

                        output.emit_done_with_stop_reason(Some("stall_timeout".to_string()));
                        return Err(anyhow::anyhow!("{stall_msg}"));
                    }
                }
            }
            _ => {
                // NAPI mode: Use tokio::select! with interrupt notification (NAPI-004)
                // and stall timeout (AMGR-016)
                // This allows immediate ESC response even during blocking operations
                // NOTE: Tool progress is emitted directly via progress_emitter callback,
                // not through tokio::select! because select! can't interleave during stream.next()
                match &interrupt_notify {
                    Some(notify) => {
                        let interrupt_fut = notify.notified();
                        tokio::select! {
                            c = stream.next() => Some(c),
                            _ = interrupt_fut => None, // Wakes immediately when interrupt() called
                            _ = tokio::time::sleep(effective_stall_timeout) => {
                                // AMGR-016: No data received for effective_stall_timeout duration.
                                // Emit a clear error and break — this is a terminal error
                                // that must NOT be caught by error classifiers (Rule [5], [6]).
                                let stall_msg = build_stall_timeout_message(effective_stall_timeout.as_secs());
                                warn!("AMGR-016: {}", stall_msg);
                                output.emit_error(&stall_msg);

                                // Preserve partial assistant text in history (Rule: mid-response stall)
                                if !assistant_text.is_empty() {
                                    handle_final_response(&assistant_text, &mut session.messages)?;
                                }

                                output.emit_done_with_stop_reason(Some("stall_timeout".to_string()));
                                return Err(anyhow::anyhow!("{stall_msg}"));
                            }
                        }
                    }
                    None => {
                        // Fallback for any mode without notify (shouldn't happen in practice)
                        // AMGR-016: Still apply stall timeout even without interrupt notify
                        match tokio::time::timeout(effective_stall_timeout, stream.next()).await {
                            Ok(chunk) => Some(chunk),
                            Err(_elapsed) => {
                                let stall_msg =
                                    build_stall_timeout_message(effective_stall_timeout.as_secs());
                                warn!("AMGR-016: {}", stall_msg);
                                output.emit_error(&stall_msg);

                                if !assistant_text.is_empty() {
                                    handle_final_response(&assistant_text, &mut session.messages)?;
                                }

                                output
                                    .emit_done_with_stop_reason(Some("stall_timeout".to_string()));
                                return Err(anyhow::anyhow!("{stall_msg}"));
                            }
                        }
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
                    // NET-001: Reset network retry counter on successful data receipt
                    if network_retry_in_progress {
                        output.emit_status("✓ Reconnected");
                        network_retry_in_progress = false;
                    }
                    network_retry_count = 0;
                    handle_text_chunk(&text.text, &mut assistant_text, Some(&request_id), output)?;

                    // STREAMING-DISPLAY: Track chunk and emit if not throttled
                    if let Some(update) = streaming_display.record_chunk(&text.text) {
                        output.emit_tokens(&update.into());
                    }
                }
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ToolCall(tool_call),
                ))) => {
                    // NET-001: Reset network retry counter on successful data receipt
                    if network_retry_in_progress {
                        output.emit_status("✓ Reconnected");
                        network_retry_in_progress = false;
                    }
                    network_retry_count = 0;
                    handle_tool_call(
                        &tool_call,
                        &mut session.messages,
                        &mut assistant_text,
                        &mut tool_calls_buffer,
                        &mut last_tool_name,
                        output,
                    )?;

                    turn_tool_infos.push(ToolCallInfo {
                        tool_name: tool_call.function.name.clone(),
                        input: tool_call.function.arguments.clone(),
                        output: None,
                        success: true, // Assume success until result arrives
                    });

                    // AMGR-016-FIX: Mark tool execution in progress — the next stream.next()
                    // will block on tool execution. Stall timeout must be disabled until
                    // the ToolResult chunk arrives.
                    // BUG-149: set the active tool_call_id so live progress emitted during
                    // this tool's execution carries the real id and folds into its card.
                    if let Ok(mut g) = active_tool_call_id.lock() {
                        *g = Some(tool_call.id.clone());
                    }
                    tool_execution_in_progress = true;
                }
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ReasoningDelta { reasoning, .. },
                ))) => {
                    // NET-001: Reset network retry counter on successful data receipt
                    // ReasoningDelta is valid data — if a network error occurs after receiving
                    // reasoning but before text, the counter should reset.
                    if network_retry_in_progress {
                        output.emit_status("✓ Reconnected");
                        network_retry_in_progress = false;
                    }
                    network_retry_count = 0;

                    // TOOL-010: Emit thinking/reasoning content from extended thinking
                    output.emit_thinking(&reasoning);

                    // PROV-041: Accumulate reasoning content for preservation on thinking exhaustion
                    accumulated_reasoning.push_str(&reasoning);

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

                    // Update matching ToolCallInfo with result data for annotation detection
                    if let Some(info) = turn_tool_infos
                        .iter_mut()
                        .rev()
                        .find(|ti| ti.output.is_none())
                    {
                        let result_text: String = tool_result
                            .content
                            .clone()
                            .into_iter()
                            .map(|c| match c {
                                rig::message::ToolResultContent::Text(t) => t.text,
                                rig::message::ToolResultContent::Image(_) => "[Image]".to_string(),
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        info.success = !super::stream_handlers::detect_tool_error(&result_text);
                        info.output = Some(result_text);
                    }

                    // PROV-001: Don't emit token updates after tool results
                    // This is when a new API segment is about to start, and the next
                    // MessageStart may have very different input values (cache growing).
                    // Emitting here causes "bouncing" in the display.
                    // Token updates only occur during text streaming and at FinalResponse.

                    // AMGR-016-FIX: Tool execution completed — re-enable stall timeout.
                    // The next stream.next() will be waiting for the LLM's next API response,
                    // where the 600s stall timeout is appropriate again.
                    // BUG-149: clear the active tool_call_id so a later stray progress emit
                    // does not carry a stale id.
                    if let Ok(mut g) = active_tool_call_id.lock() {
                        *g = None;
                    }
                    tool_execution_in_progress = false;
                }
                Some(Ok(MultiTurnStreamItem::Usage(usage))) => {
                    // NET-001: Reset network retry counter on successful data receipt
                    if network_retry_in_progress {
                        output.emit_status("✓ Reconnected");
                        network_retry_in_progress = false;
                    }
                    network_retry_count = 0;
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
                            )
                            .with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
                            emit_context_fill_from_usage(
                                output,
                                &fill_usage,
                                threshold,
                                context_window,
                            );
                        }
                    }
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                    // NET-001: Reset network retry counter on successful data receipt
                    if network_retry_in_progress {
                        output.emit_status("✓ Reconnected");
                        network_retry_in_progress = false;
                    }
                    network_retry_count = 0;
                    // PROV-005-DEBUG: Log FinalResponse received
                    debug!(
                        "[stream_loop] FinalResponse received - checking compaction_needed state"
                    );

                    // PROV-039: Capture stop_reason from FinalResponse
                    final_stop_reason = final_resp.stop_reason().map(String::from);
                    if let Some(ref reason) = final_stop_reason {
                        debug!("[stream_loop] FinalResponse stop_reason={}", reason);
                    }

                    // Get usage from FinalResponse
                    let usage = final_resp.usage();

                    // PROV-002: OpenAI-compatible providers (including Z.AI) don't emit Usage events
                    // during streaming - they only return usage in FinalResponse.
                    // STREAMING-DISPLAY: update_from_final_response handles this case
                    let final_update = if !streaming_display.has_authoritative_output()
                        && usage.input_tokens > 0
                    {
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
                    )
                    .with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
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
                                            "reasoningTokens": usage.reasoning_tokens,
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
                                            "reasoningTokens": usage.reasoning_tokens,
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
                                        "reasoningTokens": usage.reasoning_tokens,
                                        "totalInputTokens": final_update.total_input(),
                                        "totalOutputTokens": final_update.output_tokens,
                                    }),
                                    None,
                                );
                            }
                        }
                    }

                    // GEMINI-TURN: Check if Gemini model needs a continuation prompt
                    // Extracted to gemini_continuation.rs
                    {
                        use super::gemini_continuation::{
                            handle_gemini_continuation, GeminiContinuationResult,
                        };
                        match handle_gemini_continuation(
                            &agent,
                            session,
                            output,
                            &is_interrupted,
                            &mut input_queue,
                            &token_state,
                            threshold,
                            &assistant_text,
                            &streaming_display,
                            &mut final_stop_reason,
                        )
                        .await?
                        {
                            GeminiContinuationResult::NoContinuation => {
                                // Fall through to normal processing below
                            }
                            GeminiContinuationResult::Completed => {
                                return Ok(());
                            }
                            GeminiContinuationResult::CompactionNeeded(policy) => {
                                // CMPCT-027: Path D — compaction was triggered
                                // inside the Gemini continuation sub-loop.
                                // `begin_compaction_recovery(..., pop_user_prompt=false)`
                                // has already run inside `handle_gemini_continuation`,
                                // so partial text is saved, the continuation
                                // prompt is preserved mid-flight, and the
                                // lifecycle start events have been emitted.
                                // We now run the in-loop restart so the same
                                // error cascade governs the new stream.
                                //
                                // CMPCT-028: the Gemini helper selected the
                                // CompactionRecoveryPolicy when it called
                                // `begin_compaction_recovery` (or computed it
                                // inline for the stream-end-with-flag path).
                                // We forward the policy into the macro so
                                // the retry prompt string is chosen based on
                                // whether partial text was preserved rather
                                // than defaulting to the hardcoded "Continue".
                                debug!(
                                    policy = ?policy,
                                    "[stream_loop] CMPCT-027: in-loop compaction restart (Path D — Gemini continuation)"
                                );
                                // Flush any assistant_text that was left in
                                // the primary loop's buffer. The Gemini
                                // helper operated on its own local buffer so
                                // the outer `assistant_text` is typically
                                // empty here, but clearing defensively
                                // prevents it from leaking into the retry.
                                assistant_text.clear();
                                in_loop_compaction_restart!(policy);
                                continue;
                            }
                        }
                    }

                    // Normal case: add assistant text to history and finish
                    handle_final_response(&assistant_text, &mut session.messages)?;

                    // PROV-041: Check for thinking token exhaustion before finalizing
                    // This must happen AFTER handle_final_response (so the response is in history)
                    // but BEFORE emitting done (so we can retry if needed).
                    {
                        let usage = final_resp.usage();
                        let reasoning_tokens = usage.reasoning_tokens.unwrap_or(0);
                        let output_tokens = usage.output_tokens;
                        let stop_reason_ref = final_stop_reason.as_deref();

                        if is_thinking_exhaustion(
                            stop_reason_ref,
                            reasoning_tokens,
                            output_tokens,
                            THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
                        ) {
                            thinking_exhaustion_retry_count += 1;
                            // PROV-041: Track cross-turn exhaustion for session-level degradation
                            // Only increment once per turn (first detection), not on retries
                            if thinking_exhaustion_retry_count == 1 {
                                session.thinking_exhaustion_cross_turn_count += 1;
                            }
                            info!(
                                "PROV-041: Thinking exhaustion detected (attempt {}/{}, cross-turn #{}): reasoning_tokens={}, output_tokens={}, stop_reason={:?}",
                                thinking_exhaustion_retry_count,
                                MAX_THINKING_EXHAUSTION_RETRIES,
                                session.thinking_exhaustion_cross_turn_count,
                                reasoning_tokens,
                                output_tokens,
                                stop_reason_ref
                            );

                            // PROV-041: Session-level progressive degradation (Rule[7])
                            // When cross-turn threshold is reached, downgrade the session thinking
                            // level and reset the counter for the next degradation cycle.
                            if thinking_exhaustion_retry_count == 1
                                && session.thinking_exhaustion_cross_turn_count
                                    >= THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD
                            {
                                session.session_thinking_level =
                                    downgrade_thinking_level(session.session_thinking_level);
                                session.thinking_exhaustion_cross_turn_count = 0;
                                output.emit_status(&format!(
                                    "Reasoning effort automatically reduced to {:?} due to repeated thinking budget exhaustion",
                                    session.session_thinking_level,
                                ));
                            }

                            if thinking_exhaustion_retry_count <= MAX_THINKING_EXHAUSTION_RETRIES {
                                // PROV-041: Preserve actual reasoning/thinking content (not assistant_text)
                                let captured_reasoning = if !accumulated_reasoning.is_empty() {
                                    Some(accumulated_reasoning.as_str())
                                } else if !assistant_text.is_empty() {
                                    // Fallback to assistant_text if no reasoning deltas were captured
                                    Some(assistant_text.as_str())
                                } else {
                                    None
                                };
                                let recovery_msg = build_thinking_exhaustion_recovery_message(
                                    reasoning_tokens,
                                    output_tokens,
                                    captured_reasoning,
                                );

                                // Emit warning to the user
                                output.emit_status(&format!(
                                    "Thinking exhaustion detected (attempt {thinking_exhaustion_retry_count}/{MAX_THINKING_EXHAUSTION_RETRIES}). Retrying with reduced reasoning budget..."
                                ));

                                // PROV-041: Context preservation at >90% utilization (Rule[5])
                                let current_tokens = session.token_tracker.input_tokens
                                    + session.token_tracker.output_tokens;
                                let utilization_pct = if context_window > 0 {
                                    (current_tokens as f64 / context_window as f64) * 100.0
                                } else {
                                    0.0
                                };
                                if utilization_pct > 90.0 {
                                    info!(
                                        "PROV-041: Context utilization at {:.1}% — persisting session state before retry",
                                        utilization_pct
                                    );
                                    output.emit_status(&format!(
                                        "Context window at {utilization_pct:.0}% — session state preserved before retry"
                                    ));
                                    // The session.messages already contain the full conversation history.
                                    // In the NAPI layer, the persist_pending_annotations call after
                                    // run_agent_stream ensures the session is written to disk.
                                    // Here we ensure the recovery message is part of that persistence.
                                }

                                // PROV-041: Create fresh hook and token state for the retry
                                // (same pattern as PROV-040 truncation recovery)
                                let retry_token_state = Arc::new(Mutex::new(TokenState {
                                    input_tokens: session.token_tracker.input_tokens,
                                    cache_read_input_tokens: 0,
                                    cache_creation_input_tokens: 0,
                                    output_tokens: 0,
                                    compaction_needed: false,
                                }));
                                let retry_hook =
                                    CompactionHook::new(Arc::clone(&retry_token_state), threshold);

                                debug!(
                                    "API REQUEST (thinking exhaustion recovery {}/{}) - Provider: {}, Model: {}",
                                    thinking_exhaustion_retry_count,
                                    MAX_THINKING_EXHAUSTION_RETRIES,
                                    session.current_provider_name(),
                                    session.current_model_id().as_deref().unwrap_or("NONE")
                                );

                                // PROV-041: Start new stream with recovery prompt
                                // (rig clones session.messages and appends recovery_msg as new user message)
                                stream = agent
                                    .prompt_streaming_with_history_and_hook(
                                        &recovery_msg,
                                        &mut session.messages,
                                        retry_hook,
                                    )
                                    .await;

                                // Add recovery prompt to session.messages for persistence
                                session.messages.push(Message::User {
                                    content: OneOrMany::one(UserContent::text(&recovery_msg)),
                                });

                                // Reset per-stream tracking for the retry
                                assistant_text.clear();
                                accumulated_reasoning.clear();
                                final_stop_reason = None;
                                tool_calls_buffer.clear();
                                last_tool_name = None;
                                turn_tool_infos.clear();
                                tool_execution_in_progress = false;

                                // STREAMING-DISPLAY: Reset display for retry
                                streaming_display = StreamingTokenDisplay::new(
                                    session.token_tracker.input_tokens,
                                    session.token_tracker.output_tokens,
                                    session.token_tracker.cache_read_input_tokens.unwrap_or(0),
                                    session
                                        .token_tracker
                                        .cache_creation_input_tokens
                                        .unwrap_or(0),
                                );

                                debug!("[stream_loop] PROV-041: Created new stream for thinking exhaustion retry");
                                continue;
                            } else {
                                // Budget exhausted — emit warning and continue with best available response
                                let budget_msg = build_thinking_budget_exhausted_message(
                                    MAX_THINKING_EXHAUSTION_RETRIES,
                                );
                                warn!(
                                    "PROV-041: Thinking exhaustion retry budget exhausted after {} attempts",
                                    MAX_THINKING_EXHAUSTION_RETRIES
                                );
                                output.emit_status(&budget_msg);
                                // Fall through to normal completion
                            }
                        }
                    }

                    process_turn_annotations(
                        session,
                        &mut turn_tool_infos,
                        &mut previous_turn_tool_infos,
                    );

                    // CMPCT-032: Check compaction flag on the clean-exit
                    // path (FinalResponse). If the hook set
                    // `compaction_needed=true` during this turn but rig
                    // yielded `Ok(FinalResponse)` rather than
                    // `PromptCancelled` (e.g. the hook fired on the
                    // post-completion callback, or the API returned a
                    // complete response before cancellation propagated),
                    // we must run recovery before `emit_done_with_stop_reason`.
                    // Without this check the flag is silently dropped and
                    // the NEXT user turn explodes with `prompt is too long`
                    // — the exact regression tracked by CMPCT-032.
                    //
                    // Interrupts take priority — if the user cancelled,
                    // skip recovery and honour the interrupt state.
                    //
                    // `assistant_text` was already appended via
                    // `handle_final_response` above; clear it so
                    // `begin_compaction_recovery`'s flush is a no-op (the
                    // token tracker still gets flushed from
                    // `streaming_display`).
                    let final_response_needs_recovery = !is_interrupted.load(Acquire)
                        && token_state
                            .lock()
                            .map(|s| s.compaction_needed)
                            .unwrap_or(false);
                    if final_response_needs_recovery {
                        warn!(
                            "[stream_loop] CMPCT-032: FinalResponse branch exiting with \
                             compaction_needed=true — running recovery before emit_done \
                             to prevent silent context-window exhaustion on the next turn"
                        );
                        assistant_text.clear();
                        let policy = super::recovery_compaction::begin_compaction_recovery(
                            session,
                            &token_state,
                            &streaming_display,
                            &mut assistant_text,
                            output,
                            false, // FinalResponse clean-exit: no trailing User prompt
                        )?;
                        debug!(
                            policy = ?policy,
                            "[stream_loop] CMPCT-032: in-loop compaction restart (FinalResponse clean-exit)"
                        );
                        in_loop_compaction_restart!(policy);
                        continue;
                    }

                    // PROV-039: Emit done with stop_reason for truncation detection
                    output.emit_done_with_stop_reason(final_stop_reason.take());
                    break;
                }
                Some(Err(e)) => {
                    // PROV-009-DEBUG: Log EVERY error at debug level to trace compaction issues
                    debug!(
                        "[stream_loop] STREAM ERROR RECEIVED: error={}, type={:?}",
                        e,
                        std::any::type_name_of_val(&e)
                    );

                    // CMPCT-026: Single-source-of-truth classification. The
                    // helper treats the error as authoritative (structural
                    // downcast to `PromptError::PromptCancelled` via
                    // CMPCT-025's `extract_prompt_cancelled`) and treats the
                    // shared `TokenState.compaction_needed` flag as
                    // defence-in-depth. When the two signals disagree, the
                    // helper emits a structured `tracing::warn!` AND records
                    // the disagreement variant on the returned branch for
                    // downstream observability. See
                    // `error_classifiers::classify_compaction_branch`.
                    let branch = classify_compaction_branch(&e, &token_state);

                    debug!("[stream_loop] Error classification: branch={:?}", branch);

                    if matches!(branch, CompactionBranch::Recover { .. }) {
                        // CMPCT-029: reconcile in-flight tool state BEFORE
                        // begin_compaction_recovery runs. The recovery order is:
                        //   1. If PromptCancelled carries a rig chat_history,
                        //      merge any tool_call/tool_result pairs rig has
                        //      but fspec's session.messages doesn't yet.
                        //      The rig patch at streaming.rs site 508 flushes
                        //      the pending tool pair so it shows up here.
                        //   2. Drain fspec's tool_calls_buffer into
                        //      session.messages (Path 486 recovery — the hook
                        //      cancelled BEFORE the tool executed, so there
                        //      is no result to merge; the call is still in
                        //      the buffer from handle_tool_call).
                        //   3. Close any remaining orphan tool_calls with a
                        //      synthetic "cancelled_by_context_limit" result.
                        //
                        // After this, execute_compaction's defensive orphan
                        // guard will pass cleanly — regardless of which cancel
                        // site rig fired at.
                        if let Some(rig_chat_history) = extract_prompt_cancelled(&e) {
                            debug!(
                                rig_history_len = rig_chat_history.len(),
                                "[stream_loop] CMPCT-029: reconciling session.messages with rig PromptCancelled chat_history"
                            );
                            reconcile_session_messages(&mut session.messages, rig_chat_history);
                        } else {
                            debug!(
                                "[stream_loop] CMPCT-029: PromptCancelled payload not found in error chain; skipping reconcile"
                            );
                        }

                        if !tool_calls_buffer.is_empty() {
                            debug!(
                                buffer_len = tool_calls_buffer.len(),
                                "[stream_loop] CMPCT-029: draining tool_calls_buffer into session.messages (site 486 recovery)"
                            );
                            add_assistant_tool_calls_message(
                                &mut session.messages,
                                tool_calls_buffer.clone(),
                            )?;
                            tool_calls_buffer.clear();
                        }

                        let injected =
                            inject_synthetic_tool_results_for_orphans(&mut session.messages);
                        if injected > 0 {
                            warn!(
                                injected,
                                "[stream_loop] CMPCT-029: injected {} synthetic cancelled tool_result(s) before compaction",
                                injected
                            );
                        }

                        // CMPCT-023: unified compaction-recovery entry.
                        // The helper saves partial assistant text (BUG 2),
                        // flushes the token tracker (BUG 6), pops the trailing
                        // User prompt (pop_user_prompt=true — it has not been
                        // consumed by the API), sets compaction_needed on the
                        // shared token state (warn on disagreement), clears the
                        // global tool progress callback, and emits
                        // compaction_started + compaction_progress events.
                        //
                        // CMPCT-027: after recovery, we now run the compaction
                        // + stream restart IN-LOOP via the
                        // `in_loop_compaction_restart!()` macro so all
                        // post-compaction errors pass through the SAME error
                        // cascade as the original stream (BUG 5 fix).
                        //
                        // CMPCT-026: this branch fires on ANY PromptCancelled
                        // in the error chain even if `compaction_needed` was
                        // false — classify_compaction_branch has already
                        // defensively set the flag and emitted a warning.
                        //
                        // CMPCT-028: capture the CompactionRecoveryPolicy
                        // returned by begin_compaction_recovery and thread it
                        // into the macro so the retry prompt is the resume
                        // prompt when partial assistant text was preserved,
                        // or "Continue" when no partial text existed.
                        debug!(
                            "[stream_loop] CMPCT-023: invoking begin_compaction_recovery (Path C, pop_user_prompt=true)"
                        );
                        let policy = super::recovery_compaction::begin_compaction_recovery(
                            session,
                            &token_state,
                            &streaming_display,
                            &mut assistant_text,
                            output,
                            true,
                        )?;

                        debug!(
                            policy = ?policy,
                            "[stream_loop] CMPCT-027: in-loop compaction restart (Path C)"
                        );
                        in_loop_compaction_restart!(policy);
                        continue;
                    }
                    // CMPCT-026: NotCompaction — fall through to the
                    // prompt-too-long / image-content / truncation classifier
                    // cascade. Any `FlagExtraneous` disagreement has already
                    // been surfaced via `tracing::warn!` inside the helper.

                    // Check if this is a "prompt is too long" error from the API
                    let error_str = e.to_string();

                    // AMGR-016: Stall timeout errors are terminal — they must bypass
                    // ALL error classifiers and never be retried (Rule [5], Rule [6]).
                    // This guard prevents accidental classification as network/truncation errors.
                    if is_stall_timeout_error(&error_str) {
                        error!("AMGR-016: Stall timeout reached error classifier cascade (unexpected path)");
                        output.emit_error(&error_str);
                        return Err(anyhow::anyhow!("Agent error: {e}"));
                    }

                    let is_prompt_too_long = is_prompt_too_long_error(&error_str);

                    // PROV-010: Only trigger compaction if there are actual user/assistant turns to compact
                    // session.messages may contain system prompts but no compactable turns
                    let has_compactable_turns =
                        !convert_messages_to_turns(&session.messages).is_empty();

                    if is_prompt_too_long && has_compactable_turns {
                        info!(
                            "Received 'prompt is too long' error, triggering recovery compaction"
                        );
                        // CMPCT-023: unified compaction-recovery entry
                        // (Path B — API-returned "prompt is too long").
                        //
                        // The helper saves partial assistant text, flushes the
                        // token tracker, pops the trailing User prompt (it was
                        // pushed by rig before the error and has not been
                        // consumed), sets compaction_needed, clears the tool
                        // progress callback, and emits compaction_started +
                        // compaction_progress events.
                        //
                        // CMPCT-027: after recovery, we now run the compaction
                        // + stream restart IN-LOOP via the
                        // `in_loop_compaction_restart!()` macro so any
                        // post-compaction "prompt is too long" (i.e. the
                        // compaction didn't reduce context enough), PromptCancelled
                        // (hook fires again), truncation, or image-content
                        // error is handled by the SAME error cascade (BUG 5 fix).
                        //
                        // CMPCT-028: capture the CompactionRecoveryPolicy
                        // returned by begin_compaction_recovery and thread it
                        // into the macro so the retry prompt honors the
                        // partial-text preservation signal.
                        debug!(
                            "[stream_loop] CMPCT-023: invoking begin_compaction_recovery (Path B, pop_user_prompt=true)"
                        );
                        let policy = super::recovery_compaction::begin_compaction_recovery(
                            session,
                            &token_state,
                            &streaming_display,
                            &mut assistant_text,
                            output,
                            true,
                        )?;

                        debug!(
                            policy = ?policy,
                            "[stream_loop] CMPCT-027: in-loop compaction restart (Path B)"
                        );
                        in_loop_compaction_restart!(policy);
                        continue;
                    }

                    // EXT-016: Check if this is an image content error (dimensions, size, etc.)
                    // Try to recover by sanitizing image content from conversation history
                    if is_image_content_error(&error_str) {
                        warn!("Received image content error from API, attempting to sanitize conversation history");

                        // Pop the last user message first (same pattern as prompt-too-long)
                        if let Some(last_msg) = session.messages.last() {
                            if matches!(last_msg, rig::message::Message::User { .. }) {
                                session.messages.pop();
                                info!("Popped last user message before image sanitization");
                            }
                        }

                        // Sanitize image content from remaining history
                        let sanitized = sanitize_image_content(&mut session.messages);

                        if sanitized {
                            info!("Sanitized image content from conversation history — session can continue");
                            output.emit_error(&format!(
                                "{error_str}\n\n[Images removed from conversation history to recover session]"
                            ));
                            // Don't return Err — session remains usable, user can send next message
                            break;
                        }
                        // If no images found to sanitize, fall through to normal error handling
                    }

                    // PROV-040: Check if this is a truncated tool call error
                    // When the LLM hits max_tokens mid-tool-call, PROV-039 emits an enriched
                    // error. Instead of returning that error (which causes infinite retry loops),
                    // inject a recovery prompt telling the model to use an alternative strategy.
                    if is_truncated_tool_call_error(&error_str) {
                        truncation_retry_count += 1;

                        if truncation_retry_count <= MAX_TRUNCATION_RETRIES {
                            info!(
                                "PROV-040: Truncated tool call detected (attempt {}/{}), injecting recovery prompt",
                                truncation_retry_count, MAX_TRUNCATION_RETRIES
                            );

                            // Save any partial assistant text accumulated before truncation
                            if !assistant_text.is_empty() {
                                handle_final_response(&assistant_text, &mut session.messages)?;
                                assistant_text.clear();
                            }

                            // Build recovery message with alternative strategies
                            let recovery_prompt = build_truncation_recovery_message(&error_str);

                            // Create fresh hook and token state for the retry
                            let retry_token_state = Arc::new(Mutex::new(TokenState {
                                input_tokens: session.token_tracker.input_tokens,
                                cache_read_input_tokens: 0,
                                cache_creation_input_tokens: 0,
                                output_tokens: 0,
                                compaction_needed: false,
                            }));
                            let retry_hook =
                                CompactionHook::new(Arc::clone(&retry_token_state), threshold);

                            debug!(
                                "API REQUEST (truncation recovery {}/{}) - Provider: {}, Model: {}",
                                truncation_retry_count,
                                MAX_TRUNCATION_RETRIES,
                                session.current_provider_name(),
                                session.current_model_id().as_deref().unwrap_or("NONE")
                            );

                            // Start new stream with recovery prompt
                            // rig clones session.messages and appends recovery_prompt as new user message
                            stream = agent
                                .prompt_streaming_with_history_and_hook(
                                    &recovery_prompt,
                                    &mut session.messages,
                                    retry_hook,
                                )
                                .await;

                            // Add recovery prompt to session.messages for persistence
                            // (same pattern as line 646 where we push the original prompt)
                            session.messages.push(Message::User {
                                content: OneOrMany::one(UserContent::text(&recovery_prompt)),
                            });

                            // Reset per-stream tracking for the retry
                            tool_calls_buffer.clear();
                            last_tool_name = None;
                            final_stop_reason = None;
                            turn_tool_infos.clear();
                            tool_execution_in_progress = false;

                            // STREAMING-DISPLAY: Reset display for retry
                            streaming_display = StreamingTokenDisplay::new(
                                session.token_tracker.input_tokens,
                                session.token_tracker.output_tokens,
                                session.token_tracker.cache_read_input_tokens.unwrap_or(0),
                                session
                                    .token_tracker
                                    .cache_creation_input_tokens
                                    .unwrap_or(0),
                            );

                            continue;
                        }

                        // Retry budget exhausted — report to user and terminate
                        warn!(
                            "PROV-040: Truncation retry budget exhausted after {} attempts",
                            MAX_TRUNCATION_RETRIES
                        );
                        let budget_error =
                            build_truncation_budget_exhausted_message(MAX_TRUNCATION_RETRIES);
                        output.emit_error(&budget_error);
                        return Err(anyhow::anyhow!("Agent error: truncation retry budget exhausted after {MAX_TRUNCATION_RETRIES} attempts"));
                    }

                    // NET-001: Check for transient network/connection errors.
                    // When the HTTP connection drops mid-stream (e.g., "error sending request"),
                    // retry the request using the full conversation history rather than failing.
                    // The retry happens here (stream_loop level) because chat completion APIs
                    // are stateless — SSE-level reconnection would lose accumulated state.
                    if is_transient_network_error(&error_str) {
                        network_retry_count += 1;

                        if network_retry_count <= MAX_NETWORK_RETRIES {
                            let delay = network_retry_delay(network_retry_count);
                            info!(
                                "NET-001: Transient network error detected (attempt {}/{}), retrying in {:.1}s: {}",
                                network_retry_count,
                                MAX_NETWORK_RETRIES,
                                delay.as_secs_f64(),
                                error_str
                            );
                            // Only show the reconnecting message once (first attempt).
                            // Subsequent retries are silent to avoid cluttering the conversation.
                            if network_retry_count == 1 {
                                output.emit_status("⟳ Reconnecting...");
                            }
                            network_retry_in_progress = true;

                            tokio::time::sleep(delay).await;

                            // Check for interruption during the sleep
                            if is_interrupted.load(Acquire) {
                                break;
                            }

                            // Save any partial assistant text accumulated before the disconnect
                            if !assistant_text.is_empty() {
                                handle_final_response(&assistant_text, &mut session.messages)?;
                                assistant_text.clear();
                            }

                            // Create fresh hook and token state for the retry
                            let retry_token_state = Arc::new(Mutex::new(TokenState {
                                input_tokens: session.token_tracker.input_tokens,
                                cache_read_input_tokens: 0,
                                cache_creation_input_tokens: 0,
                                output_tokens: session.token_tracker.output_tokens,
                                compaction_needed: false,
                            }));
                            let retry_hook =
                                CompactionHook::new(Arc::clone(&retry_token_state), threshold);

                            debug!(
                                "API REQUEST (network retry {}/{}) - Provider: {}, Model: {}",
                                network_retry_count,
                                MAX_NETWORK_RETRIES,
                                session.current_provider_name(),
                                session.current_model_id().as_deref().unwrap_or("NONE")
                            );

                            // Restart the stream with "Continue" prompt.
                            // Conversation history is intact — model picks up where it left off.
                            stream = agent
                                .prompt_streaming_with_history_and_hook(
                                    "Continue",
                                    &mut session.messages,
                                    retry_hook,
                                )
                                .await;

                            // Add continuation prompt to session messages for persistence
                            session.messages.push(Message::User {
                                content: OneOrMany::one(UserContent::text("Continue")),
                            });

                            // Reset per-stream tracking for the retry
                            tool_calls_buffer.clear();
                            last_tool_name = None;
                            final_stop_reason = None;
                            accumulated_reasoning.clear();
                            turn_tool_infos.clear();
                            tool_execution_in_progress = false;

                            // Reset streaming display for retry
                            streaming_display = StreamingTokenDisplay::new(
                                session.token_tracker.input_tokens,
                                session.token_tracker.output_tokens,
                                session.token_tracker.cache_read_input_tokens.unwrap_or(0),
                                session
                                    .token_tracker
                                    .cache_creation_input_tokens
                                    .unwrap_or(0),
                            );

                            debug!("[stream_loop] NET-001: Created new stream for network retry");
                            continue;
                        }

                        // Retry budget exhausted — fall through to terminal error
                        warn!(
                            "NET-001: Network retry budget exhausted after {} attempts",
                            MAX_NETWORK_RETRIES
                        );
                        output.emit_status("✗ Reconnection failed");
                        // Fall through to the terminal error handler below
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
                    // PROV-005-DEBUG: Log stream ended
                    debug!(
                        "[stream_loop] Stream ended (None) - assistant_text_len={}, checking compaction_needed",
                        assistant_text.len()
                    );
                    // Check compaction state at stream end
                    if let Ok(state) = token_state.lock() {
                        debug!(
                            "[stream_loop] At stream end: compaction_needed={}, input={}, output={}",
                            state.compaction_needed,
                            state.input_tokens,
                            state.output_tokens
                        );
                    }

                    // Stream ended
                    if !assistant_text.is_empty() {
                        handle_final_response(&assistant_text, &mut session.messages)?;
                    }

                    process_turn_annotations(
                        session,
                        &mut turn_tool_infos,
                        &mut previous_turn_tool_infos,
                    );

                    // PROV-039: Propagate stop_reason on stream-ended (None) path
                    output.emit_done_with_stop_reason(final_stop_reason.take());
                    break;
                }
                _ => {
                    // PROV-005-DEBUG: Log unknown stream items
                    debug!("[stream_loop] Unknown stream item received (ignored)");
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

    // TOOL-011/BUG-126/RPC-398: Clear the tool progress callback using the
    // same session id it was registered under.
    set_tool_progress_callback(session_id, None);

    // CMPCT-032: Production-mode post-loop safety net.
    //
    // CMPCT-027 moved compaction-and-retry into the in-loop
    // `in_loop_compaction_restart!()` macro, but the error-arm macro only
    // fires from `stream.next()` error branches. Several clean-exit paths
    // can leave `token_state.compaction_needed == true` unhandled:
    //   - thinking-exhaustion retry that breaks with the flag set
    //   - `None` stream-end with a late-firing post-call hook
    //   - any future break path that bypasses the FinalResponse CMPCT-032
    //     check above
    //
    // Before CMPCT-032 the compaction check here was gated behind
    // `#[cfg(debug_assertions)]`, which meant release builds had NO
    // production safety net — the flag was silently dropped and the NEXT
    // user turn exploded with `prompt is too long`. This block is now
    // production-mode: if the flag reaches this point, we compact the
    // session in place so the next turn operates on the reduced context.
    //
    // Interrupts take priority — if the user cancelled, we skip recovery
    // and let the interrupt state propagate.
    //
    // The `assistant_text` buffer is already empty at this point (the loop
    // only breaks after appending it via `handle_final_response`), so
    // `begin_compaction_recovery`'s partial-text flush is a no-op; the
    // helper still flushes the token tracker and emits lifecycle events.
    let post_loop_needs_recovery = !is_interrupted.load(Acquire)
        && token_state
            .lock()
            .map(|s| s.compaction_needed)
            .unwrap_or(false);
    if post_loop_needs_recovery {
        warn!(
            "[stream_loop] CMPCT-032: POST-LOOP safety net fired — loop exited with \
             compaction_needed=true without invoking the in-loop macro. This indicates \
             a break path bypassed both the FinalResponse compaction check and the \
             error-arm in-loop macro (likely a thinking-exhaustion retry, stream-end \
             with a late-firing hook, or a new break path added without compaction \
             handling). Running compaction in place so the next user turn does not \
             exceed the context window."
        );
        let mut post_loop_text = String::new();
        match super::recovery_compaction::begin_compaction_recovery(
            session,
            &token_state,
            &streaming_display,
            &mut post_loop_text,
            output,
            false, // post-loop: no trailing User prompt to pop
        ) {
            Ok(_policy) => {
                // Compact the session in place so the next turn operates on
                // the reduced context. We cannot kick off a retry stream
                // here because the primary loop has already broken — the
                // next user message will begin a fresh `run_agent_stream`
                // invocation on the compacted session.
                if let Err(e) = super::recovery_compaction::execute_compaction_and_capture_events(
                    session,
                    compaction_in_progress.clone(),
                    prompt,
                    threshold,
                    context_window,
                    &token_state,
                    output,
                )
                .await
                {
                    warn!(
                        "[stream_loop] CMPCT-032: POST-LOOP execute_compaction failed: {e}; \
                         flag remains set — next turn may still exceed context window"
                    );
                } else {
                    // Clear the flag on success so the next turn starts clean.
                    if let Ok(mut state) = token_state.lock() {
                        state.compaction_needed = false;
                        state.input_tokens = session.token_tracker.input_tokens;
                        state.cache_read_input_tokens = 0;
                        state.cache_creation_input_tokens = 0;
                        state.output_tokens = 0;
                    }
                }
            }
            Err(e) => {
                warn!(
                    "[stream_loop] CMPCT-032: POST-LOOP begin_compaction_recovery failed: {e}; \
                     flag remains set — next turn may still exceed context window"
                );
            }
        }
    }

    // CMPCT-001: Update session token tracker with BOTH current context AND cumulative billing
    // Uses the consolidated update_from_usage method to reduce code duplication
    if !is_interrupted.load(Acquire) {
        let final_display = streaming_display.current();
        // TOKEN-001: compute per-turn delta via the single TokenTracker helper
        // BEFORE update_from_usage mutates self.output_tokens.
        let per_turn_output_delta = session
            .token_tracker
            .compute_output_delta(final_display.output_tokens);
        let final_usage = ApiTokenUsage::new(
            final_display.input_tokens,
            final_display.cache_read_tokens,
            final_display.cache_creation_tokens,
            per_turn_output_delta,
        );
        tracing::debug!(
            "CMPCT-001: Before update: cumulative_billed_input={}, final_display.input_tokens={}",
            session.token_tracker.cumulative_billed_input,
            final_display.input_tokens
        );
        session
            .token_tracker
            .update_from_usage(&final_usage, final_display.output_tokens);
        tracing::debug!(
            "CMPCT-001: After update: cumulative_billed_input={}, cumulative_billed_output={}",
            session.token_tracker.cumulative_billed_input,
            session.token_tracker.cumulative_billed_output
        );
    }

    Ok(())
}
