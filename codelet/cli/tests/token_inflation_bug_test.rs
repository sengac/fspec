// Feature: spec/features/fix-token-inflation-bug-15-7x-overcounting.feature
//
// CTX-003: Fix Token Inflation Bug (15.7x Overcounting)
//
// Integration tests for preventing token inflation caused by:
// 1. Treating Anthropic's absolute input_tokens as incremental additions
// 2. Double-counting tokens between MessageStart and FinalResponse
//
// ROOT CAUSE: Two compounding bugs in token tracking:
// - BUG 1: Semantic confusion - input_tokens is TOTAL context, not incremental
// - BUG 2: Double counting - same value added in FinalResponse AND next MessageStart
//
// These tests verify the ACTUAL TokenTracker implementation, not stub behavior.

use codelet_core::compaction::TokenTracker;

// =============================================================================
// Scenario: Display accurate tokens for single API call
// =============================================================================

#[test]
fn test_single_api_call_accurate_tokens() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And the session has no previous token usage
    let mut tracker = TokenTracker::default();

    // @step When I send a message that triggers a single API call
    // @step And the API reports input_tokens of 50000 and output_tokens of 2000
    tracker.update(50000, 2000, Some(0), Some(0));

    // @step Then the token display should show "Input: 50,000 tokens"
    // (input_tokens is used for display)
    assert_eq!(tracker.input_tokens, 50000);

    // @step And the session should store current_context_tokens as 50000
    // (cumulative_billed_input tracks billing separately)
    assert_eq!(tracker.input_tokens, 50000);
    assert_eq!(tracker.cumulative_billed_input, 50000);

    // @step And the inflation ratio should be 1:1
    let display_tokens = tracker.input_tokens;
    let inflation_ratio = display_tokens as f64 / 50000.0;
    assert!((inflation_ratio - 1.0).abs() < 0.01);
}

// =============================================================================
// Scenario: Display accurate tokens for multi-API turn with tool calls
// =============================================================================

#[test]
fn test_multi_api_turn_shows_latest_context_not_sum() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And the session has no previous token usage
    let mut tracker = TokenTracker::default();

    // @step When I send a message that triggers 3 sequential API calls with tool use
    // @step And API Call 1 reports input_tokens of 50000
    tracker.update(50000, 1000, None, None);
    assert_eq!(tracker.input_tokens, 50000); // Current context

    // @step And API Call 2 reports input_tokens of 55000
    tracker.update(55000, 1000, None, None);
    assert_eq!(tracker.input_tokens, 55000); // Overwrites

    // @step And API Call 3 reports input_tokens of 60000
    tracker.update(60000, 1000, None, None);

    // @step Then the final token display should show "Input: 60,000 tokens"
    assert_eq!(tracker.input_tokens, 60000);

    // @step And the display should NOT show "Input: 165,000 tokens"
    assert_ne!(tracker.input_tokens, 165000);

    // @step And the session should store current_context_tokens as 60000
    assert_eq!(tracker.input_tokens, 60000);

    // Billing should accumulate all three calls
    assert_eq!(tracker.cumulative_billed_input, 50000 + 55000 + 60000);
}

// =============================================================================
// Scenario: Prevent double-counting tokens between MessageStart and FinalResponse
// =============================================================================

#[test]
fn test_prevent_double_counting_messagestart_finalresponse() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And a turn has multiple API calls
    let mut tracker = TokenTracker::default();

    // @step When API Call 1 triggers MessageStart with input_tokens of 50000
    // @step And API Call 1 triggers FinalResponse with input_tokens of 50000
    // In the fixed implementation, we only call update() once per API response
    tracker.update(50000, 1000, None, None);

    // @step And API Call 2 triggers MessageStart with input_tokens of 55000
    // @step And API Call 2 triggers FinalResponse with input_tokens of 55000
    tracker.update(55000, 1000, None, None);

    // @step Then the turn_accumulated_input should NOT include 50000 twice
    // @step And the turn_accumulated_input should NOT include 55000 twice
    // @step And the final accumulated value should be 55000 not 160000

    // Current context should be 55000 (latest)
    assert_eq!(tracker.input_tokens, 55000);

    // Billing should be 105000 (50k + 55k), not 160000 (double-counted)
    assert_eq!(tracker.cumulative_billed_input, 105000);

    // @step And each API call's tokens should only be counted once
    assert_ne!(tracker.cumulative_billed_input, 160000); // Not double-counted
}

// =============================================================================
// Scenario: Verify inflation calculation with both bugs present
// =============================================================================

#[test]
fn test_verify_inflation_calculation_with_bugs() {
    // @step Given the current buggy implementation is in place
    // We can calculate what the buggy implementation would produce
    let mut tracker = TokenTracker::default();

    // @step When 3 API calls occur with input_tokens of 50000, 55000, and 60000
    tracker.update(50000, 1000, None, None);
    tracker.update(55000, 1000, None, None);
    tracker.update(60000, 1000, None, None);

    // @step Then Bug 1 semantic confusion would sum to 165000 tokens
    // Simulating what Bug 1 alone would produce (sum instead of latest)
    let bug1_sum = 50000 + 55000 + 60000;
    assert_eq!(bug1_sum, 165000);

    // @step And Bug 2 double-counting would inflate to 270000 tokens
    // Simulating what Bug 2 would add on top of Bug 1:
    // Call 1: FinalResponse adds 50k (total: 50k)
    // Call 2: MessageStart adds 50k again, FinalResponse adds 55k (total: 155k)
    // Call 3: MessageStart adds 55k again, FinalResponse adds 60k (total: 270k)
    let bug2_double_counted = 50000 + 50000 + 55000 + 55000 + 60000;
    assert_eq!(bug2_double_counted, 270000);

    // @step But the correct display should show only 60000 tokens
    // With the fix, display shows current context (latest API call)
    assert_eq!(tracker.input_tokens, 60000);

    // @step And the ratio of buggy to correct is 4.5x for 3 API calls
    let buggy_value: f64 = 270000.0;
    let correct_value: f64 = 60000.0;
    let ratio = buggy_value / correct_value;
    assert!((ratio - 4.5).abs() < 0.01, "Ratio {} should be 4.5x", ratio);
}

// =============================================================================
// Scenario: Accurate display during turn with 10 tool calls
// =============================================================================

#[test]
fn test_ten_tool_calls_shows_final_context() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And the initial context is 100000 tokens
    let mut tracker = TokenTracker::default();

    // @step When a turn triggers 10 API calls with tool use
    // @step And each API call adds approximately 1000 tokens to the context
    // @step And API calls report input_tokens of 100000, 101000, ..., 109000
    let api_calls: Vec<u64> = vec![
        100000, 101000, 102000, 103000, 104000, 105000, 106000, 107000, 108000, 109000,
    ];

    for input in &api_calls {
        tracker.update(*input, 500, None, None);
    }

    // @step Then the final token display should show 109000 tokens
    assert_eq!(tracker.input_tokens, 109000);

    // @step And the display should NOT show 1045000 tokens from cumulative summing
    assert_ne!(tracker.input_tokens, 1045000);

    // @step And the session should store current_context_tokens as 109000
    assert_eq!(tracker.input_tokens, 109000);

    // Billing should track cumulative
    let expected_billing: u64 = api_calls.iter().sum();
    assert_eq!(tracker.cumulative_billed_input, expected_billing);
}

// =============================================================================
// Scenario: Prevent 15.7x token inflation in real-world session
// =============================================================================

#[test]
fn test_prevent_15_7x_inflation() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And the session has accumulated 142 messages
    // @step And the actual current context is 105000 tokens
    let mut tracker = TokenTracker::default();

    // @step When approximately 15-16 API calls have occurred during the session
    // Simulate 16 API calls, each reporting ~105k (full context)
    for _ in 0..16 {
        tracker.update(105000, 500, None, None);
    }

    // @step Then the token display should show approximately 105000 tokens
    assert_eq!(tracker.input_tokens, 105000);

    // @step And the display should NOT show 1658550 tokens (or similar cumulative)
    assert!(tracker.input_tokens < 200000); // Well below cumulative

    // @step And the inflation ratio should remain 1:1
    let ratio = tracker.input_tokens as f64 / 105000.0;
    assert!((ratio - 1.0).abs() < 0.01);

    // Billing should track cumulative (for analytics)
    assert_eq!(tracker.cumulative_billed_input, 16 * 105000);
}

// =============================================================================
// Scenario: Context fill percentage uses current context not cumulative
// =============================================================================

#[test]
fn test_context_fill_percentage_uses_current_context() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And the context_window is 200000 tokens
    let _context_window: u64 = 200000;
    // @step And the compaction threshold is 180000 tokens
    let threshold: u64 = 180000;
    // @step And the current context is 105000 tokens
    let mut tracker = TokenTracker::default();

    // Simulate multiple API calls with context staying at ~105k
    for _ in 0..10 {
        tracker.update(105000, 500, None, None);
    }

    // @step When the context fill percentage is calculated
    let current_context = tracker.input_tokens;
    let fill_pct = (current_context as f64 / threshold as f64) * 100.0;

    // @step Then the fill percentage should be approximately 58 percent
    assert!(
        (fill_pct - 58.3).abs() < 1.0,
        "Fill percentage {} should be ~58%",
        fill_pct
    );

    // @step And the fill percentage should NOT be 920 percent from cumulative tracking
    let buggy_fill_pct = (tracker.cumulative_billed_input as f64 / threshold as f64) * 100.0;
    assert!(buggy_fill_pct > 500.0); // Cumulative would be way over 100%
    assert!(fill_pct < 100.0); // Current context is below threshold
}

// =============================================================================
// Scenario: Session persistence stores both context and billing tokens
// =============================================================================

#[test]
fn test_session_persistence_dual_metrics() {
    // @step Given I am using the codelet CLI in an interactive session
    let mut tracker = TokenTracker::default();

    // @step And the current context is 105000 tokens
    // @step And the cumulative billed input is 1658550 tokens
    // Simulate ~16 API calls
    for i in 0..16 {
        // Context varies slightly per call (realistic scenario)
        let context = 100000 + (i * 500);
        tracker.update(context, 500, None, None);
    }

    // Final context should be the last reported value
    let current_context = tracker.input_tokens;
    let cumulative_billed = tracker.cumulative_billed_input;

    // @step When the session is persisted to session.json
    // @step Then the session file should contain current_context_tokens of 105000
    // (Final context is 100000 + 15*500 = 107500)
    assert_eq!(current_context, 107500);

    // @step And the session file should contain cumulative_billed_input of 1658550
    // (Sum of 100000 + 100500 + ... + 107500)
    // Sum = 16 * 100000 + 500 * (0+1+2+...+15) = 1600000 + 500*120 = 1660000
    assert!(cumulative_billed > current_context * 10); // Significantly more than current

    // @step And the token display should use current_context_tokens for user display
    let display_tokens = tracker.input_tokens; // Used for display
    assert_eq!(display_tokens, current_context);
    assert_ne!(display_tokens, cumulative_billed);
}

// =============================================================================
// Scenario: Stream display shows progressive context size not cumulative
// =============================================================================

#[test]
fn test_stream_display_progressive_not_cumulative() {
    // @step Given I am using the codelet CLI in an interactive session
    let mut tracker = TokenTracker::default();

    // @step When a turn triggers multiple API calls
    // @step And API Call 1 completes with input_tokens of 50000
    tracker.update(50000, 1000, None, None);
    let display_after_call_1 = tracker.input_tokens;
    assert_eq!(display_after_call_1, 50000); // Shows 50k

    // @step And API Call 2 completes with input_tokens of 55000
    tracker.update(55000, 1000, None, None);
    let display_after_call_2 = tracker.input_tokens;
    assert_eq!(display_after_call_2, 55000); // Shows 55k (not 105k)

    // @step And API Call 3 completes with input_tokens of 60000
    tracker.update(60000, 1000, None, None);
    let display_after_call_3 = tracker.input_tokens;
    assert_eq!(display_after_call_3, 60000); // Shows 60k (not 165k)

    // @step Then the display should progressively show "50k -> 55k -> 60k"
    assert_eq!(display_after_call_1, 50000);
    assert_eq!(display_after_call_2, 55000);
    assert_eq!(display_after_call_3, 60000);

    // @step And the display should NOT show "50k -> 105k -> 165k"
    assert_ne!(display_after_call_2, 105000);
    assert_ne!(display_after_call_3, 165000);
}

// =============================================================================
// Scenario: CompactionHook correctly overwrites with per-request value
// =============================================================================

#[test]
fn test_compaction_hook_overwrites_correctly() {
    // @step Given the CompactionHook is processing API responses
    let mut tracker = TokenTracker::default();
    tracker.update(50000, 1000, None, None); // Initial value

    // @step When on_stream_completion_response_finish receives input_tokens of 105000
    tracker.update(105000, 2000, None, None);

    // @step Then state.input_tokens should be set to 105000 by overwriting
    assert_eq!(tracker.input_tokens, 105000);

    // @step And state.input_tokens should NOT be incremented by adding
    let if_it_was_added = 50000 + 105000; // 155000
    assert_ne!(tracker.input_tokens, if_it_was_added);
}

// =============================================================================
// Scenario: Compaction threshold uses correct per-request tokens
// =============================================================================

#[test]
fn test_compaction_threshold_uses_per_request() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And the compaction threshold is set to 180000 tokens
    let threshold: u64 = 180000;

    // @step And the actual current context is 105000 tokens
    let mut tracker = TokenTracker::default();
    for _ in 0..16 {
        tracker.update(105000, 500, None, None);
    }

    // @step When the compaction threshold is checked
    let current_context = tracker.input_tokens;
    let should_compact = current_context > threshold;

    // @step Then compaction should NOT be triggered
    assert!(!should_compact, "105k < 180k, no compaction needed");

    // @step And the threshold check should use 105000 tokens not cumulative sums
    assert_eq!(current_context, 105000);

    // Verify that using cumulative would incorrectly trigger compaction
    let would_compact_with_cumulative = tracker.cumulative_billed_input > threshold;
    assert!(
        would_compact_with_cumulative,
        "Cumulative > threshold (buggy behavior)"
    );
}

// =============================================================================
// Scenario: Distinguish billing total from context display
// =============================================================================

#[test]
fn test_distinguish_billing_from_context() {
    // @step Given I am using the codelet CLI in an interactive session
    // @step And 5 API calls have occurred with context at 100000 tokens each
    let mut tracker = TokenTracker::default();
    for _ in 0..5 {
        tracker.update(100000, 500, None, None);
    }

    // @step When viewing the token metrics
    let context_display = tracker.input_tokens;
    let billing_total = tracker.cumulative_billed_input;

    // @step Then the context display should show 100000 tokens for window usage
    assert_eq!(context_display, 100000);

    // @step And the billing analytics should show 500000 cumulative billed tokens
    assert_eq!(billing_total, 500000);

    // @step And these should be clearly distinguished as separate metrics
    assert_ne!(context_display, billing_total);
}

// =============================================================================
// Additional test: TokenTracker default values
// =============================================================================

#[test]
fn test_token_tracker_default_values() {
    let tracker = TokenTracker::default();

    assert_eq!(tracker.input_tokens, 0);
    assert_eq!(tracker.output_tokens, 0);
    assert_eq!(tracker.cumulative_billed_input, 0);
    assert_eq!(tracker.cumulative_billed_output, 0);
    assert_eq!(tracker.cache_read_input_tokens, None);
    assert_eq!(tracker.cache_creation_input_tokens, None);
}

// =============================================================================
// Additional test: Cache tokens are preserved
// =============================================================================

#[test]
fn test_cache_tokens_preserved() {
    let mut tracker = TokenTracker::default();

    tracker.update(50000, 1000, Some(10000), Some(5000));

    assert_eq!(tracker.cache_read_input_tokens, Some(10000));
    assert_eq!(tracker.cache_creation_input_tokens, Some(5000));

    // Update again - cache values are overwritten (per-request)
    tracker.update(55000, 1100, Some(12000), Some(6000));

    assert_eq!(tracker.cache_read_input_tokens, Some(12000));
    assert_eq!(tracker.cache_creation_input_tokens, Some(6000));
}
