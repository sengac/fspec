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
use crate::interactive_helpers::{compression_ratio, convert_messages_to_turns, execute_compaction};
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::get_debug_capture_manager;
use codelet_common::token_estimator::count_tokens;
use codelet_core::compaction::annotation_detector::{detect_annotations, ToolCallInfo, TurnContext};
use codelet_core::{ApiTokenUsage, CompactionHook, RigAgent, TokenState, ensure_thought_signatures, GeminiTurnCompletionFacade, TurnCompletionFacade, ContinuationStrategy, StreamingTokenDisplay};
use codelet_tools::set_tool_progress_callback;
use codelet_tui::{InputQueue, StatusDisplay, TuiEvent};
use crossterm::event::KeyCode;
use futures::StreamExt;
use rig::agent::MultiTurnStreamItem;
use rig::completion::{CompletionModel, GetTokenUsage};
use rig::message::{Message, ToolResultContent, UserContent};
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

/// Check if an error indicates the prompt/context is too long
/// PROV-010: Exclude thinking budget configuration errors (budget_tokens)
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: codelet/cli/tests/prompt_too_long_recovery_test.rs
pub fn is_prompt_too_long_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();
    
    // PROV-010: Exclude thinking budget configuration errors
    // These contain "budget_tokens" and should NOT trigger compaction
    if error_lower.contains("budget_tokens") {
        return false;
    }
    
    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}

/// EXT-016: Check if an error indicates image content was rejected by the API.
///
/// Detects 400 errors related to image dimensions, image size, or image processing.
/// This is used to trigger image content sanitization in the error recovery path.
///
/// This function is public for testing.
pub fn is_image_content_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();

    // Must mention "image" in conjunction with dimension/size-related terms
    if error_lower.contains("image") {
        return error_lower.contains("dimension")
            || error_lower.contains("exceed")
            || error_lower.contains("too large")
            || error_lower.contains("max allowed size")
            || error_lower.contains("size");
    }

    false
}

/// EXT-016: Sanitize image content from conversation history.
///
/// Walks messages and replaces any Image content (UserContent::Image,
/// ToolResultContent::Image within UserContent::ToolResult) with text placeholders.
///
/// Returns `true` if any image content was replaced.
///
/// This function is public for testing.
pub fn sanitize_image_content(messages: &mut [Message]) -> bool {
    let mut replaced = false;

    for msg in messages.iter_mut().rev() {
        if let Message::User { content } = msg {
            let mut has_image = false;
            for item in content.iter() {
                match item {
                    UserContent::Image { .. } => {
                        has_image = true;
                        break;
                    }
                    UserContent::ToolResult(tool_result) => {
                        for tr_item in tool_result.content.iter() {
                            if matches!(tr_item, ToolResultContent::Image { .. }) {
                                has_image = true;
                                break;
                            }
                        }
                        if has_image {
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if has_image {
                let mut new_parts: Vec<UserContent> = Vec::new();
                for item in content.iter() {
                    match item {
                        UserContent::Image { .. } => {
                            new_parts.push(UserContent::text(
                                "[Image removed: exceeded provider pixel dimension limit]",
                            ));
                            replaced = true;
                        }
                        UserContent::ToolResult(tool_result) => {
                            // Check if tool result contains images
                            let has_tr_image = tool_result
                                .content
                                .iter()
                                .any(|i| matches!(i, ToolResultContent::Image { .. }));

                            if has_tr_image {
                                // Replace image content within tool result
                                let mut new_tr_parts: Vec<ToolResultContent> = Vec::new();
                                for tr_item in tool_result.content.iter() {
                                    match tr_item {
                                        ToolResultContent::Image { .. } => {
                                            new_tr_parts.push(ToolResultContent::text(
                                                "[Image removed: exceeded provider pixel dimension limit]",
                                            ));
                                            replaced = true;
                                        }
                                        other => {
                                            new_tr_parts.push(other.clone());
                                        }
                                    }
                                }
                                if let Ok(new_tr_content) =
                                    OneOrMany::many(new_tr_parts)
                                {
                                    // Preserve call_id if present (OpenAI provider path)
                                    if let Some(call_id) = &tool_result.call_id {
                                        new_parts.push(UserContent::tool_result_with_call_id(
                                            &tool_result.id,
                                            call_id.clone(),
                                            new_tr_content,
                                        ));
                                    } else {
                                        new_parts.push(UserContent::tool_result(
                                            &tool_result.id,
                                            new_tr_content,
                                        ));
                                    }
                                } else {
                                    new_parts.push(item.clone());
                                }
                            } else {
                                new_parts.push(item.clone());
                            }
                        }
                        other => {
                            new_parts.push(other.clone());
                        }
                    }
                }
                if let Ok(new_content) = OneOrMany::many(new_parts) {
                    *content = new_content;
                }
            }
        }
    }

    replaced
}

/// PROV-040: Maximum number of consecutive truncation recovery retries per turn.
/// After this many retries, the error is reported to the user as non-recoverable.
/// Public for testing — tests must import the real constant, not hardcode the value.
pub const MAX_TRUNCATION_RETRIES: u32 = 2;

/// PROV-040: Check if an error indicates a truncated tool call due to output token limit.
///
/// Detects the enriched error message emitted by PROV-039 in the Anthropic streaming
/// handler when `stop_reason == "max_tokens"` and a pending tool call was never closed.
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: codelet/cli/tests/truncation_recovery_test.rs
pub fn is_truncated_tool_call_error(error_str: &str) -> bool {
    error_str.contains("Tool call truncated due to output token limit")
}

/// PROV-040: Build a structured recovery message for a truncated tool call.
///
/// Extracts the tool name from the error message and generates guidance telling
/// the model to use an alternative strategy for large content.
///
/// This function is public for testing.
pub fn build_truncation_recovery_message(error_str: &str) -> String {
    // Extract tool name from the error message: "Tool '...' received incomplete JSON arguments"
    let tool_name = error_str
        .split("Tool '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .unwrap_or("unknown");

    // Extract partial arguments from the error: "Partial arguments: ..."
    let partial_args = error_str
        .split("Partial arguments: ")
        .nth(1)
        .unwrap_or("(not available)");

    format!(
        "Your {tool_name} tool call was truncated because it exceeded the output token limit. \
         The model hit max_tokens while generating the tool call arguments.\n\n\
         Truncated tool: {tool_name}\n\
         Partial arguments received: {partial_args}\n\n\
         IMPORTANT: You MUST use a different strategy to accomplish this task:\n\
         1. Use the Bash tool with cat/heredoc to write large files: \
            Bash(command='cat << '\"'\"'HEREDOC_EOF'\"'\"' > /path/to/file\\n...content...\\nHEREDOC_EOF')\n\
         2. Split the content into multiple smaller Write calls \
            (write the first portion, then use Edit to append the rest)\n\
         3. For very large generated content, use Bash with echo/printf commands\n\n\
         Do NOT retry the same large {tool_name} call — it will be truncated again."
    )
}

/// PROV-040: Build the error message displayed when the truncation retry budget is exhausted.
///
/// This function is public for testing.
pub fn build_truncation_budget_exhausted_message(max_retries: u32) -> String {
    format!(
        "Tool call truncated {} times — retry budget exhausted. \
         The content is too large for a single tool call. \
         Use Bash with heredoc/echo to write large files, \
         or split into multiple smaller operations.",
        max_retries
    )
}

// =============================================================================
// PROV-041: Thinking token exhaustion recovery
// =============================================================================

/// PROV-041: Maximum number of consecutive thinking exhaustion retries per turn.
/// After this many retries, thinking is disabled entirely for the turn.
/// Public for testing — tests must import the real constant, not hardcode the value.
pub const MAX_THINKING_EXHAUSTION_RETRIES: u32 = 2;

/// PROV-041: Output token threshold below which a response is considered "near-empty".
/// If a response terminates with FinishReason::Length AND has reasoning_tokens > 0
/// AND output_tokens < this threshold, it's classified as thinking exhaustion.
/// Public for testing — tests must import the real constant, not hardcode the value.
pub const THINKING_EXHAUSTION_OUTPUT_THRESHOLD: u64 = 50;

/// PROV-041: Threshold for session-level progressive degradation across turns.
/// After this many thinking exhaustion events across different turns (not retries),
/// the session-level reasoning effort is automatically downgraded.
/// Public for testing — tests must import the real constant, not hardcode the value.
pub const THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD: u32 = 3;

/// PROV-041: Detect whether a response represents thinking token exhaustion.
///
/// Thinking exhaustion occurs when the model spent most/all of its token budget on
/// reasoning/thinking and produced little or no useful output. This is distinct from
/// regular output truncation (PROV-039/PROV-040) where the model produces useful content
/// that simply exceeds the token limit.
///
/// Detection heuristic:
/// - stop_reason indicates length/max_tokens (case-insensitive)
/// - reasoning_tokens > 0 (model was actually thinking)
/// - output_tokens < threshold (model said almost nothing)
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: codelet/cli/tests/thinking_exhaustion_recovery_test.rs
pub fn is_thinking_exhaustion(
    stop_reason: Option<&str>,
    reasoning_tokens: u64,
    output_tokens: u64,
    threshold: u64,
) -> bool {
    // Must have a stop_reason indicating length/truncation
    let Some(reason) = stop_reason else {
        return false;
    };

    let reason_lower = reason.to_lowercase();
    let is_length_stop = reason_lower == "max_tokens"
        || reason_lower == "length";

    if !is_length_stop {
        return false;
    }

    // Must have reasoning tokens (model was actually thinking)
    if reasoning_tokens == 0 {
        return false;
    }

    // Output must be below threshold (model said almost nothing)
    output_tokens < threshold
}

/// PROV-041: Build a recovery message for thinking exhaustion.
///
/// Generates a message to inject into the conversation that:
/// 1. Preserves any captured thinking content as context
/// 2. Instructs the model to be more concise in reasoning
/// 3. Indicates the thinking budget has been reduced
///
/// This function is public for testing.
pub fn build_thinking_exhaustion_recovery_message(
    reasoning_tokens: u64,
    output_tokens: u64,
    captured_reasoning: Option<&str>,
) -> String {
    let mut msg = format!(
        "Your previous response was interrupted because you spent too many tokens on reasoning \
         ({reasoning_tokens} reasoning tokens, only {output_tokens} output tokens). \
         Your thinking budget has been reduced for this retry.\n\n\
         IMPORTANT: Be more concise in your reasoning. Focus on producing useful output \
         rather than extensive internal deliberation."
    );

    if let Some(reasoning) = captured_reasoning {
        // Truncate very long reasoning to avoid bloating the context
        let truncated = if reasoning.len() > 2000 {
            &reasoning[..2000]
        } else {
            reasoning
        };
        msg.push_str(&format!(
            "\n\nYour previous reasoning (preserved as context):\n{truncated}"
        ));
    }

    msg
}

/// PROV-041: Build the message displayed when thinking exhaustion retry budget is exhausted.
///
/// This function is public for testing.
pub fn build_thinking_budget_exhausted_message(max_retries: u32) -> String {
    format!(
        "Thinking exhaustion occurred {max_retries} times — retry budget exhausted. \
         Thinking/reasoning has been disabled for this turn to produce a response. \
         The model will respond without extended reasoning."
    )
}

/// PROV-041: Downgrade a ThinkingLevel by one step.
///
/// Degradation path: High → Medium → Low → Off → Off
/// Used for both per-turn retry degradation and session-level progressive degradation.
///
/// This function is public for testing.
pub fn downgrade_thinking_level(level: codelet_tools::facade::ThinkingLevel) -> codelet_tools::facade::ThinkingLevel {
    use codelet_tools::facade::ThinkingLevel;
    match level {
        ThinkingLevel::High => ThinkingLevel::Medium,
        ThinkingLevel::Medium => ThinkingLevel::Low,
        ThinkingLevel::Low => ThinkingLevel::Off,
        ThinkingLevel::Off => ThinkingLevel::Off,
    }
}

/// CMPCT-002: Check if an error indicates compaction was cancelled by the hook
/// This is used to detect when the CompactionHook cancels a request due to token threshold
fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    error.to_string().contains("PromptCancelled")
}

/// CMPCT-002: Signal that compaction is needed by setting the flag in token state
/// This allows the post-loop compaction logic to detect and handle it
fn signal_compaction_needed(token_state: &Arc<Mutex<TokenState>>) {
    // PROV-009-DEBUG: Log when signal_compaction_needed is called with backtrace
    debug!(
        "[signal_compaction_needed] CALLED - setting compaction_needed=true - BACKTRACE:\n{:?}",
        std::backtrace::Backtrace::capture()
    );
    if let Ok(mut state) = token_state.lock() {
        state.compaction_needed = true;
    }
}

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
    )
    .await
}

/// BRIDGE-007: Image data from bridge for multimodal support
#[derive(Clone)]
pub struct BridgeImage {
    /// Base64-encoded image data
    pub data: String,
    /// Media type (e.g., "image/jpeg", "image/png")
    pub media_type: String,
}

/// EXT-016: Build user message content from prompt text and optional bridge images.
///
/// Validates pixel dimensions on each image before including it. Oversized images
/// are replaced with a text error message (Layer 3 defense-in-depth).
///
/// This function is public for testing.
pub fn build_user_content_with_images(
    prompt: &str,
    images: Option<Vec<BridgeImage>>,
) -> OneOrMany<UserContent> {
    use rig::message::ImageMediaType;

    match images {
        Some(bridge_images) => {
            let mut content_parts: Vec<UserContent> = Vec::new();
            if !prompt.is_empty() {
                content_parts.push(UserContent::text(prompt));
            }
            for img in bridge_images {
                let media_type = match img.media_type.as_str() {
                    "image/jpeg" | "image/jpg" => Some(ImageMediaType::JPEG),
                    "image/png" => Some(ImageMediaType::PNG),
                    "image/gif" => Some(ImageMediaType::GIF),
                    "image/webp" => Some(ImageMediaType::WEBP),
                    _ => Some(ImageMediaType::JPEG),
                };

                // EXT-016: Validate pixel dimensions before adding user-pasted images
                // This is Layer 3 defense-in-depth — prevents oversized bridge images
                // from entering conversation history
                if let Some((width, height)) =
                    codelet_tools::image_dimensions::extract_dimensions_from_base64(&img.data)
                {
                    if codelet_tools::image_dimensions::exceeds_pixel_limit(width, height) {
                        let error_msg =
                            codelet_tools::image_dimensions::format_dimension_error(None, width, height);
                        warn!(
                            "Rejecting user-pasted image: {}x{} exceeds limit",
                            width, height
                        );
                        // Add error as text instead of the image
                        content_parts.push(UserContent::text(error_msg));
                        continue;
                    }
                }

                content_parts.push(UserContent::image_base64(img.data, media_type, None));
            }
            OneOrMany::many(content_parts)
                .unwrap_or_else(|_| OneOrMany::one(UserContent::text(prompt)))
        }
        None => OneOrMany::one(UserContent::text(prompt)),
    }
}

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
    // CTX-002: Use usable_context (context_window - output_reservation) instead of 90% threshold
    let threshold = calculate_usable_context(context_window, max_output_tokens);

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
            estimated_total, threshold
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
    if let Some(emitter) = output.progress_emitter() {
        set_tool_progress_callback(Some(Arc::new(move |chunk: &str, is_stderr: bool| {
            emitter.emit_tool_progress("", "bash", chunk, is_stderr);
        })));
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
    let mut tool_calls_buffer: Vec<rig::message::AssistantContent> = Vec::new();
    let mut last_tool_name: Option<String> = None;

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

                    turn_tool_infos.push(ToolCallInfo {
                        tool_name: tool_call.function.name.clone(),
                        input: tool_call.function.arguments.clone(),
                        output: None,
                        success: true, // Assume success until result arrives
                    });
                }
                Some(Ok(MultiTurnStreamItem::StreamAssistantItem(
                    StreamedAssistantContent::ReasoningDelta { reasoning, .. },
                ))) => {
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
                    if let Some(info) = turn_tool_infos.iter_mut().rev().find(|ti| {
                        ti.output.is_none()
                    }) {
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
                            ).with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
                            emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);
                        }
                    }
                }
                Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
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
                    ).with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
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
                                debug!(
                                    "API REQUEST (Gemini continuation) - Provider: {}, Model: {}",
                                    session.current_provider_name(),
                                    session.current_model_id().as_deref().unwrap_or("NONE")
                                );
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
                                        // PROV-039: Propagate stop_reason on Gemini continuation interrupt
                                        output.emit_done_with_stop_reason(final_stop_reason.take());
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

                                                debug!(
                                                    "API REQUEST (Gemini nested continuation) - Provider: {}, Model: {}",
                                                    session.current_provider_name(),
                                                    session.current_model_id().as_deref().unwrap_or("NONE")
                                                );
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
                                                
                                                // UX-002: Use structured compaction event
                                                output.emit_compaction_started();
                                                
                                                // UX-002: Emit progress for continuation compaction
                                                let total_turns = session.messages.len() as u32 / 2;
                                                output.emit_compaction_progress("Context limit reached", 0, total_turns.max(1));
                                                
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
                                // PROV-039: Propagate stop_reason on Gemini continuation completion
                                output.emit_done_with_stop_reason(final_stop_reason.take());
                                return Ok(());
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
                                && session.thinking_exhaustion_cross_turn_count >= THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD
                            {
                                session.session_thinking_level = downgrade_thinking_level(session.session_thinking_level);
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
                                    "Thinking exhaustion detected (attempt {}/{}). Retrying with reduced reasoning budget...",
                                    thinking_exhaustion_retry_count,
                                    MAX_THINKING_EXHAUSTION_RETRIES
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
                                        "Context window at {:.0}% — session state preserved before retry",
                                        utilization_pct
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
                                let retry_hook = CompactionHook::new(Arc::clone(&retry_token_state), threshold);

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

                                // STREAMING-DISPLAY: Reset display for retry
                                streaming_display = StreamingTokenDisplay::new(
                                    session.token_tracker.input_tokens,
                                    session.token_tracker.output_tokens,
                                    session.token_tracker.cache_read_input_tokens.unwrap_or(0),
                                    session.token_tracker.cache_creation_input_tokens.unwrap_or(0),
                                );

                                debug!("[stream_loop] PROV-041: Created new stream for thinking exhaustion retry");
                                continue;
                            } else {
                                // Budget exhausted — emit warning and continue with best available response
                                let budget_msg = build_thinking_budget_exhausted_message(MAX_THINKING_EXHAUSTION_RETRIES);
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
                    
                    // CMPCT-002: Check if this error is due to compaction hook cancellation
                    // using the helper function for DRY compliance
                    let is_compaction_cancel = is_compaction_cancelled(&e);

                    // Check if compaction was actually triggered by the hook
                    let compaction_triggered = token_state
                        .lock()
                        .map(|state| state.compaction_needed)
                        .unwrap_or(false);

                    // PROV-009-DEBUG: Log error classification
                    debug!(
                        "[stream_loop] Error classification: is_compaction_cancel={}, compaction_triggered={}",
                        is_compaction_cancel,
                        compaction_triggered
                    );

                    if is_compaction_cancel && compaction_triggered {
                        // This is a compaction cancellation - break to run compaction logic
                        // Don't log as error, this is expected behavior
                        debug!("[stream_loop] Breaking due to compaction cancellation (expected)");
                        break;
                    }

                    // Check if this is a "prompt is too long" error from the API
                    let error_str = e.to_string();
                    let is_prompt_too_long = is_prompt_too_long_error(&error_str);

                    // PROV-010: Only trigger compaction if there are actual user/assistant turns to compact
                    // session.messages may contain system prompts but no compactable turns
                    let has_compactable_turns = !convert_messages_to_turns(&session.messages).is_empty();
                    
                    if is_prompt_too_long && has_compactable_turns {
                        info!("Received 'prompt is too long' error, triggering recovery compaction");
                        // UX-002: Use structured compaction event instead of string status
                        output.emit_compaction_started();
                        
                        // UX-002: Emit progress for emergency compaction
                        let total_turns = session.messages.len() as u32 / 2;
                        output.emit_compaction_progress("Emergency compaction", 0, total_turns.max(1));

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
                            let retry_hook = CompactionHook::new(Arc::clone(&retry_token_state), threshold);

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

                            // STREAMING-DISPLAY: Reset display for retry
                            streaming_display = StreamingTokenDisplay::new(
                                session.token_tracker.input_tokens,
                                session.token_tracker.output_tokens,
                                session.token_tracker.cache_read_input_tokens.unwrap_or(0),
                                session.token_tracker.cache_creation_input_tokens.unwrap_or(0),
                            );

                            continue;
                        }

                        // Retry budget exhausted — report to user and terminate
                        warn!(
                            "PROV-040: Truncation retry budget exhausted after {} attempts",
                            MAX_TRUNCATION_RETRIES
                        );
                        let budget_error = build_truncation_budget_exhausted_message(MAX_TRUNCATION_RETRIES);
                        output.emit_error(&budget_error);
                        return Err(anyhow::anyhow!("Agent error: truncation retry budget exhausted after {} attempts", MAX_TRUNCATION_RETRIES));
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

    // TOOL-011: Clear the tool progress callback
    set_tool_progress_callback(None);

    // Check if hook triggered compaction
    let compaction_needed = token_state
        .lock()
        .map(|state| state.compaction_needed)
        .unwrap_or(false);

    // PROV-005-DEBUG: Log post-loop compaction state
    debug!(
        "[stream_loop] POST-LOOP: compaction_needed={}, is_interrupted={}",
        compaction_needed,
        is_interrupted.load(Acquire)
    );

    if compaction_needed && !is_interrupted.load(Acquire) {
        // PROV-005-DEBUG: Log entry to compaction block
        debug!(
            "[stream_loop] ENTERING compaction block - messages_len={}, approx_turns={}",
            session.messages.len(),
            session.messages.len() / 2
        );
        // UX-002: Use structured compaction event - this triggers both CLI display and NAPI state change
        output.emit_compaction_started();
        
        // UX-002: Emit progress for automatic compaction
        let total_turns = session.messages.len() as u32 / 2; // Approximate turn count
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

        match execute_compaction(session, compaction_in_progress.clone(), Some(prompt)).await {
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
                debug!(
                    "API REQUEST (retry after compaction) - Provider: {}, Model: {}",
                    session.current_provider_name(),
                    session.current_model_id().as_deref().unwrap_or("NONE")
                );
                // Use synthetic continuation prompt — the original prompt is already
                // embedded in the compaction instruction in session.messages.
                let mut retry_stream = agent
                    .prompt_streaming_with_history_and_hook(
                        "Continue",
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
                        // PROV-039: Propagate stop_reason even on retry-interrupt path
                        output.emit_done_with_stop_reason(None);
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
                                ).with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
                                emit_context_fill_from_usage(output, &fill_usage, threshold, context_window);
                            }
                        }
                        Some(Ok(MultiTurnStreamItem::FinalResponse(final_resp))) => {
                            // PROV-039: Capture stop_reason from retry FinalResponse
                            let retry_stop_reason = final_resp.stop_reason().map(String::from);

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
                            ).with_reasoning_tokens(usage.reasoning_tokens.unwrap_or(0));
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
                            // Done is emitted here; CompactionComplete comes from
                            // agent_loop after apply_pending_dag succeeds.
                            // PROV-039: Propagate stop_reason from retry stream
                            output.emit_done_with_stop_reason(retry_stop_reason);
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
                            // Done is emitted here; CompactionComplete comes from
                            // agent_loop after apply_pending_dag succeeds.
                            // PROV-039: retry_stop_reason may not be set if stream ended
                            // without FinalResponse — emit None (will default to end_turn)
                            output.emit_done_with_stop_reason(None);
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
                // UX-002: Use structured compaction failed event
                output.emit_compaction_failed(&format!("{e} - will retry on next turn"));

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
