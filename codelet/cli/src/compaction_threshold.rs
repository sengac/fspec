//! Compaction Threshold Calculation (CLI-020)
//!
//! Provides constants and functions for calculating when context compaction
//! should be triggered, including the autocompact buffer that leaves headroom
//! after compaction to reduce re-compaction frequency.
//!
//! Reference: spec/features/autocompact-buffer.feature

/// Autocompact buffer in tokens
///
/// This buffer defines the target context size AFTER compaction, leaving
/// headroom before the next compaction trigger.
///
/// How it works:
/// - Threshold: calculate_usable_context() → context_window - min(max_output, 32k)
/// - Budget (target after compact): contextWindow - AUTOCOMPACT_BUFFER (e.g., 150k for 200k window)
/// - Headroom: threshold - budget (varies by model)
///
/// This separation ensures:
/// - After compaction, there's headroom before next trigger
/// - Reduces re-compaction frequency by providing buffer space for new interactions
///
/// Example for Claude (200k context, 8k max_output):
/// - Trigger threshold: 191,808 tokens (200k - 8k)
/// - Target after compaction: 150,000 tokens (200k - 50k)
/// - Headroom before re-trigger: 41,808 tokens
pub const AUTOCOMPACT_BUFFER: u64 = 50_000;

/// Calculate the summarization budget for context compaction
///
/// Matches TypeScript implementation in compaction.ts:calculateSummarizationBudget()
///
/// The budget determines how many tokens to target after compaction.
/// It is separate from the threshold calculation.
///
/// Logic:
/// - If context_window <= AUTOCOMPACT_BUFFER: budget = context_window * 0.8
/// - Otherwise: budget = context_window - AUTOCOMPACT_BUFFER
///
/// # Arguments
/// * `context_window` - The provider's context window size in tokens
///
/// # Returns
/// * The budget in tokens to target after compaction
///
/// # Examples
/// ```
/// use codelet_cli::compaction_threshold::calculate_summarization_budget;
///
/// // Claude with 200k context
/// let budget = calculate_summarization_budget(200_000);
/// assert_eq!(budget, 150_000); // 200k - 50k
///
/// // Small context window (40k)
/// let budget = calculate_summarization_budget(40_000);
/// assert_eq!(budget, 32_000); // 40k * 0.8
/// ```
pub fn calculate_summarization_budget(context_window: u64) -> u64 {
    if context_window <= AUTOCOMPACT_BUFFER {
        (context_window as f64 * 0.8) as u64
    } else {
        context_window - AUTOCOMPACT_BUFFER
    }
}

// =============================================================================
// CTX-002: Optimized Compaction Window Limit Trigger
// =============================================================================

/// Session output token maximum
///
/// This constant caps the output reservation to prevent over-reserving context
/// for models with very large max_output values. Set to 32k tokens as a
/// reasonable upper bound for output reservation.
pub const SESSION_OUTPUT_TOKEN_MAX: u64 = 32_000;

/// Calculate usable context given model limits
///
/// Algorithm:
/// 1. output_reservation = min(model_max_output, SESSION_OUTPUT_TOKEN_MAX)
/// 2. If output_reservation == 0, use SESSION_OUTPUT_TOKEN_MAX as fallback
/// 3. usable_context = context_window - output_reservation
///
/// # Arguments
/// * `context_window` - The model's total context window
/// * `model_max_output` - The model's maximum output tokens (0 if unknown)
///
/// # Returns
/// * The usable context in tokens (space available for input/cache/output)
pub fn calculate_usable_context(context_window: u64, model_max_output: u64) -> u64 {
    let output_reservation = model_max_output.min(SESSION_OUTPUT_TOKEN_MAX);
    let output_reservation = if output_reservation == 0 {
        SESSION_OUTPUT_TOKEN_MAX
    } else {
        output_reservation
    };
    context_window.saturating_sub(output_reservation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_defined() {
        assert_eq!(AUTOCOMPACT_BUFFER, 50_000);
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);
    }

    // =========================================================================
    // CTX-002 Tests: Optimized Compaction Window Limit Trigger
    // Feature: spec/features/optimized-compaction-trigger.feature
    // =========================================================================

    // -------------------------------------------------------------------------
    // Scenario: Calculate usable context for Claude Sonnet
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_claude_sonnet() {
        // @step Given a model with context_window of 200000 tokens
        let context_window = 200_000;

        // @step And the model has max_output_tokens of 8192
        let max_output = 8_192;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 191808 tokens
        // 200,000 - min(8,192, 32,000) = 200,000 - 8,192 = 191,808
        assert_eq!(usable, 191_808);
    }

    // -------------------------------------------------------------------------
    // Scenario: Calculate usable context for GPT-4
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_gpt4() {
        // @step Given a model with context_window of 128000 tokens
        let context_window = 128_000;

        // @step And the model has max_output_tokens of 4096
        let max_output = 4_096;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 123904 tokens
        // 128,000 - min(4,096, 32,000) = 128,000 - 4,096 = 123,904
        assert_eq!(usable, 123_904);
    }

    // -------------------------------------------------------------------------
    // Scenario: SESSION_OUTPUT_MAX caps high-output models
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_high_output_capped() {
        // @step Given a model with context_window of 200000 tokens
        let context_window = 200_000;

        // @step And the model has max_output_tokens of 64000
        let max_output = 64_000;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 168000 tokens
        // 200,000 - min(64,000, 32,000) = 200,000 - 32,000 = 168,000
        assert_eq!(usable, 168_000);
    }

    // -------------------------------------------------------------------------
    // Scenario: Unknown model with zero max_output uses SESSION_OUTPUT_MAX fallback
    // -------------------------------------------------------------------------
    #[test]
    fn test_usable_context_zero_max_output_fallback() {
        // @step Given a model with context_window of 100000 tokens
        let context_window = 100_000;

        // @step And the model has max_output_tokens of 0
        let max_output = 0;

        // @step And SESSION_OUTPUT_TOKEN_MAX is 32000
        assert_eq!(SESSION_OUTPUT_TOKEN_MAX, 32_000);

        // @step When I calculate usable context
        let usable = calculate_usable_context(context_window, max_output);

        // @step Then usable context should be 68000 tokens
        // min(0, 32000) = 0, but 0 triggers fallback to SESSION_OUTPUT_MAX
        // usable = 100,000 - 32,000 = 68,000 (NOT 100,000)
        assert_eq!(usable, 68_000);
    }

    #[test]
    fn test_calculate_budget_large_context() {
        // Large context: 200k
        let budget = calculate_summarization_budget(200_000);
        assert_eq!(budget, 150_000); // 200k - 50k
    }

    #[test]
    fn test_calculate_budget_small_context() {
        // Small context: 40k (less than buffer)
        let budget = calculate_summarization_budget(40_000);
        assert_eq!(budget, 32_000); // 40k * 0.8
    }

    #[test]
    fn test_calculate_budget_equal_to_buffer() {
        // Context equals buffer: 50k
        let budget = calculate_summarization_budget(50_000);
        assert_eq!(budget, 40_000); // 50k * 0.8
    }
}
