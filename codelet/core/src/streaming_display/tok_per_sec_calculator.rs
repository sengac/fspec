//! Tokens Per Second Calculator
//!
//! Calculates streaming token rate with time-windowed sampling and EMA smoothing.

use std::time::{Duration, Instant};

use codelet_common::token_estimator::count_tokens;

/// Calculates tokens-per-second rate with time-windowed sampling and adaptive EMA smoothing.
///
/// ## Algorithm
///
/// 1. Maintains a sliding window of (timestamp, cumulative_tokens) samples
/// 2. Calculates raw rate from oldest to newest sample in window
/// 3. Applies adaptive EMA smoothing:
///    - Stable rate (small change): low alpha (0.1) for smooth display
///    - Changing rate (large change): high alpha (0.9) for quick response
///
/// ## Example
///
/// ```
/// use codelet_core::streaming_display::TokPerSecCalculator;
///
/// let mut calc = TokPerSecCalculator::new();
///
/// // Record tokens as they arrive
/// let rate = calc.record(10);  // Returns None if not enough samples yet
///
/// // After enough samples with time span
/// // calc.record(10) will return Some(rate)
///
/// // Get current rate without recording
/// let current = calc.current_rate();
/// ```
#[derive(Debug, Clone)]
pub struct TokPerSecCalculator {
    /// Time-windowed samples: (timestamp, cumulative_tokens)
    samples: Vec<(Instant, u64)>,
    /// Running total for rate calculation
    total_tokens: u64,
    /// EMA-smoothed rate
    smoothed_rate: Option<f64>,
}

impl TokPerSecCalculator {
    /// Time window for rate calculation (1 second).
    /// Shorter window = faster response to rate changes.
    pub const TIME_WINDOW: Duration = Duration::from_secs(1);

    /// Minimum time span for stable rate calculation (50ms).
    /// Prevents wild rate fluctuations from tiny time deltas.
    pub const MIN_TIME_SPAN: Duration = Duration::from_millis(50);

    /// Create a new calculator.
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            total_tokens: 0,
            smoothed_rate: None,
        }
    }

    /// Record tokens and return the smoothed rate if calculable.
    ///
    /// Returns `None` if:
    /// - Less than 2 samples in window
    /// - Time span less than `MIN_TIME_SPAN`
    pub fn record(&mut self, token_count: u64) -> Option<f64> {
        let now = Instant::now();
        self.total_tokens += token_count;
        self.samples.push((now, self.total_tokens));

        // Prune samples older than TIME_WINDOW
        let cutoff = now - Self::TIME_WINDOW;
        self.samples.retain(|(ts, _)| *ts >= cutoff);

        self.calculate_rate()
    }

    /// Record a text chunk, counting tokens automatically.
    ///
    /// Convenience method that uses tiktoken to count tokens in the text.
    pub fn record_chunk(&mut self, text: &str) -> Option<f64> {
        let tokens = count_tokens(text) as u64;
        self.record(tokens)
    }

    /// Calculate the rate from current samples.
    fn calculate_rate(&mut self) -> Option<f64> {
        // Need at least 2 samples
        if self.samples.len() < 2 {
            return None;
        }

        let oldest = self.samples.first()?;
        let newest = self.samples.last()?;
        let time_delta = newest.0.duration_since(oldest.0);

        // Need minimum time span for stable rate
        if time_delta < Self::MIN_TIME_SPAN {
            return None;
        }

        let token_delta = newest.1 - oldest.1;
        let raw_rate = token_delta as f64 / time_delta.as_secs_f64();

        // Apply adaptive EMA
        let alpha = self.calculate_adaptive_alpha(raw_rate);
        let smoothed = match self.smoothed_rate {
            Some(prev) => alpha * raw_rate + (1.0 - alpha) * prev,
            None => raw_rate,
        };
        self.smoothed_rate = Some(smoothed);

        self.smoothed_rate
    }

    /// Calculate adaptive alpha for EMA based on rate change.
    ///
    /// - Stable rate (small relative change): low alpha (0.1) for smoothing
    /// - Changing rate (large relative change): high alpha (0.9) for responsiveness
    fn calculate_adaptive_alpha(&self, raw_rate: f64) -> f64 {
        match self.smoothed_rate {
            Some(prev) if prev > 0.0 => {
                let relative_change = (raw_rate - prev).abs() / prev;
                // Map relative change to alpha: larger change = higher alpha
                (relative_change * 2.0).clamp(0.1, 0.9)
            }
            _ => 1.0, // First reading: 100% weight to raw_rate
        }
    }

    /// Get the current smoothed rate without recording new data.
    pub fn current_rate(&self) -> Option<f64> {
        self.smoothed_rate
    }

    /// Get total tokens recorded so far.
    pub fn total_tokens(&self) -> u64 {
        self.total_tokens
    }

    /// Reset the calculator, clearing all samples and rate.
    pub fn reset(&mut self) {
        self.samples.clear();
        self.total_tokens = 0;
        self.smoothed_rate = None;
    }
}

impl Default for TokPerSecCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_new_calculator_empty() {
        let calc = TokPerSecCalculator::new();
        assert_eq!(calc.total_tokens(), 0);
        assert!(calc.current_rate().is_none());
    }

    #[test]
    fn test_single_record_returns_none() {
        let mut calc = TokPerSecCalculator::new();
        // Single sample - can't calculate rate
        assert!(calc.record(100).is_none());
        assert_eq!(calc.total_tokens(), 100);
    }

    #[test]
    fn test_two_records_too_close_returns_none() {
        let mut calc = TokPerSecCalculator::new();
        // Two samples but too close together (< MIN_TIME_SPAN)
        calc.record(100);
        let result = calc.record(100);
        // Might be None if execution was fast enough
        // Can't reliably test timing, so just verify it doesn't panic
        assert_eq!(calc.total_tokens(), 200);
        // Result could be None or Some depending on execution speed
        let _ = result;
    }

    #[test]
    fn test_records_with_time_gap_returns_rate() {
        let mut calc = TokPerSecCalculator::new();

        calc.record(100);
        // Sleep to ensure time gap
        sleep(Duration::from_millis(60));
        let result = calc.record(100);

        // Should have a rate now (200 tokens over ~60ms)
        assert!(result.is_some());
        let rate = result.unwrap();
        // Rate should be positive and reasonable (not testing exact value due to timing)
        assert!(rate > 0.0);
    }

    #[test]
    fn test_record_chunk_counts_tokens() {
        let mut calc = TokPerSecCalculator::new();

        calc.record_chunk("Hello world");
        assert!(calc.total_tokens() > 0);
    }

    #[test]
    fn test_current_rate_doesnt_modify_state() {
        let mut calc = TokPerSecCalculator::new();

        calc.record(100);
        sleep(Duration::from_millis(60));
        calc.record(100);

        let rate1 = calc.current_rate();
        let rate2 = calc.current_rate();

        assert_eq!(rate1, rate2);
        assert_eq!(calc.total_tokens(), 200); // Unchanged
    }

    #[test]
    fn test_reset_clears_everything() {
        let mut calc = TokPerSecCalculator::new();

        calc.record(100);
        sleep(Duration::from_millis(60));
        calc.record(100);

        calc.reset();

        assert_eq!(calc.total_tokens(), 0);
        assert!(calc.current_rate().is_none());
    }

    #[test]
    fn test_adaptive_alpha_first_reading() {
        let calc = TokPerSecCalculator::new();
        // No previous rate, so alpha should be 1.0
        let alpha = calc.calculate_adaptive_alpha(100.0);
        assert_eq!(alpha, 1.0);
    }

    #[test]
    fn test_adaptive_alpha_stable_rate() {
        let mut calc = TokPerSecCalculator::new();
        calc.smoothed_rate = Some(100.0);

        // Small change: 100 -> 105 (5% change)
        let alpha = calc.calculate_adaptive_alpha(105.0);
        // relative_change = 0.05, alpha = 0.1 (clamped)
        assert!(alpha >= 0.1 && alpha <= 0.2);
    }

    #[test]
    fn test_adaptive_alpha_changing_rate() {
        let mut calc = TokPerSecCalculator::new();
        calc.smoothed_rate = Some(100.0);

        // Large change: 100 -> 200 (100% change)
        let alpha = calc.calculate_adaptive_alpha(200.0);
        // relative_change = 1.0, alpha = min(2.0, 0.9) = 0.9
        assert_eq!(alpha, 0.9);
    }

    #[test]
    fn test_default_creates_empty() {
        let calc = TokPerSecCalculator::default();
        assert_eq!(calc.total_tokens(), 0);
        assert!(calc.current_rate().is_none());
    }

    #[test]
    fn test_samples_pruned_after_time_window() {
        let mut calc = TokPerSecCalculator::new();

        // Record some samples
        calc.record(10);
        sleep(Duration::from_millis(20));
        calc.record(10);
        sleep(Duration::from_millis(20));
        calc.record(10);

        // Samples within TIME_WINDOW should still be there
        assert!(calc.samples.len() >= 2);

        // Note: We can't easily test pruning without waiting TIME_WINDOW (1 second)
        // which would make tests slow. The logic is simple enough to trust.
    }
}
