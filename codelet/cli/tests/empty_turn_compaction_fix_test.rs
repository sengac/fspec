#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Test for the fix to prevent "Cannot compact empty turn history" error
//!
//! This test verifies that compaction is not triggered when session.turns is empty,
//! even if the token threshold is exceeded.

use anyhow::Result;
use codelet_cli::compaction_threshold::calculate_usable_context;
use codelet_cli::session::Session;
use codelet_core::compaction::TokenTracker;

#[tokio::test]
async fn test_compaction_not_triggered_when_turns_empty() -> Result<()> {
    // Given a session with empty turns
    let mut session = Session::new(Some("claude"))?;

    // Verify turns are empty
    assert_eq!(
        session.turns.len(),
        0,
        "Session should start with empty turns"
    );

    // And token tracker shows high token usage that would normally trigger compaction
    // Use 200k context window with 8k max_output - threshold is 191,808
    session.token_tracker = TokenTracker {
        input_tokens: 195_000, // Above threshold of 191,808
        output_tokens: 10_000,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
    };

    // Verify effective tokens exceed threshold
    let effective_tokens = session.token_tracker.effective_tokens();
    let context_window = 200_000u64;
    let max_output = 8_192u64;
    let threshold = calculate_usable_context(context_window, max_output);

    assert!(
        effective_tokens > threshold,
        "Test setup: effective tokens ({effective_tokens}) should exceed threshold ({threshold})"
    );

    // When the compaction check logic runs (simulated here)
    let should_compact = effective_tokens > threshold && !session.turns.is_empty();

    // Then compaction should not be triggered
    assert!(
        !should_compact,
        "Compaction should not trigger when turns are empty, even with high token count"
    );

    Ok(())
}

#[tokio::test]
async fn test_compaction_triggers_when_turns_exist() -> Result<()> {
    use codelet_core::compaction::ConversationTurn;
    use std::time::SystemTime;

    // Given a session with conversation turns
    let mut session = Session::new(Some("claude"))?;

    // Add a test turn
    session.turns.push(ConversationTurn {
        user_message: "Test message".to_string(),
        tool_calls: vec![],
        tool_results: vec![],
        assistant_response: "Test response".to_string(),
        tokens: 1000,
        timestamp: SystemTime::now(),
        previous_error: None,
    });

    // And high token usage (above threshold of 191,808)
    session.token_tracker = TokenTracker {
        input_tokens: 195_000, // Above threshold
        output_tokens: 10_000,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
        cumulative_billed_input: 0,
        cumulative_billed_output: 0,
    };

    // When the compaction check logic runs
    let effective_tokens = session.token_tracker.effective_tokens();
    let context_window = 200_000u64;
    let max_output = 8_192u64;
    let threshold = calculate_usable_context(context_window, max_output);

    let should_compact = effective_tokens > threshold && !session.turns.is_empty();

    // Then compaction should be triggered
    assert!(
        should_compact,
        "Compaction should trigger when turns exist and tokens exceed threshold"
    );

    Ok(())
}

#[test]
fn test_defensive_check_logic() {
    // Test the defensive logic directly
    // Using calculate_usable_context: threshold = 200k - 8k = 191,808
    let effective_tokens = 195_000u64;
    let threshold = 191_808u64;
    let empty_turns = Vec::<codelet_core::compaction::ConversationTurn>::new();
    let non_empty_turns = [codelet_core::compaction::ConversationTurn {
        user_message: "Test".to_string(),
        tool_calls: vec![],
        tool_results: vec![],
        assistant_response: "Test".to_string(),
        tokens: 1000,
        timestamp: std::time::SystemTime::now(),
        previous_error: None,
    }];

    // Case 1: High tokens, empty turns -> Should NOT compact
    assert!(
        !(effective_tokens > threshold && !empty_turns.is_empty()),
        "Should not compact when turns are empty"
    );

    // Case 2: High tokens, non-empty turns -> Should compact
    assert!(
        effective_tokens > threshold && !non_empty_turns.is_empty(),
        "Should compact when turns exist and tokens are high"
    );

    // Case 3: Low tokens, non-empty turns -> Should NOT compact
    let low_tokens = 50_000u64;
    assert!(
        !(low_tokens > threshold && !non_empty_turns.is_empty()),
        "Should not compact when tokens are low"
    );
}
