//! Feature: spec/features/codelet-apitokenusage-reasoning-tokens.feature
//!
//! This test file validates the acceptance criteria defined in the feature file.
//! Scenarios map directly to Gherkin scenarios.
//!
//! Layer 3: codelet-core ApiTokenUsage reasoning_tokens

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codelet_core::ApiTokenUsage;

// =============================================================================
// Scenario: ApiTokenUsage includes reasoning_tokens field
// =============================================================================

#[test]
fn test_api_token_usage_includes_reasoning_tokens_field() {
    // @step Given the codelet-core ApiTokenUsage struct is defined
    // @step When I inspect the ApiTokenUsage struct fields
    let usage = ApiTokenUsage::default();

    // @step Then it should have a reasoning_tokens field of type u64
    let _reasoning: u64 = usage.reasoning_tokens;

    // @step And the Default impl should set reasoning_tokens to 0
    assert_eq!(usage.reasoning_tokens, 0);
}

// =============================================================================
// Scenario: ApiTokenUsage updates from rig Usage with reasoning tokens
// =============================================================================

#[test]
fn test_api_token_usage_updates_from_usage_with_reasoning_tokens() {
    // @step Given a rig Usage with reasoning_tokens Some(3000)
    let rig_usage = rig::completion::Usage {
        input_tokens: 1000,
        output_tokens: 500,
        total_tokens: 1500,
        reasoning_tokens: Some(3000),
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };

    // @step When update_from_usage is called on ApiTokenUsage
    let mut api_usage = ApiTokenUsage::default();
    api_usage.update_from_usage(&rig_usage);

    // @step Then ApiTokenUsage.reasoning_tokens should be 3000
    assert_eq!(api_usage.reasoning_tokens, 3000);
}

// =============================================================================
// Scenario: ApiTokenUsage updates from rig Usage with None reasoning tokens
// =============================================================================

#[test]
fn test_api_token_usage_updates_from_usage_with_none_reasoning_tokens() {
    // @step Given a rig Usage with reasoning_tokens None
    let rig_usage = rig::completion::Usage {
        input_tokens: 1000,
        output_tokens: 500,
        total_tokens: 1500,
        reasoning_tokens: None,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
    };

    // @step When update_from_usage is called on ApiTokenUsage
    let mut api_usage = ApiTokenUsage::default();
    api_usage.update_from_usage(&rig_usage);

    // @step Then ApiTokenUsage.reasoning_tokens should be 0
    assert_eq!(api_usage.reasoning_tokens, 0);
}

// =============================================================================
// Scenario: ApiTokenUsage total_context includes reasoning tokens
// =============================================================================

#[test]
fn test_api_token_usage_total_context_includes_reasoning_tokens() {
    // @step Given an ApiTokenUsage with input_tokens 10000 output_tokens 500 and reasoning_tokens 2000
    let usage = ApiTokenUsage {
        input_tokens: 10000,
        output_tokens: 500,
        reasoning_tokens: 2000,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
    };

    // @step When total_context() is called
    let result = usage.total_context();

    // @step Then the result should be 12500
    assert_eq!(result, 12500);
}

// =============================================================================
// Scenario: ApiTokenUsage total_context without reasoning tokens
// =============================================================================

#[test]
fn test_api_token_usage_total_context_without_reasoning_tokens() {
    // @step Given an ApiTokenUsage with input_tokens 10000 output_tokens 500 and reasoning_tokens 0
    let usage = ApiTokenUsage {
        input_tokens: 10000,
        output_tokens: 500,
        reasoning_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
    };

    // @step When total_context() is called
    let result = usage.total_context();

    // @step Then the result should be 10500
    assert_eq!(result, 10500);
}
