// Feature: spec/features/compression-ratio-clamping.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.
//
// CMPCT-039: the shared `compression_ratio()` helper in
// rust/cli/src/interactive_helpers.rs must clamp its result to [0.0, 1.0]
// so no producer can ever ship a negative ratio on the wire.
//
// The producer-level scenario for `compact_session` is pinned by
// rust/sessions/tests/rpc418_compact_session.rs (the RPC result is an
// acknowledgement with compression_ratio exactly 0.0).

use codelet_cli::interactive_helpers::compression_ratio;

// ========================================
// Scenario: Helper clamps a context-growth ratio to zero
// ========================================

#[test]
fn test_helper_clamps_context_growth_ratio_to_zero() {
    // @step Given a compaction where the compacted token count 1600 exceeds the original token count 1000
    let original_tokens: u64 = 1000;
    let compacted_tokens: u64 = 1600;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns exactly 0.0
    assert_eq!(
        ratio, 0.0,
        "compression_ratio(1000, 1600) must clamp to exactly 0.0, got {ratio}"
    );

    // @step And the helper never returns a negative value
    assert!(
        ratio >= 0.0,
        "compression_ratio must never return a negative value, got {ratio}"
    );
}

// ========================================
// Scenario: Helper reports a normal reduction unchanged
// ========================================

#[test]
fn test_helper_reports_normal_reduction_unchanged() {
    // @step Given a compaction where the original token count 1000 shrinks to a compacted token count of 400
    let original_tokens: u64 = 1000;
    let compacted_tokens: u64 = 400;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns 0.6 representing 60 percent of tokens removed
    assert!(
        (ratio - 0.6).abs() < 1e-12,
        "compression_ratio(1000, 400) must be 0.6 (60% removed), got {ratio}"
    );
}

// ========================================
// Scenario: Helper returns zero when the original token count is zero
// ========================================

#[test]
fn test_helper_returns_zero_when_original_is_zero() {
    // @step Given a compaction where the original token count is 0
    let original_tokens: u64 = 0;
    let compacted_tokens: u64 = 500;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns exactly 0.0 via the division guard
    assert_eq!(
        ratio, 0.0,
        "compression_ratio(0, 500) must return 0.0 via the zero-original guard, got {ratio}"
    );
}

// ========================================
// Scenario: Helper returns zero when compacted tokens equal original tokens
// ========================================

#[test]
fn test_helper_returns_zero_when_compacted_equals_original() {
    // @step Given a compaction where the compacted token count equals the original token count of 800
    let original_tokens: u64 = 800;
    let compacted_tokens: u64 = 800;

    // @step When the compression ratio is calculated by the shared helper
    let ratio = compression_ratio(original_tokens, compacted_tokens);

    // @step Then the helper returns exactly 0.0
    assert_eq!(
        ratio, 0.0,
        "compression_ratio(800, 800) must be exactly 0.0 (no reduction), got {ratio}"
    );
}
