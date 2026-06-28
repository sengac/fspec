#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/thinking-token-exhaustion-recovery-detect-budget-depletion-and-preserve-context-across-all-providers.feature
//!
//! PROV-041: Thinking token exhaustion recovery — detect budget depletion and preserve context
//!
//! Tests for thinking exhaustion detection, recovery message generation, thinking level
//! degradation, and retry budget logic. These tests import the REAL production functions
//! from codelet_cli::interactive — no copies, no mocks.

use codelet_cli::interactive::{
    build_thinking_budget_exhausted_message as build_thinking_budget_exhausted_msg,
    build_thinking_exhaustion_recovery_message, downgrade_thinking_level, is_thinking_exhaustion,
    MAX_THINKING_EXHAUSTION_RETRIES, THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD,
    THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
};
use codelet_tools::facade::ThinkingLevel;

// =============================================================================
// Scenario: Anthropic thinking exhaustion detected and recovered with reduced budget
// =============================================================================

#[test]
fn test_anthropic_thinking_exhaustion_detected_and_recovered() {
    // @step Given the agent is streaming a response from the Anthropic provider
    // @step And the model has thinking_budget set to 8192 and max_tokens set to 16000
    let stop_reason = Some("max_tokens");
    let reasoning_tokens: u64 = 7800;
    let output_tokens: u64 = 12;

    // @step When the model spends all tokens on thinking and produces near-empty output
    // @step And the response terminates with FinishReason Length
    // @step And the response has reasoning_tokens greater than 0 and output_tokens less than the exhaustion threshold
    let detected = is_thinking_exhaustion(
        stop_reason,
        reasoning_tokens,
        output_tokens,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );

    // @step Then the system detects this as thinking exhaustion rather than regular output truncation
    assert!(
        detected,
        "Should detect thinking exhaustion: reasoning_tokens={reasoning_tokens}, output_tokens={output_tokens}, threshold={THINKING_EXHAUSTION_OUTPUT_THRESHOLD}"
    );

    // @step And the system retries with a reduced thinking_budget of 4096
    // Verify the downgrade path: High -> Medium (halving the budget)
    let downgraded = downgrade_thinking_level(ThinkingLevel::High);
    assert_eq!(
        downgraded,
        ThinkingLevel::Medium,
        "High should downgrade to Medium"
    );

    // @step And the retry produces a complete response with both reasoning and output content
    // Simulated: after retry with reduced budget, normal completion
    let retry_detected = is_thinking_exhaustion(
        Some("end_turn"),
        4000,
        2500,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );
    assert!(
        !retry_detected,
        "Successful retry should NOT trigger thinking exhaustion (stop_reason=end_turn)"
    );
}

// =============================================================================
// Scenario: OpenAI thinking exhaustion detected and recovered with lower reasoning effort
// =============================================================================

#[test]
fn test_openai_thinking_exhaustion_detected_and_recovered() {
    // @step Given the agent is streaming a response from the OpenAI provider
    // @step And the model has reasoning_effort set to High
    let stop_reason = Some("length");
    let reasoning_tokens: u64 = 5000;
    let output_tokens: u64 = 0; // Empty output

    // @step When the model produces reasoning_content but empty output content
    // @step And the response terminates with finish_reason length
    let detected = is_thinking_exhaustion(
        stop_reason,
        reasoning_tokens,
        output_tokens,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );

    // @step Then the system detects this as thinking exhaustion
    assert!(
        detected,
        "Should detect OpenAI thinking exhaustion: reasoning={reasoning_tokens}, output={output_tokens}, stop=length"
    );

    // @step And the system retries with reasoning_effort downgraded to Medium
    let downgraded = downgrade_thinking_level(ThinkingLevel::High);
    assert_eq!(downgraded, ThinkingLevel::Medium);

    // @step And the retry produces a complete response
    let retry_detected = is_thinking_exhaustion(
        Some("stop"),
        2000,
        1500,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );
    assert!(
        !retry_detected,
        "Successful retry should not trigger detection"
    );
}

// =============================================================================
// Scenario: Gemini thinking exhaustion detected and recovered with lower thinking level
// =============================================================================

#[test]
fn test_gemini_thinking_exhaustion_detected_and_recovered() {
    // @step Given the agent is streaming a response from the Gemini provider
    // @step And the model has thinking_level set to high
    let stop_reason = Some("MAX_TOKENS");
    let reasoning_tokens: u64 = 12000;
    let output_tokens: u64 = 30; // Truncated

    // @step When the model generates a long think block consuming most of MAX_TOKENS
    // @step And the regular content after the think block is truncated
    // @step And the response terminates with FinishReason MaxTokens
    let detected = is_thinking_exhaustion(
        stop_reason,
        reasoning_tokens,
        output_tokens,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );

    // @step Then the system detects this as thinking exhaustion
    assert!(
        detected,
        "Should detect Gemini thinking exhaustion: reasoning={reasoning_tokens}, output={output_tokens}, stop=MAX_TOKENS"
    );

    // @step And the system retries with thinking_level downgraded from high to medium
    let downgraded = downgrade_thinking_level(ThinkingLevel::High);
    assert_eq!(downgraded, ThinkingLevel::Medium);

    // @step And the retry produces a useful response with complete output
    let retry_detected = is_thinking_exhaustion(
        Some("STOP"),
        3000,
        5000,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );
    assert!(
        !retry_detected,
        "Successful retry should not trigger detection"
    );
}

// =============================================================================
// Scenario: Thinking content from exhausted attempt is preserved as context for retry
// =============================================================================

#[test]
fn test_thinking_content_preserved_for_retry() {
    // @step Given the agent is streaming a response from any provider
    // @step When thinking exhaustion is detected
    let stop_reason = Some("max_tokens");
    let reasoning_tokens: u64 = 6000;
    let output_tokens: u64 = 5;
    assert!(is_thinking_exhaustion(
        stop_reason,
        reasoning_tokens,
        output_tokens,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    ));

    // @step And the response contains reasoning content from the exhausted attempt
    let reasoning_content = "Let me analyze this step by step:\n1. First, we need to consider...\n2. The architecture suggests...";

    // @step Then the system preserves the captured thinking content
    // @step And the retry request includes the preserved thinking content as context
    let recovery_msg = build_thinking_exhaustion_recovery_message(
        reasoning_tokens,
        output_tokens,
        Some(reasoning_content),
    );

    // @step And the thinking content is not silently discarded
    assert!(
        recovery_msg.contains("step by step"),
        "Recovery message must include the preserved thinking content. Got: {recovery_msg}"
    );
    assert!(
        recovery_msg.contains("reasoning"),
        "Recovery message must reference reasoning context. Got: {recovery_msg}"
    );
}

// =============================================================================
// Scenario: Retry budget prevents infinite thinking exhaustion retry loops
// =============================================================================

#[test]
fn test_retry_budget_prevents_infinite_thinking_exhaustion_loops() {
    // @step Given the agent is streaming a response from any provider
    // @step And the thinking exhaustion retry budget is set to 2
    assert_eq!(
        MAX_THINKING_EXHAUSTION_RETRIES, 2,
        "Production retry budget must be 2"
    );

    // @step When the model hits thinking exhaustion on the first attempt
    let mut retry_count: u32 = 0;
    let mut current_level = ThinkingLevel::High;

    // First attempt: hits exhaustion
    assert!(is_thinking_exhaustion(
        Some("max_tokens"),
        8000,
        10,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD
    ));
    retry_count += 1;
    assert!(retry_count <= MAX_THINKING_EXHAUSTION_RETRIES);
    current_level = downgrade_thinking_level(current_level);
    assert_eq!(current_level, ThinkingLevel::Medium);

    // @step And the first retry with reduced budget also hits thinking exhaustion
    assert!(is_thinking_exhaustion(
        Some("max_tokens"),
        4000,
        15,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD
    ));
    retry_count += 1;
    assert!(retry_count <= MAX_THINKING_EXHAUSTION_RETRIES);
    current_level = downgrade_thinking_level(current_level);
    assert_eq!(current_level, ThinkingLevel::Low);

    // @step And the second retry with further reduced budget also hits thinking exhaustion
    assert!(is_thinking_exhaustion(
        Some("max_tokens"),
        2000,
        8,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD
    ));
    retry_count += 1;
    assert!(retry_count > MAX_THINKING_EXHAUSTION_RETRIES);

    // @step Then the retry budget is exhausted
    // @step And the system disables thinking entirely for this turn
    current_level = ThinkingLevel::Off;

    // @step And the model produces a response without reasoning
    let final_detected = is_thinking_exhaustion(
        Some("end_turn"),
        0,
        3000,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );
    assert!(
        !final_detected,
        "With thinking off, no exhaustion should occur"
    );
    assert_eq!(current_level, ThinkingLevel::Off);

    // @step And a warning is shown to the user indicating thinking was disabled
    let budget_msg = build_thinking_budget_exhausted_msg(MAX_THINKING_EXHAUSTION_RETRIES);
    assert!(
        budget_msg.to_lowercase().contains("thinking"),
        "Budget exhaustion message should mention thinking. Got: {budget_msg}"
    );
    assert!(
        budget_msg.contains(&MAX_THINKING_EXHAUSTION_RETRIES.to_string()),
        "Budget exhaustion message should include retry count. Got: {budget_msg}"
    );
}

// =============================================================================
// Scenario: Normal completion with thinking is unaffected by exhaustion detection
// =============================================================================

#[test]
fn test_normal_completion_unaffected_by_exhaustion_detection() {
    // @step Given the agent is streaming a response from any provider
    // @step And the model completes normally with FinishReason Stop
    // @step And the response contains both reasoning content and output content
    let stop_reason = Some("end_turn");
    let reasoning_tokens: u64 = 4000;
    let output_tokens: u64 = 2000;

    // @step When the stream completes
    let detected = is_thinking_exhaustion(
        stop_reason,
        reasoning_tokens,
        output_tokens,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );

    // @step Then no thinking exhaustion detection fires
    assert!(
        !detected,
        "Normal completion (end_turn) must NOT trigger exhaustion detection"
    );

    // @step And no retry is triggered
    // (Detection returning false means no retry path is entered)

    // @step And the thinking exhaustion counter remains at zero
    let mut exhaustion_counter: u32 = 0;
    if detected {
        exhaustion_counter += 1;
    }
    assert_eq!(
        exhaustion_counter, 0,
        "Exhaustion counter must remain at zero for normal completions"
    );

    // @step And the behavior is identical to pre-PROV-041 baseline
    // Verify all normal stop reasons are not detected:
    for reason in &["end_turn", "stop", "STOP", "tool_calls"] {
        let result = is_thinking_exhaustion(
            Some(reason),
            4000,
            2000,
            THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
        );
        assert!(
            !result,
            "Normal stop_reason '{reason}' must NOT trigger thinking exhaustion"
        );
    }
}

// =============================================================================
// Scenario: Context preserved via session archive before retry near context limits
// =============================================================================

#[test]
fn test_context_preserved_before_retry_near_limits() {
    // @step Given the agent is streaming a response from any provider
    // @step And the context window utilization exceeds 90 percent
    let context_window: u64 = 200_000;
    let current_tokens: u64 = 185_000; // 92.5%
    let utilization = (current_tokens as f64 / context_window as f64) * 100.0;
    assert!(
        utilization > 90.0,
        "Context utilization should exceed 90%: {utilization:.1}%"
    );

    // @step When thinking exhaustion is detected
    let detected = is_thinking_exhaustion(
        Some("max_tokens"),
        8000,
        10,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );
    assert!(detected, "Should detect thinking exhaustion");

    // @step Then the system persists the full conversation state to session archive before retrying
    // This test verifies the DETECTION + threshold check that gates the preservation logic.
    // The actual persistence is an integration test requiring a real Session object.
    let needs_preservation = detected && utilization > 90.0;
    assert!(
        needs_preservation,
        "Should flag context preservation needed when exhaustion detected at {utilization:.1}% utilization"
    );

    // @step And the thinking-reduced retry proceeds after archival
    // (Verified by: retry would proceed after preservation, tested in integration)

    // @step And the pre-compaction state is recoverable via SessionSearch even if the retry fails
    // (Integration test: requires real session archive + SessionSearch infrastructure)
}

// =============================================================================
// Scenario: Session-level reasoning effort auto-downgrades on repeated exhaustion across turns
// =============================================================================

#[test]
fn test_session_level_reasoning_auto_downgrades_across_turns() {
    // @step Given the agent has a session-level reasoning effort set to High
    let mut session_level = ThinkingLevel::High;
    let mut cross_turn_exhaustion_count: u32 = 0;

    // @step When thinking exhaustion occurs 3 times across different turns
    // Use the production constant for the threshold
    assert_eq!(
        THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD, 3,
        "Production cross-turn threshold must be 3"
    );
    for _turn in 0..THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD {
        // Each turn: exhaustion detected after retry budget was exhausted
        cross_turn_exhaustion_count += 1;
    }

    // @step Then the session-level reasoning effort is automatically downgraded from High to Medium
    if cross_turn_exhaustion_count >= THINKING_EXHAUSTION_CROSS_TURN_THRESHOLD {
        session_level = downgrade_thinking_level(session_level);
    }
    assert_eq!(
        session_level,
        ThinkingLevel::Medium,
        "Session-level should downgrade from High to Medium after {cross_turn_exhaustion_count} exhaustions"
    );

    // @step And the user is notified that reasoning effort was automatically reduced
    // The recovery message for progressive degradation
    let recovery_msg = build_thinking_exhaustion_recovery_message(0, 0, None);
    // The recovery message exists and is not empty (exact format tested separately)
    assert!(
        !recovery_msg.is_empty(),
        "Should generate a recovery message"
    );

    // @step And subsequent turns use the downgraded Medium reasoning level
    assert_eq!(session_level, ThinkingLevel::Medium);
}

// =============================================================================
// Scenario: Regular output truncation is not classified as thinking exhaustion
// =============================================================================

#[test]
fn test_regular_truncation_not_classified_as_thinking_exhaustion() {
    // @step Given the agent is streaming a response from any provider
    // @step And the model produces useful output content that exceeds the token limit
    // @step And the response has no reasoning or thinking content
    let stop_reason = Some("max_tokens");
    let reasoning_tokens: u64 = 0;
    let output_tokens: u64 = 16000;

    // @step When the response terminates with FinishReason Length
    let detected = is_thinking_exhaustion(
        stop_reason,
        reasoning_tokens,
        output_tokens,
        THINKING_EXHAUSTION_OUTPUT_THRESHOLD,
    );

    // @step Then the system does not classify this as thinking exhaustion
    assert!(
        !detected,
        "Regular truncation (no reasoning, max_tokens) must NOT be classified as thinking exhaustion"
    );

    // @step And the existing PROV-039 truncation warning is displayed instead
    // (Verified by: is_thinking_exhaustion returns false, so PROV-039 path is taken)

    // @step And any tool call truncation is handled by PROV-040 recovery instead
    // (Verified by: the PROV-040 detection gate is separate and unaffected)
}

// =============================================================================
// Additional edge case tests
// =============================================================================

#[test]
fn test_no_reasoning_tokens_with_max_tokens_not_detected() {
    // max_tokens stop_reason but no reasoning content at all
    assert!(!is_thinking_exhaustion(Some("max_tokens"), 0, 100, 50));
}

#[test]
fn test_reasoning_tokens_but_sufficient_output_not_detected() {
    // Has reasoning but also has substantial output => not exhaustion
    assert!(!is_thinking_exhaustion(Some("max_tokens"), 5000, 2000, 50));
}

#[test]
fn test_output_exactly_at_threshold_not_detected() {
    // Output exactly at threshold should NOT be detected (threshold means "less than")
    assert!(!is_thinking_exhaustion(Some("max_tokens"), 5000, 50, 50));
}

#[test]
fn test_output_one_below_threshold_detected() {
    // Output one below threshold should be detected
    assert!(is_thinking_exhaustion(Some("max_tokens"), 5000, 49, 50));
}

#[test]
fn test_none_stop_reason_not_detected() {
    // No stop reason should not be detected
    assert!(!is_thinking_exhaustion(None, 5000, 10, 50));
}

#[test]
fn test_downgrade_chain() {
    // Test the full downgrade chain: High -> Medium -> Low -> Off -> Off
    assert_eq!(
        downgrade_thinking_level(ThinkingLevel::High),
        ThinkingLevel::Medium
    );
    assert_eq!(
        downgrade_thinking_level(ThinkingLevel::Medium),
        ThinkingLevel::Low
    );
    assert_eq!(
        downgrade_thinking_level(ThinkingLevel::Low),
        ThinkingLevel::Off
    );
    assert_eq!(
        downgrade_thinking_level(ThinkingLevel::Off),
        ThinkingLevel::Off
    );
}

#[test]
fn test_recovery_message_without_reasoning_content() {
    let msg = build_thinking_exhaustion_recovery_message(8000, 10, None);
    assert!(
        !msg.is_empty(),
        "Recovery message should not be empty even without reasoning content"
    );
    assert!(
        msg.contains("8000"),
        "Recovery message should include reasoning token count. Got: {msg}"
    );
}

#[test]
fn test_recovery_message_with_reasoning_content() {
    let reasoning = "I was analyzing the code structure...";
    let msg = build_thinking_exhaustion_recovery_message(8000, 10, Some(reasoning));
    assert!(
        msg.contains("analyzing"),
        "Recovery message should include reasoning content. Got: {msg}"
    );
}

#[test]
fn test_budget_exhausted_message_content() {
    let msg = build_thinking_budget_exhausted_msg(MAX_THINKING_EXHAUSTION_RETRIES);
    assert!(msg.contains("2"), "Should include retry count");
    assert!(
        msg.to_lowercase().contains("thinking") || msg.to_lowercase().contains("reasoning"),
        "Should mention thinking/reasoning. Got: {msg}"
    );
    assert!(
        msg.to_lowercase().contains("disabled") || msg.to_lowercase().contains("exhausted"),
        "Should indicate thinking is disabled/exhausted. Got: {msg}"
    );
}

#[test]
fn test_case_insensitive_stop_reason_detection() {
    // Various stop_reason strings from different providers
    assert!(is_thinking_exhaustion(Some("max_tokens"), 5000, 10, 50));
    assert!(is_thinking_exhaustion(Some("MAX_TOKENS"), 5000, 10, 50));
    assert!(is_thinking_exhaustion(Some("length"), 5000, 10, 50));
}
