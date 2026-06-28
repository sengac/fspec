#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/reasoning-token-propagation.feature
//!
//! Tests for TokenInfo reasoning token mapping from TokenDisplayUpdate and ApiTokenUsage.

use codelet_cli::interactive::output::TokenInfo;
use codelet_core::ApiTokenUsage;
use codelet_core::TokenDisplayUpdate;

// =============================================================================
// Scenario: TokenInfo maps reasoning tokens from TokenDisplayUpdate
// =============================================================================

#[test]
fn test_token_info_from_token_display_update_with_reasoning() {
    // @step Given a TokenDisplayUpdate with reasoning_tokens set to 3000
    let update = TokenDisplayUpdate {
        input_tokens: 1000,
        output_tokens: 500,
        cache_read_tokens: 200,
        cache_creation_tokens: 100,
        tokens_per_second: Some(50.0),
        reasoning_tokens: 3000,
    };

    // @step When the TokenDisplayUpdate is converted to TokenInfo via From trait
    let info: TokenInfo = update.into();

    // @step Then the resulting TokenInfo should have reasoning_tokens equal to Some(3000)
    assert_eq!(info.reasoning_tokens, Some(3000));
}

#[test]
fn test_token_info_from_token_display_update_zero_reasoning() {
    let update = TokenDisplayUpdate {
        input_tokens: 1000,
        output_tokens: 500,
        cache_read_tokens: 200,
        cache_creation_tokens: 100,
        tokens_per_second: None,
        reasoning_tokens: 0,
    };

    let info: TokenInfo = update.into();

    // Zero reasoning should map to None (or Some(0)) - either is acceptable
    // The important thing is it doesn't panic
    assert!(info.reasoning_tokens.is_none() || info.reasoning_tokens == Some(0));
}

// =============================================================================
// Scenario: TokenInfo from_usage maps reasoning tokens from ApiTokenUsage
// =============================================================================

#[test]
fn test_token_info_from_usage_with_reasoning() {
    // @step Given an ApiTokenUsage with reasoning_tokens of 2000
    let usage = ApiTokenUsage::new(10_000, 5_000, 1_000, 500).with_reasoning_tokens(2_000);

    // @step When TokenInfo::from_usage is called
    let info = TokenInfo::from_usage(usage, Some(50.0));

    // @step Then the resulting TokenInfo should have reasoning_tokens equal to Some(2000)
    assert_eq!(info.reasoning_tokens, Some(2_000));
}

#[test]
fn test_token_info_from_usage_without_reasoning() {
    let usage = ApiTokenUsage::new(10_000, 5_000, 1_000, 500);

    let info = TokenInfo::from_usage(usage, Some(50.0));

    // When ApiTokenUsage has reasoning_tokens = 0, TokenInfo should reflect that
    assert!(info.reasoning_tokens.is_none() || info.reasoning_tokens == Some(0));
}
