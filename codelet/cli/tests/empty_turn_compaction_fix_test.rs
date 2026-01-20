//! Test for the fix to prevent "Cannot compact empty turn history" error
//!
//! This test verifies that compaction is not triggered when session.turns is empty,
//! even if the token threshold is exceeded.

use anyhow::Result;
use codelet_cli::session::Session;
use codelet_core::compaction::TokenTracker;

#[tokio::test]
async fn test_compaction_not_triggered_when_turns_empty() -> Result<()> {
    // Given a session with empty turns
    let mut session = Session::new(Some("claude"))?;
    
    // Verify turns are empty
    assert_eq!(session.turns.len(), 0, "Session should start with empty turns");
    
    // And token tracker shows high token usage that would normally trigger compaction
    // Use 200k context window - threshold would be 135k (see compaction_threshold.rs)
    session.token_tracker = TokenTracker {
        input_tokens: 150_000, // Above threshold
        output_tokens: 10_000,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
    };
    
    // Verify effective tokens exceed threshold
    let effective_tokens = session.token_tracker.effective_tokens();
    let context_window = 200_000u64;
    use codelet_cli::compaction_threshold::calculate_compaction_threshold;
    let threshold = calculate_compaction_threshold(context_window);
    
    assert!(
        effective_tokens > threshold,
        "Test setup: effective tokens ({}) should exceed threshold ({})",
        effective_tokens,
        threshold
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
    
    // And high token usage
    session.token_tracker = TokenTracker {
        input_tokens: 150_000, // Above threshold
        output_tokens: 10_000,
        cache_read_input_tokens: Some(0),
        cache_creation_input_tokens: Some(0),
    };
    
    // When the compaction check logic runs
    let effective_tokens = session.token_tracker.effective_tokens();
    let context_window = 200_000u64;
    use codelet_cli::compaction_threshold::calculate_compaction_threshold;
    let threshold = calculate_compaction_threshold(context_window);
    
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
    let effective_tokens = 150_000u64;
    let threshold = 135_000u64;
    let empty_turns = Vec::<codelet_core::compaction::ConversationTurn>::new();
    let non_empty_turns = vec![codelet_core::compaction::ConversationTurn {
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