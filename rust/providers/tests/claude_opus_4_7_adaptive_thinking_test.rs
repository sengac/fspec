//! Feature: spec/features/claude-opus-4-7-adaptive-thinking.feature
//!
//! This test file validates that Claude Opus 4.7 is correctly recognised as an
//! adaptive-only thinking model.  Tests use PRODUCTION code from codelet_tools
//! and codelet_providers.
//!
//! PROV-079: Opus 4.7 adaptive thinking support.
//! Note: No per-model constant needed — default-adaptive logic covers all
//! Claude 4.6+ models automatically.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use codelet_tools::facade::is_adaptive_thinking_model;

use codelet_providers::claude::build_beta_headers;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Scenario: Opus 4.7 is detected as adaptive thinking model
    // =========================================================================

    #[test]
    fn test_opus_4_7_is_adaptive_thinking_model() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";

        // @step When I check is_adaptive_thinking_model
        let result = is_adaptive_thinking_model(model);

        // @step Then the result should be true
        assert!(
            result,
            "claude-opus-4-7 should be an adaptive thinking model"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.7 beta headers exclude interleaved-thinking
    // =========================================================================

    #[test]
    fn test_opus_4_7_beta_headers_exclude_interleaved_thinking() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";

        // @step When I build beta headers for the model
        let headers = build_beta_headers(model, false);

        // @step Then the headers should NOT include "interleaved-thinking-2025-05-14"
        assert!(
            !headers.contains("interleaved-thinking-2025-05-14"),
            "Opus 4.7 should NOT include interleaved-thinking, got: {headers}"
        );

        // @step And the headers should include "prompt-caching-2024-07-31"
        assert!(
            headers.contains("prompt-caching-2024-07-31"),
            "Should include prompt-caching, got: {headers}"
        );
    }
}
