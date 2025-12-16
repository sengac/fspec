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
/// - Threshold (when to compact): contextWindow * 0.9 (e.g., 180k for 200k window)
/// - Budget (target after compact): contextWindow - AUTOCOMPACT_BUFFER (e.g., 150k for 200k window)
/// - Headroom created: threshold - budget (e.g., 180k - 150k = 30k tokens)
///
/// This separation ensures:
/// - Compaction triggers at a consistent percentage (90%) of context window
/// - After compaction, there's 30k+ tokens of headroom before next trigger
/// - Reduces re-compaction frequency by providing buffer space for new interactions
///
/// Example for 200k context window:
/// - Trigger threshold: 180,000 tokens (200k * 0.9)
/// - Target after compaction: 150,000 tokens (200k - 50k)
/// - Headroom before re-trigger: 30,000 tokens
pub const AUTOCOMPACT_BUFFER: u64 = 50_000;

/// Compaction threshold ratio
///
/// The percentage of the context window at which compaction triggers.
/// Value of 0.9 means compaction triggers when effective tokens exceed 90%
/// of the context window.
///
/// IMPORTANT: This ratio applies to the context window, NOT the budget.
/// - Threshold = contextWindow * COMPACTION_THRESHOLD_RATIO
/// - Budget = contextWindow - AUTOCOMPACT_BUFFER (calculated separately)
///
/// Example: For a 200k context window
/// - Threshold = 200k * 0.9 = 180k tokens (when compaction triggers)
/// - Budget = 200k - 50k = 150k tokens (target size after compaction)
pub const COMPACTION_THRESHOLD_RATIO: f64 = 0.9;

/// Calculate the compaction threshold for a given context window
///
/// Matches TypeScript implementation in runner.ts:getCompactionThreshold()
///
/// The threshold is calculated as:
/// `context_window * COMPACTION_THRESHOLD_RATIO`
///
/// This determines when compaction should trigger based on effective tokens.
///
/// # Arguments
/// * `context_window` - The provider's context window size in tokens
///
/// # Returns
/// * The threshold in tokens above which compaction should trigger
///
/// # Examples
/// ```
/// use codelet_cli::compaction_threshold::calculate_compaction_threshold;
///
/// // Claude with 200k context
/// let threshold = calculate_compaction_threshold(200_000);
/// assert_eq!(threshold, 180_000); // 200k * 0.9
///
/// // OpenAI with 128k context
/// let threshold = calculate_compaction_threshold(128_000);
/// assert_eq!(threshold, 115_200); // 128k * 0.9
/// ```
pub fn calculate_compaction_threshold(context_window: u64) -> u64 {
    (context_window as f64 * COMPACTION_THRESHOLD_RATIO) as u64
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_defined() {
        assert_eq!(AUTOCOMPACT_BUFFER, 50_000);
        assert!((COMPACTION_THRESHOLD_RATIO - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_threshold_claude() {
        // Claude: 200k context
        let threshold = calculate_compaction_threshold(200_000);
        assert_eq!(threshold, 180_000); // 200k * 0.9
    }

    #[test]
    fn test_calculate_threshold_openai() {
        // OpenAI: 128k context
        let threshold = calculate_compaction_threshold(128_000);
        assert_eq!(threshold, 115_200); // 128k * 0.9
    }

    #[test]
    fn test_calculate_threshold_small_context() {
        // Small context: 60k
        let threshold = calculate_compaction_threshold(60_000);
        assert_eq!(threshold, 54_000); // 60k * 0.9
    }

    #[test]
    fn test_calculate_threshold_equal_to_buffer() {
        // Context equals buffer: 50k
        let threshold = calculate_compaction_threshold(50_000);
        assert_eq!(threshold, 45_000); // 50k * 0.9
    }

    #[test]
    fn test_calculate_threshold_smaller_than_buffer() {
        // Context smaller than buffer: 30k
        let threshold = calculate_compaction_threshold(30_000);
        assert_eq!(threshold, 27_000); // 30k * 0.9
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
