#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature
//!
//! CMPCT-044: The stream-loop error cascade only triggers compaction when the
//! API error matches the fixed `is_prompt_too_long_error` substring list (or a
//! typed `PromptCancelled`). Provider context-overflow errors with different
//! wording fall through to the terminal arm and kill the session.
//!
//! These tests cover the new `is_context_overflow_error` classifier:
//! a strict superset of `is_prompt_too_long_error` that also matches
//! provider-variant wording, survives anyhow wrapping, and keeps the PROV-010
//! thinking-budget exclusion.

use codelet_cli::interactive::{is_context_overflow_error, is_prompt_too_long_error};

/// Wrap an API error string the way the stream loop sees it: the provider
/// body is inside the error chain after rig's streaming error layers.
fn api_error(body: &str) -> anyhow::Error {
    anyhow::anyhow!(body.to_string())
}

/// Scenario: Overflow classifier matches every error is_prompt_too_long_error matches
#[test]
fn overflow_classifier_matches_every_prompt_too_long_error() {
    // @step Given a provider context-overflow error string that the existing is_prompt_too_long_error classifier matches
    let legacy_matches = [
        "prompt is too long (requested: 250000)",
        "This model's maximum context length is 200000 tokens.",
        "context_length_exceeded",
        "You exceeded your current quota due to too many tokens",
        "This input exceeds the model's maximum context",
        "invalid_request_error: token limit reached (maximum)",
    ];

    // @step When is_context_overflow_error is called with that error string
    // @step Then is_context_overflow_error returns true
    // @step And the new classifier matches it for the same reason as the old one
    for err in &legacy_matches {
        assert!(
            is_prompt_too_long_error(err),
            "precondition: legacy classifier must match '{err}'"
        );
        assert!(
            is_context_overflow_error(&api_error(err)),
            "is_context_overflow_error must be a strict superset of \
             is_prompt_too_long_error — must match '{err}'"
        );
    }
}

/// Scenario: Overflow classifier matches provider-variant wording the old classifier misses
#[test]
fn overflow_classifier_matches_provider_variant_wording() {
    // @step Given an OpenAI-compatible provider error "Input is too long: 201,000 tokens > 200,000 maximum"
    let err = "Input is too long: 201,000 tokens > 200,000 maximum";

    // @step When is_context_overflow_error is called with that error string
    let detected = is_context_overflow_error(&api_error(err));

    // @step Then is_context_overflow_error returns true
    assert!(
        detected,
        "is_context_overflow_error must match '{err}'"
    );

    // @step And is_prompt_too_long_error returns false for that same string
    assert!(
        !is_prompt_too_long_error(err),
        "the old classifier must NOT match '{err}' — this is the missed-error gap"
    );
}

/// Scenario: Overflow classifier matches Bedrock and Vertex context-limit wording
#[test]
fn overflow_classifier_matches_bedrock_vertex_wording() {
    // @step Given a provider error in the style "The input (approximately 200,500 tokens) exceeds the maximum number of input tokens (200,000)"
    let bedrock = "The input (approximately 200,500 tokens) exceeds the maximum \
                   number of input tokens (200,000) allowed for this model.";
    let vertex = "Context length 201000 exceeds the maximum allowed for this model.";
    let generic = "context length exceeded: got 201,000, max 200,000";

    // @step When is_context_overflow_error is called with that error string
    // @step Then is_context_overflow_error returns true
    for err in [bedrock, vertex, generic] {
        assert!(
            is_context_overflow_error(&api_error(err)),
            "is_context_overflow_error must match provider variant '{err}'"
        );
    }
}

/// Scenario: Overflow classifier rejects thinking-budget configuration errors
#[test]
fn overflow_classifier_rejects_thinking_budget_errors() {
    // @step Given an API error containing "budget_tokens" from a thinking-budget configuration failure
    let err = "budget_tokens must be greater than max_tokens: invalid thinking budget configuration";

    // @step When is_context_overflow_error is called with that error string
    // @step Then is_context_overflow_error returns false
    assert!(
        !is_context_overflow_error(&api_error(err)),
        "PROV-010: thinking-budget configuration errors must NEVER trigger compaction"
    );
}

/// Scenario: Overflow classifier rejects truncation and unrelated errors
#[test]
fn overflow_classifier_rejects_truncation_and_unrelated_errors() {
    // @step Given an API error that is NOT a context-overflow error (a truncation, rate-limit, or auth error)
    let not_overflow = [
        "Tool call truncated due to output token limit",
        "Rate limit reached for gpt-4o in organization org-123",
        "401 Invalid API key provided: sk-abc123",
        "model not found: claude-9-ultra",
        "500 Internal server error",
        "Connection reset by peer",
    ];

    // @step When is_context_overflow_error is called with that error string
    // @step Then is_context_overflow_error returns false
    for err in &not_overflow {
        assert!(
            !is_context_overflow_error(&api_error(err)),
            "is_context_overflow_error must NOT match non-overflow error '{err}'"
        );
    }
}

/// Scenario: Overflow classifier sees through anyhow context wrapping
#[test]
fn overflow_classifier_sees_through_anyhow_context_wrasing() {
    // @step Given a context-overflow API error that is wrapped with anyhow context layers
    let inner: anyhow::Error =
        anyhow::anyhow!("Input is too long: 210,000 tokens > 200,000 maximum");
    let wrapped = inner
        .context("upstream streaming error")
        .context("agent turn failed");

    // @step When is_context_overflow_error is called with the wrapped error
    // @step Then is_context_overflow_error returns true based on the full error chain
    assert!(
        is_context_overflow_error(&wrapped),
        "the classifier must walk the full anyhow error chain, not just the outer message"
    );
}
