//! Error classification functions for the streaming loop.
//!
//! Pure functions that classify error strings into categories. No side effects, no state.
//! Used by the stream loop to determine which recovery strategy to apply.

use codelet_core::TokenState;
use std::sync::{Arc, Mutex};
use tracing::warn;

/// Check if an error indicates the prompt/context is too long
/// PROV-010: Exclude thinking budget configuration errors (budget_tokens)
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: rust/cli/tests/prompt_too_long_recovery_test.rs
pub fn is_prompt_too_long_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();

    // PROV-010: Exclude thinking budget configuration errors
    // These contain "budget_tokens" and should NOT trigger compaction
    if error_lower.contains("budget_tokens") {
        return false;
    }

    error_lower.contains("prompt is too long")
        || error_lower.contains("maximum context length")
        || error_lower.contains("context_length_exceeded")
        || error_lower.contains("too many tokens")
        || error_lower.contains("exceeds the model")
        || (error_lower.contains("invalid_request_error")
            && (error_lower.contains("token") || error_lower.contains("maximum")))
}

/// EXT-016: Check if an error indicates image content was rejected by the API.
///
/// Detects 400 errors related to image dimensions, image size, or image processing.
/// This is used to trigger image content sanitization in the error recovery path.
///
/// This function is public for testing.
pub fn is_image_content_error(error_str: &str) -> bool {
    let error_lower = error_str.to_lowercase();

    // Must mention "image" in conjunction with dimension/size-related terms
    if error_lower.contains("image") {
        return error_lower.contains("dimension")
            || error_lower.contains("exceed")
            || error_lower.contains("too large")
            || error_lower.contains("max allowed size")
            || error_lower.contains("size");
    }

    false
}

/// PROV-040: Check if an error indicates a truncated tool call due to output token limit.
///
/// Detects the enriched error message emitted by PROV-039 in the Anthropic streaming
/// handler when `stop_reason == "max_tokens"` and a pending tool call was never closed.
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy. See: rust/cli/tests/truncation_recovery_test.rs
pub fn is_truncated_tool_call_error(error_str: &str) -> bool {
    error_str.contains("Tool call truncated due to output token limit")
}

/// Check if an error is a transient network/connection error that can be retried.
///
/// Detects HTTP transport failures, DNS errors, connection resets, and similar
/// transient issues that may resolve on retry. These errors originate from the
/// reqwest HTTP client or the SSE transport layer.
///
/// This function is public for testing. Tests MUST import and test the
/// real function, NOT a copy.
pub fn is_transient_network_error(error_str: &str) -> bool {
    let lower = error_str.to_lowercase();

    // reqwest-level transport errors
    lower.contains("error sending request")
        || lower.contains("connection reset")
        || lower.contains("connection refused")
        || lower.contains("connection closed")
        || lower.contains("broken pipe")
        || lower.contains("network is unreachable")
        || lower.contains("dns error")
        || lower.contains("connection aborted")
        || lower.contains("connection was not ready")
        // Timeout errors (network-level, not API-level)
        || (lower.contains("timed out") && !lower.contains("context_length"))
        || lower.contains("operation timed out")
        // hyper-level errors
        || lower.contains("hyper::error")
        || lower.contains("stream closed before completion")
        // SSE transport errors (wrapping the above)
        || (lower.contains("sse error") && lower.contains("http client error"))
        // reqwest TLS errors
        || lower.contains("ssl routines")
        || lower.contains("certificate")
        // Generic I/O errors during streaming
        || (lower.contains("sse error") && lower.contains("instance"))
        // EOF during streaming
        || lower.contains("unexpected eof")
        || lower.contains("incomplete message")
}

/// AMGR-016: Check if an error indicates a stall timeout (no streaming data received).
///
/// Stall timeouts are TERMINAL errors that must NOT be retried by any error classifier.
/// They bypass the entire error classifier cascade — Rule [5], Rule [6].
///
/// Uses the canonical prefix from `recovery_stall::STALL_TIMEOUT_ERROR_PREFIX` to ensure
/// the identifier string is always in sync between creation and detection.
///
/// This function is public for testing.
pub fn is_stall_timeout_error(error_str: &str) -> bool {
    error_str.contains(super::recovery_stall::STALL_TIMEOUT_ERROR_PREFIX)
}

/// CMPCT-002: Check if an error indicates compaction was cancelled by the hook.
///
/// CMPCT-025: Uses structural downcasting with `anyhow::Error::chain()`
/// traversal, making detection robust against:
/// - `.context(...)` wrapping (adds a new layer to the source chain)
/// - `StreamingError::Prompt(Box<PromptError>)` wrapping (rig's current yield path)
/// - Arbitrary nesting of the above
///
/// Only returns `true` when a typed `PromptError::PromptCancelled` variant is
/// present in the error chain. Bare string errors such as
/// `anyhow::Error::msg("PromptCancelled")` are NOT matched, since they are not
/// real typed cancellations.
///
/// CMPCT-026: Production code now uses `classify_compaction_branch` instead
/// of this thin boolean wrapper. The function is retained under
/// `#[cfg(test)]` so the CMPCT-025 regression suite (which exercises the
/// underlying `extract_prompt_cancelled` chain walk via this wrapper) still
/// runs without triggering `-D dead-code`.
#[cfg(test)]
pub(super) fn is_compaction_cancelled(error: &anyhow::Error) -> bool {
    extract_prompt_cancelled(error).is_some()
}

/// CMPCT-025: Extract the `chat_history` payload from a
/// `PromptError::PromptCancelled` variant if one exists anywhere in the
/// anyhow error chain.
///
/// Returns `None` when no `PromptCancelled` variant is present (including the
/// case of bare string errors that merely render as `"PromptCancelled"` via
/// `Display`).
///
/// The chain walk handles two shapes of `PromptError` node:
/// 1. A direct `PromptError` value (e.g. `anyhow::Error::from(PromptError::...)`).
/// 2. A `Box<PromptError>` value — this is what rig yields today via
///    `StreamingError::Prompt(#[from] Box<PromptError>)`. `thiserror`'s
///    generated `source()` returns the boxed value as-is, so the runtime
///    type seen in the chain is `Box<PromptError>`, not `PromptError`.
///    The inner `if let` guard ensures only the `PromptCancelled` variant
///    is treated as a match; other boxed `PromptError` variants fall through.
///
/// Callers that need the conversation state captured at cancellation time
/// (e.g. for in-view DAG reconstruction — see CMPCT-029) can use this helper
/// to recover the dropped-on-the-floor `chat_history` payload.
pub(super) fn extract_prompt_cancelled(
    error: &anyhow::Error,
) -> Option<&Vec<rig::message::Message>> {
    for err in error.chain() {
        // Case 1: direct PromptError in the chain.
        if let Some(rig::completion::PromptError::PromptCancelled { chat_history }) =
            err.downcast_ref::<rig::completion::PromptError>()
        {
            return Some(chat_history);
        }
        // Case 2: Box<PromptError> in the chain — rig's StreamingError::Prompt
        // carries `Box<PromptError>` and `thiserror` exposes it verbatim
        // through source(). Match the boxed form too, but only claim a match
        // for the PromptCancelled variant so unrelated boxed variants
        // (e.g. MaxDepthError) continue walking.
        if let Some(boxed) = err.downcast_ref::<Box<rig::completion::PromptError>>() {
            if let rig::completion::PromptError::PromptCancelled { chat_history } = boxed.as_ref() {
                return Some(chat_history);
            }
        }
    }
    None
}

/// CMPCT-026: Why the two signals disagreed at classification time.
///
/// The stream loop has historically gated compaction recovery on the
/// conjunction `is_compaction_cancelled(&e) && state.compaction_needed`.
/// After CMPCT-025 the error side is structurally reliable (typed
/// `PromptError::PromptCancelled` downcast), so the flag collapses from a
/// gate into a defence-in-depth cross-check. When the two signals disagree
/// the classifier records which side was "wrong" so callers can emit a
/// single structured `tracing::warn!` without duplicating the branching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionDisagreement {
    /// `PromptCancelled` fired but `compaction_needed` was `false`.
    /// Classifier has defensively set the flag to `true` so the post-loop
    /// recovery handler still runs. This historically silently terminated
    /// the session.
    FlagMissing,
    /// `compaction_needed` is `true` but the error is NOT `PromptCancelled`.
    /// Something upstream set the flag without a real cancel — the error
    /// must fall through to the normal classifier cascade rather than
    /// routing to compaction recovery.
    FlagExtraneous,
}

/// CMPCT-026: Result of classifying a stream error against the shared
/// `TokenState.compaction_needed` flag.
///
/// Follows Option A of the CMPCT-026 plan: the error is authoritative.
/// Presence of a typed `PromptError::PromptCancelled` in the error chain
/// routes to `Recover`; absence routes to `NotCompaction`. The flag is
/// recorded only as a disagreement marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionBranch {
    /// Route to the compaction-recovery path. Caller is responsible for
    /// flushing partial state via `begin_compaction_recovery` and invoking
    /// the in-loop compaction restart (CMPCT-027).
    Recover {
        /// `Some(FlagMissing)` when PromptCancelled fired without the flag
        /// being set; the classifier has already defensively set the flag.
        /// `None` when both signals agreed.
        disagreement: Option<CompactionDisagreement>,
    },
    /// Fall through to the normal error-classifier cascade
    /// (`is_prompt_too_long_error`, `is_image_content_error`, …).
    NotCompaction {
        /// `Some(FlagExtraneous)` when the flag was true but no
        /// PromptCancelled was present — the flag is left untouched so
        /// upstream state mutations remain observable. `None` for the
        /// ordinary case.
        disagreement: Option<CompactionDisagreement>,
    },
}

/// CMPCT-026: Single source of truth for the compaction-cancel branch.
///
/// Replaces the fragile `is_compaction_cancelled(&e) && state.compaction_needed`
/// conjunction in `stream_loop.rs` and the structurally-identical
/// conditional in `gemini_continuation.rs`. Today only one of four
/// `cancel_sig.cancel()` call sites in `codelet-core` also sets
/// `state.compaction_needed` — the `&&` guard therefore silently drops
/// recoverable sessions whenever a non-threshold cancel path fires.
///
/// Contract:
/// - If `PromptCancelled` is present in the error chain: return `Recover`.
///   When the flag was also already `true`, `disagreement` is `None`.
///   When the flag was `false`, the classifier sets it to `true` for
///   defence-in-depth (so the post-loop handler still routes correctly)
///   and attaches `disagreement: Some(FlagMissing)`. A `warn!` is emitted
///   in either disagreement case.
/// - If `PromptCancelled` is absent: return `NotCompaction`. When the flag
///   was `true`, attach `disagreement: Some(FlagExtraneous)` and `warn!`.
///   The flag is NEVER cleared from this branch — upstream state
///   mutations must remain observable.
pub fn classify_compaction_branch(
    error: &anyhow::Error,
    token_state: &Arc<Mutex<TokenState>>,
) -> CompactionBranch {
    let prompt_cancelled = extract_prompt_cancelled(error).is_some();
    let flag = token_state
        .lock()
        .map(|state| state.compaction_needed)
        .unwrap_or(false);

    match (prompt_cancelled, flag) {
        (true, true) => CompactionBranch::Recover { disagreement: None },
        (true, false) => {
            warn!(
                error = %error,
                "PromptCancelled without compaction_needed=true; recovering anyway"
            );
            if let Ok(mut state) = token_state.lock() {
                state.compaction_needed = true;
            }
            CompactionBranch::Recover {
                disagreement: Some(CompactionDisagreement::FlagMissing),
            }
        }
        (false, true) => {
            warn!(
                error = %error,
                "compaction_needed=true but error is not PromptCancelled; falling through to classifier cascade"
            );
            CompactionBranch::NotCompaction {
                disagreement: Some(CompactionDisagreement::FlagExtraneous),
            }
        }
        (false, false) => CompactionBranch::NotCompaction { disagreement: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Feature: spec/features/agent-stall-detection.feature

    // ====================================================================
    // Scenario: Stall timeout error is not caught by error classifiers
    // ====================================================================

    // @step Given the stream loop has a stall timeout configured
    // @step When the stall timeout fires due to no tokens received
    // @step Then the error should bypass the error classifier cascade
    // @step And the error should not be retried as a network or truncation error
    // @step And the stream loop should break immediately with a terminal error
    #[test]
    fn stall_timeout_error_is_identified_by_dedicated_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 300s. \
                         The LLM connection is alive but not producing tokens. \
                         This may indicate an API-side hang or an overloaded endpoint.";

        assert!(
            is_stall_timeout_error(stall_msg),
            "is_stall_timeout_error must identify stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_network_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 300s. \
                         The LLM connection is alive but not producing tokens.";

        assert!(
            !is_transient_network_error(stall_msg),
            "is_transient_network_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_truncation_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 300s.";

        assert!(
            !is_truncated_tool_call_error(stall_msg),
            "is_truncated_tool_call_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_prompt_too_long_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 300s.";

        assert!(
            !is_prompt_too_long_error(stall_msg),
            "is_prompt_too_long_error must NOT catch stall timeout errors"
        );
    }

    #[test]
    fn stall_timeout_error_not_caught_by_image_content_classifier() {
        let stall_msg = "Generation stalled: no streaming data received for 300s.";

        assert!(
            !is_image_content_error(stall_msg),
            "is_image_content_error must NOT catch stall timeout errors"
        );
    }

    // ====================================================================
    // Scenario: Network retry logic is not affected by stall timeout
    // ====================================================================

    // @step Given a subordinate agent is running and streaming a response
    // @step When a transient network error occurs during streaming
    // @step Then the existing NET-001 retry logic should handle the error
    // @step And the stall timeout should not interfere with the retry backoff
    // @step And the agent should complete normally after successful retry
    #[test]
    fn network_errors_are_not_classified_as_stall() {
        let network_errors = [
            "error sending request for url",
            "connection reset by peer",
            "dns error: failed to lookup address",
            "connection refused",
            "operation timed out",
        ];

        for error_msg in &network_errors {
            assert!(
                is_transient_network_error(error_msg),
                "'{error_msg}' should be classified as transient network error"
            );
            assert!(
                !is_stall_timeout_error(error_msg),
                "'{error_msg}' must NOT be classified as stall timeout"
            );
        }
    }

    #[test]
    fn stall_classifier_does_not_match_unrelated_errors() {
        let unrelated = [
            "API key invalid",
            "rate limit exceeded",
            "model not found",
            "internal server error",
            "connection reset",
        ];

        for error_msg in &unrelated {
            assert!(
                !is_stall_timeout_error(error_msg),
                "'{error_msg}' must NOT be classified as stall timeout"
            );
        }
    }

    // ====================================================================
    // CMPCT-025: Structural downcast of PromptError::PromptCancelled
    //
    // Feature: spec/features/replace-stringly-typed-is-compaction-cancelled-with-structural-prompterror-downcast.feature
    //
    // These tests cover the five scenarios defined for CMPCT-025:
    //   1. Direct PromptError::PromptCancelled is detected
    //   2. .context()-wrapped PromptCancelled is detected via chain traversal
    //   3. StreamingError::Prompt-wrapped PromptCancelled is detected
    //   4. Bare string errors that merely say "PromptCancelled" are rejected
    //   5. Other PromptError variants are not mistaken for cancellation
    // ====================================================================

    use rig::agent::StreamingError;
    use rig::completion::PromptError;
    use rig::message::Message;

    #[test]
    fn structurally_detects_prompt_cancelled() {
        // @step Given an anyhow::Error built directly from PromptError::PromptCancelled
        let err: anyhow::Error = PromptError::PromptCancelled {
            chat_history: Box::new(Vec::<Message>::new()),
        }
        .into();

        // @step When is_compaction_cancelled is called with that error
        let detected = is_compaction_cancelled(&err);

        // @step Then the function returns true
        assert!(
            detected,
            "structural downcast must detect a direct PromptCancelled error"
        );

        // @step And extract_prompt_cancelled returns Some(chat_history)
        let extracted = extract_prompt_cancelled(&err);
        assert!(
            extracted.is_some(),
            "extract_prompt_cancelled must return the chat_history payload"
        );
        assert!(
            extracted.map(Vec::is_empty).unwrap_or(false),
            "chat_history payload should match the one embedded in the variant"
        );
    }

    #[test]
    fn detects_wrapped_prompt_cancelled() {
        // @step Given an anyhow::Error built from PromptError::PromptCancelled
        let inner: anyhow::Error = PromptError::PromptCancelled {
            chat_history: Box::new(Vec::<Message>::new()),
        }
        .into();

        // @step And the error is then wrapped with anyhow::Context::context
        let wrapped = inner.context("upstream streaming error");

        // @step When is_compaction_cancelled is called with that error
        // @step Then the function returns true
        assert!(
            is_compaction_cancelled(&wrapped),
            "chain traversal must see through .context() wrapping"
        );
    }

    #[test]
    fn detects_streaming_error_wrapped_prompt_cancelled() {
        // @step Given a StreamingError::Prompt containing Box(PromptError::PromptCancelled) is created
        let streaming_err = StreamingError::Prompt(Box::new(PromptError::PromptCancelled {
            chat_history: Box::new(Vec::<Message>::new()),
        }));

        // @step When the error is converted using the production path anyhow::Error::from(e)
        let err: anyhow::Error = anyhow::Error::from(streaming_err);

        // @step Then extract_prompt_cancelled returns Some(chat_history)
        let extracted = extract_prompt_cancelled(&err);
        assert!(
            extracted.is_some(),
            "extract_prompt_cancelled must return the chat_history payload via Error::from path"
        );

        // @step And the typed error chain is preserved for downstream downcast_ref extraction
        assert!(
            is_compaction_cancelled(&err),
            "chain traversal must find PromptCancelled inside StreamingError::Prompt"
        );
    }

    #[test]
    fn does_not_match_bare_string() {
        // @step Given an anyhow::Error created via anyhow::Error::msg("PromptCancelled")
        let err = anyhow::Error::msg("PromptCancelled");

        // @step When is_compaction_cancelled is called with that error
        // @step Then the function returns false
        // @step And bare string errors are correctly rejected as non-typed cancellations
        assert!(
            !is_compaction_cancelled(&err),
            "bare string errors are not typed PromptCancelled variants"
        );
    }

    #[test]
    fn does_not_match_other_prompt_errors() {
        // @step Given an anyhow::Error built from PromptError::MaxDepthError
        let err: anyhow::Error = PromptError::MaxDepthError {
            max_depth: 3,
            chat_history: Box::new(Vec::<Message>::new()),
            prompt: Box::new(Message::from("ignored")),
        }
        .into();

        // @step When is_compaction_cancelled is called with that error
        // @step Then the function returns false
        assert!(
            !is_compaction_cancelled(&err),
            "MaxDepthError is a different variant and must not match"
        );
        assert!(
            extract_prompt_cancelled(&err).is_none(),
            "extract_prompt_cancelled must yield None for non-cancellation variants"
        );
    }

    #[test]
    fn error_from_preserves_downcast_for_extract_prompt_cancelled() {
        // @step Given a StreamingError::Prompt(Box(PromptError::PromptCancelled)) is created
        let streaming_err = StreamingError::Prompt(Box::new(PromptError::PromptCancelled {
            chat_history: Box::new(Vec::<Message>::new()),
        }));

        // @step When the error is converted via anyhow::Error::from(e) which matches production
        let err: anyhow::Error = anyhow::Error::from(streaming_err);

        // @step Then the error chain preserves the original StreamingError and PromptError types
        let mut found_streaming = false;
        let mut found_prompt = false;
        for e in err.chain() {
            if e.downcast_ref::<StreamingError>().is_some() {
                found_streaming = true;
            }
            if e.downcast_ref::<Box<rig::completion::PromptError>>()
                .is_some()
            {
                found_prompt = true;
            }
        }
        assert!(
            found_streaming,
            "error chain must preserve the original StreamingError type"
        );

        // @step And extract_prompt_cancelled successfully downcasts and returns Some(chat_history)
        assert!(
            found_prompt,
            "error chain must preserve the boxed PromptError type for downcast"
        );
        assert!(
            extract_prompt_cancelled(&err).is_some(),
            "extract_prompt_cancelled must return Some via the preserved type chain"
        );
    }
}
