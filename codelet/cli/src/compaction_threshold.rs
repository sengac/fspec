//! Compaction Threshold Calculation (CLI-020)
//!
//! Provides constants and functions for calculating when context compaction
//! should be triggered, including the autocompact buffer that leaves headroom
//! after compaction to reduce re-compaction frequency.
//!
//! Reference: spec/features/autocompact-buffer.feature

/// Autocompact buffer in tokens
///
/// This buffer is subtracted from the context window before calculating
/// the compaction threshold. It ensures that after compaction, there's
/// headroom below the threshold to prevent immediate re-compaction on
/// the next turn.
///
/// Without this buffer:
/// - Compaction triggers at 90% of context window
/// - After compaction, context might be at 40-50%
/// - Next turn adds tokens, quickly hitting 90% again
/// - Results in frequent, disruptive re-compactions
///
/// With this buffer (50k tokens):
/// - Effective threshold is (context_window - 50k) * 0.9
/// - For 200k context: threshold becomes 135k instead of 180k
/// - After compaction, more headroom exists before next trigger
pub const AUTOCOMPACT_BUFFER: u64 = 50_000;

/// Compaction threshold ratio
///
/// The percentage of the summarization budget at which compaction triggers.
/// Value of 0.9 means compaction triggers when effective tokens exceed 90%
/// of the available budget (context_window - buffer).
pub const COMPACTION_THRESHOLD_RATIO: f64 = 0.9;

/// Calculate the compaction threshold for a given context window
///
/// The threshold is calculated as:
/// `(context_window - AUTOCOMPACT_BUFFER) * COMPACTION_THRESHOLD_RATIO`
///
/// Uses saturating subtraction to prevent underflow if context_window
/// is smaller than the buffer.
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
/// assert_eq!(threshold, 135_000); // (200k - 50k) * 0.9
///
/// // OpenAI with 128k context
/// let threshold = calculate_compaction_threshold(128_000);
/// assert_eq!(threshold, 70_200); // (128k - 50k) * 0.9
/// ```
pub fn calculate_compaction_threshold(context_window: u64) -> u64 {
    let summarization_budget = context_window.saturating_sub(AUTOCOMPACT_BUFFER);
    (summarization_budget as f64 * COMPACTION_THRESHOLD_RATIO) as u64
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
        assert_eq!(threshold, 135_000);
    }

    #[test]
    fn test_calculate_threshold_openai() {
        // OpenAI: 128k context
        let threshold = calculate_compaction_threshold(128_000);
        assert_eq!(threshold, 70_200);
    }

    #[test]
    fn test_calculate_threshold_small_context() {
        // Small context: 60k
        let threshold = calculate_compaction_threshold(60_000);
        assert_eq!(threshold, 9_000); // (60k - 50k) * 0.9
    }

    #[test]
    fn test_calculate_threshold_equal_to_buffer() {
        // Context equals buffer: 50k
        let threshold = calculate_compaction_threshold(50_000);
        assert_eq!(threshold, 0);
    }

    #[test]
    fn test_calculate_threshold_smaller_than_buffer() {
        // Context smaller than buffer: 30k
        let threshold = calculate_compaction_threshold(30_000);
        assert_eq!(threshold, 0); // saturating_sub prevents underflow
    }
}
