#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/context-overflow-error-classifier.feature
//!
//! CMPCT-044 (classifier slice): The stream-loop error cascade only triggers compaction when the
//! API error matches the fixed `is_prompt_too_long_error` substring list (or a
//! typed `PromptCancelled`). Provider context-overflow errors with different
//! wording fall through to the terminal arm and kill the session.
//!
//! BUG-306: overflow is now classified only from machine-readable provider
//! codes (CMPCT-054, structured-terminal-overflow-classification.feature);
//! these tests keep the rejection guarantees for non-overflow errors.

use codelet_cli::interactive::is_context_overflow_error;

/// Wrap an API error string the way the stream loop sees it: the provider
/// body is inside the error chain after rig's streaming error layers.
fn api_error(body: &str) -> anyhow::Error {
    anyhow::anyhow!(body.to_string())
}

/// Scenario: Overflow classifier rejects thinking-budget configuration errors
#[test]
fn overflow_classifier_rejects_thinking_budget_errors() {
    // @step Given an API error containing "budget_tokens" from a thinking-budget configuration failure
    let err =
        "budget_tokens must be greater than max_tokens: invalid thinking budget configuration";

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
