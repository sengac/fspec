//! Output Token Tracker
//!
//! Tracks output tokens with explicit estimated vs authoritative states.
//! Handles the gap between streaming chunks and API usage events.

use codelet_common::token_estimator::count_tokens;

/// Tracks output tokens with explicit estimated vs authoritative states.
///
/// During streaming, text chunks arrive before (or without) `Usage` events.
/// This tracker maintains both an estimate (from tiktoken) and authoritative
/// values (from API), using the best available value for display.
///
/// ## State Transitions
///
/// ```text
/// record_chunk()     → Updates estimate (always)
/// update_from_usage() → Sets authoritative value
/// start_new_segment() → Accumulates and resets for next API call
/// display_tokens()    → Returns max(estimate, authoritative) + cumulative_base
/// ```
///
/// ## Example
///
/// ```
/// use codelet_core::streaming_display::OutputTokenTracker;
///
/// let mut tracker = OutputTokenTracker::new(0);
///
/// // Streaming chunks arrive
/// tracker.record_chunk("Hello world");
/// assert!(tracker.display_tokens() > 0); // Has estimate
///
/// // Usage event arrives with authoritative value
/// tracker.update_from_usage(5);
/// assert_eq!(tracker.display_tokens(), 5); // Uses max(estimate, auth)
/// ```
#[derive(Debug, Clone)]
pub struct OutputTokenTracker {
    /// Estimated tokens from tiktoken (always updated on chunks)
    estimated: u64,
    /// Authoritative tokens from API (updated when Usage events arrive)
    authoritative: Option<u64>,
    /// Cumulative output across API segments (for multi-turn tool loops)
    cumulative_base: u64,
}

impl OutputTokenTracker {
    /// Create a new tracker with initial cumulative value.
    ///
    /// # Arguments
    /// * `initial_cumulative` - Starting cumulative output (e.g., from previous turns)
    pub fn new(initial_cumulative: u64) -> Self {
        Self {
            estimated: 0,
            authoritative: None,
            cumulative_base: initial_cumulative,
        }
    }

    /// Record a text chunk, updating the estimate.
    ///
    /// This should be called for every text/reasoning chunk during streaming.
    pub fn record_chunk(&mut self, text: &str) {
        self.estimated += count_tokens(text) as u64;
    }

    /// Record a known token count (for when caller already counted).
    pub fn record_tokens(&mut self, tokens: u64) {
        self.estimated += tokens;
    }

    /// Update from API Usage event, setting the authoritative value.
    ///
    /// This is called when a `Usage` event arrives with `output_tokens > 0`.
    pub fn update_from_usage(&mut self, output_tokens: u64) {
        self.authoritative = Some(output_tokens);
    }

    /// Called when a new API segment starts (MessageStart with output == 0).
    ///
    /// Accumulates the previous segment's output to `cumulative_base` and resets
    /// the current segment tracking.
    pub fn start_new_segment(&mut self) {
        // Accumulate previous segment's output before reset
        // Use authoritative if available, otherwise use estimate
        let segment_output = self.authoritative.unwrap_or(self.estimated);
        self.cumulative_base += segment_output;

        // Reset for new segment
        self.estimated = 0;
        self.authoritative = None;
    }

    /// Get the display value for output tokens.
    ///
    /// Returns `cumulative_base + max(estimated, authoritative)` to ensure
    /// the display never "goes backwards" when authoritative values arrive
    /// that are lower than estimates.
    pub fn display_tokens(&self) -> u64 {
        let current_segment = match self.authoritative {
            Some(auth) => self.estimated.max(auth),
            None => self.estimated,
        };
        self.cumulative_base + current_segment
    }

    /// Get the cumulative base (tokens from completed segments).
    pub fn cumulative_base(&self) -> u64 {
        self.cumulative_base
    }

    /// Get the current estimate (this segment only).
    pub fn current_estimate(&self) -> u64 {
        self.estimated
    }

    /// Get the current authoritative value if available.
    pub fn current_authoritative(&self) -> Option<u64> {
        self.authoritative
    }

    /// Check if we have authoritative data for the current segment.
    pub fn has_authoritative(&self) -> bool {
        self.authoritative.is_some()
    }
}

impl Default for OutputTokenTracker {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tracker_starts_at_initial_cumulative() {
        let tracker = OutputTokenTracker::new(100);
        assert_eq!(tracker.cumulative_base(), 100);
        assert_eq!(tracker.current_estimate(), 0);
        assert_eq!(tracker.display_tokens(), 100);
    }

    #[test]
    fn test_record_chunk_updates_estimate() {
        let mut tracker = OutputTokenTracker::new(0);

        // Record some text - "Hello" is about 1 token
        tracker.record_chunk("Hello world, this is a test message");
        assert!(tracker.current_estimate() > 0);
        assert_eq!(tracker.display_tokens(), tracker.current_estimate());
    }

    #[test]
    fn test_record_tokens_directly() {
        let mut tracker = OutputTokenTracker::new(0);

        tracker.record_tokens(50);
        assert_eq!(tracker.current_estimate(), 50);
        assert_eq!(tracker.display_tokens(), 50);

        tracker.record_tokens(30);
        assert_eq!(tracker.current_estimate(), 80);
        assert_eq!(tracker.display_tokens(), 80);
    }

    #[test]
    fn test_update_from_usage_sets_authoritative() {
        let mut tracker = OutputTokenTracker::new(0);

        tracker.record_tokens(100); // Estimate
        tracker.update_from_usage(80); // Authoritative (lower)

        assert!(tracker.has_authoritative());
        assert_eq!(tracker.current_authoritative(), Some(80));
        // Display uses max(estimate, auth) = max(100, 80) = 100
        assert_eq!(tracker.display_tokens(), 100);
    }

    #[test]
    fn test_authoritative_higher_than_estimate() {
        let mut tracker = OutputTokenTracker::new(0);

        tracker.record_tokens(50); // Estimate
        tracker.update_from_usage(80); // Authoritative (higher)

        // Display uses max(estimate, auth) = max(50, 80) = 80
        assert_eq!(tracker.display_tokens(), 80);
    }

    #[test]
    fn test_start_new_segment_accumulates() {
        let mut tracker = OutputTokenTracker::new(0);

        // First segment
        tracker.record_tokens(100);
        tracker.update_from_usage(90); // Authoritative value
        assert_eq!(tracker.display_tokens(), 100); // max(100, 90)

        // Start new segment
        tracker.start_new_segment();

        // Cumulative should have the authoritative value (90)
        assert_eq!(tracker.cumulative_base(), 90);
        assert_eq!(tracker.current_estimate(), 0);
        assert!(!tracker.has_authoritative());
        assert_eq!(tracker.display_tokens(), 90);

        // Add more in second segment
        tracker.record_tokens(50);
        assert_eq!(tracker.display_tokens(), 140); // 90 + 50
    }

    #[test]
    fn test_start_new_segment_uses_estimate_when_no_authoritative() {
        let mut tracker = OutputTokenTracker::new(0);

        // First segment - only estimate, no authoritative
        tracker.record_tokens(100);
        assert!(!tracker.has_authoritative());

        // Start new segment
        tracker.start_new_segment();

        // Cumulative should have the estimate (100)
        assert_eq!(tracker.cumulative_base(), 100);
    }

    #[test]
    fn test_multi_segment_accumulation() {
        let mut tracker = OutputTokenTracker::new(50); // Start with 50 from previous turns

        // Segment 1
        tracker.record_tokens(100);
        tracker.update_from_usage(95);
        tracker.start_new_segment();
        assert_eq!(tracker.cumulative_base(), 50 + 95); // 145

        // Segment 2
        tracker.record_tokens(80);
        tracker.update_from_usage(75);
        tracker.start_new_segment();
        assert_eq!(tracker.cumulative_base(), 145 + 75); // 220

        // Segment 3 (ongoing)
        tracker.record_tokens(30);
        assert_eq!(tracker.display_tokens(), 220 + 30); // 250
    }

    #[test]
    fn test_default_starts_empty() {
        let tracker = OutputTokenTracker::default();
        assert_eq!(tracker.cumulative_base(), 0);
        assert_eq!(tracker.current_estimate(), 0);
        assert!(!tracker.has_authoritative());
        assert_eq!(tracker.display_tokens(), 0);
    }
}
