//! PROV-041: Thinking token exhaustion recovery.
//!
//! Handles detection of thinking/reasoning token exhaustion, recovery message
//! generation, progressive degradation of thinking levels, and retry budget management.

use codelet_tools::facade::ThinkingLevel;

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
pub fn downgrade_thinking_level(level: ThinkingLevel) -> ThinkingLevel {
    match level {
        ThinkingLevel::High => ThinkingLevel::Medium,
        ThinkingLevel::Medium => ThinkingLevel::Low,
        ThinkingLevel::Low => ThinkingLevel::Off,
        ThinkingLevel::Off => ThinkingLevel::Off,
    }
}
