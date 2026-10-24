//! PROV-041: Thinking token exhaustion recovery.
//!
//! Handles detection of thinking/reasoning token exhaustion, recovery message
//! generation, progressive degradation of thinking levels, and retry budget management.

use codelet_tools::facade::ThinkingLevel;

/// PROV-041: Maximum number of consecutive thinking exhaustion retries per turn.
/// After this many prompt-only retries, recovery stops with the available partial response.
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
/// real function, NOT a copy. See: rust/cli/tests/thinking_exhaustion_recovery_test.rs
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
    let is_length_stop = reason_lower == "max_tokens" || reason_lower == "length";

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

pub(super) fn is_thinking_only_exhaustion(
    stop_reason: Option<&str>,
    captured_reasoning: &str,
    visible_output: &str,
    has_tool_progress: bool,
) -> bool {
    !has_tool_progress
        && !captured_reasoning.trim().is_empty()
        && visible_output.trim().is_empty()
        && stop_reason.is_some_and(|reason| {
            reason.eq_ignore_ascii_case("length") || reason.eq_ignore_ascii_case("max_tokens")
        })
}

/// PROV-041: Build a recovery message for thinking exhaustion.
///
/// Generates a message to inject into the conversation that:
/// 1. Preserves any captured thinking content as context
/// 2. Instructs the model to be more concise in reasoning
/// 3. Explains that model settings and token limits are unchanged
///
/// This function is public for testing.
pub fn build_thinking_exhaustion_recovery_message(
    reasoning_tokens: u64,
    output_tokens: u64,
    captured_reasoning: Option<&str>,
) -> String {
    let mut msg = format!(
        "Your previous response reached its token limit before producing enough answer content. \
         Recorded usage counters: {reasoning_tokens} reasoning tokens, {output_tokens} output tokens. \
         This is a prompt-only retry; model settings and token limits are unchanged.\n\n\
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
        "Thinking exhaustion retry budget exhausted after {max_retries} retries. \
         No further recovery requests will be sent; model settings and token limits are unchanged. \
         Keeping the available partial response."
    )
}

/// PROV-041: Downgrade a ThinkingLevel by one step.
///
/// Degradation path: High → Medium → Low → Off → Off
/// Used for session-level progressive degradation on subsequent turns.
///
/// This function is public for testing.
pub fn downgrade_thinking_level(level: ThinkingLevel) -> ThinkingLevel {
    match level {
        ThinkingLevel::High => ThinkingLevel::Medium,
        ThinkingLevel::Medium => ThinkingLevel::Low,
        ThinkingLevel::Low => ThinkingLevel::Off,
        ThinkingLevel::Off => ThinkingLevel::Off,
    }
}
