//! Streaming Token Display
//!
//! Composes all token display components for easy use in stream loops.

use super::{DisplayThrottle, OutputTokenTracker, TokPerSecCalculator};
use rig::completion::Usage;
use std::time::Duration;

/// Token display update to emit to the UI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenDisplayUpdate {
    /// Input tokens (total context)
    pub input_tokens: u64,
    /// Output tokens (cumulative across segments)
    pub output_tokens: u64,
    /// Cache read tokens
    pub cache_read_tokens: u64,
    /// Cache creation tokens
    pub cache_creation_tokens: u64,
    /// Tokens per second rate (if available)
    pub tokens_per_second: Option<f64>,
}

impl TokenDisplayUpdate {
    /// Create a new update with all fields.
    pub fn new(
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_creation_tokens: u64,
        tokens_per_second: Option<f64>,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_creation_tokens,
            tokens_per_second,
        }
    }

    /// Calculate total input (input + cache_read + cache_creation).
    pub fn total_input(&self) -> u64 {
        self.input_tokens + self.cache_read_tokens + self.cache_creation_tokens
    }

    /// Calculate total context (total_input + output).
    pub fn total_context(&self) -> u64 {
        self.total_input() + self.output_tokens
    }
}

/// Manages all token display concerns during streaming.
///
/// This struct composes [`OutputTokenTracker`], [`TokPerSecCalculator`], and
/// [`DisplayThrottle`] to provide a simple API for tracking and displaying
/// token information during streaming responses.
///
/// ## Provider Differences
///
/// - **Anthropic/Gemini**: Send `Usage` events during streaming, providing
///   authoritative values mid-stream. Call `update_from_usage()` when these arrive.
///
/// - **OpenAI/Z.AI**: Only send usage in `FinalResponse`. During streaming,
///   `record_chunk()` provides estimated values until `update_from_final_response()`
///   sets authoritative values.
///
/// ## Example
///
/// ```ignore
/// use codelet_core::streaming_display::StreamingTokenDisplay;
///
/// let mut display = StreamingTokenDisplay::new(prev_input, prev_output, cache_read, cache_creation);
///
/// // During streaming
/// if let Some(update) = display.record_chunk(&text) {
///     output.emit_tokens(&update.into());
/// }
///
/// // When Usage event arrives (Anthropic: output > 0)
/// if let Some(update) = display.update_from_usage(&usage) {
///     output.emit_tokens(&update.into());
/// }
///
/// // When new API segment starts (MessageStart)
/// display.start_new_segment(&usage);
/// ```
#[derive(Debug, Clone)]
pub struct StreamingTokenDisplay {
    /// Output token tracking (estimated vs authoritative)
    output: OutputTokenTracker,
    /// Input tokens (last known value)
    input_tokens: u64,
    /// Cache read tokens (last known value)
    cache_read_tokens: u64,
    /// Cache creation tokens (last known value)
    cache_creation_tokens: u64,
    /// Previous input tokens (for OpenAI-compatible fallback)
    prev_input_tokens: u64,
    /// Rate calculation
    rate: TokPerSecCalculator,
    /// Display throttling
    throttle: DisplayThrottle,
}

impl StreamingTokenDisplay {
    /// Create a new streaming token display with initial values.
    ///
    /// # Arguments
    /// * `prev_input` - Previous session's input tokens (total context)
    /// * `prev_output` - Previous session's output tokens (cumulative)
    /// * `prev_cache_read` - Previous cache read tokens
    /// * `prev_cache_creation` - Previous cache creation tokens
    pub fn new(
        prev_input: u64,
        prev_output: u64,
        prev_cache_read: u64,
        prev_cache_creation: u64,
    ) -> Self {
        Self {
            output: OutputTokenTracker::new(prev_output),
            input_tokens: prev_input,
            cache_read_tokens: prev_cache_read,
            cache_creation_tokens: prev_cache_creation,
            prev_input_tokens: prev_input,
            rate: TokPerSecCalculator::new(),
            throttle: DisplayThrottle::with_default_interval(),
        }
    }

    /// Create with a custom throttle interval.
    pub fn with_throttle_interval(
        prev_input: u64,
        prev_output: u64,
        prev_cache_read: u64,
        prev_cache_creation: u64,
        throttle_interval: Duration,
    ) -> Self {
        let mut display = Self::new(prev_input, prev_output, prev_cache_read, prev_cache_creation);
        display.throttle = DisplayThrottle::new(throttle_interval);
        display
    }

    /// Record a text chunk during streaming.
    ///
    /// Updates output estimate and rate calculation.
    /// Returns `Some(update)` if throttle allows, `None` if throttled.
    pub fn record_chunk(&mut self, text: &str) -> Option<TokenDisplayUpdate> {
        // Update output estimate
        self.output.record_chunk(text);

        // Update rate calculation
        let rate = self.rate.record_chunk(text);

        // Throttle display updates
        if self.throttle.should_emit() {
            Some(self.current_display(rate))
        } else {
            None
        }
    }

    /// Update from a Usage event (Anthropic MessageDelta with output > 0).
    ///
    /// Sets authoritative values for input, output, and cache tokens.
    /// Always returns an update (not throttled) since Usage events are authoritative.
    pub fn update_from_usage(&mut self, usage: &Usage) -> Option<TokenDisplayUpdate> {
        // Update authoritative values
        if usage.input_tokens > 0 {
            self.input_tokens = usage.input_tokens;
        }
        self.output.update_from_usage(usage.output_tokens);

        if let Some(cr) = usage.cache_read_input_tokens {
            self.cache_read_tokens = cr;
        }
        if let Some(cc) = usage.cache_creation_input_tokens {
            self.cache_creation_tokens = cc;
        }

        // Always emit on Usage events (authoritative data)
        Some(self.current_display(self.rate.current_rate()))
    }

    /// Handle MessageStart (new API segment starting).
    ///
    /// Called when a Usage event arrives with `output_tokens == 0`, indicating
    /// a new API call is starting (common in multi-turn tool loops).
    ///
    /// Does NOT emit an update (to avoid display "bouncing").
    pub fn start_new_segment(&mut self, usage: &Usage) {
        // Accumulate previous segment before reset
        self.output.start_new_segment();

        // Update input and cache from new segment
        // Only update input_tokens if the new value is > 0, otherwise keep previous
        // (matching the original TokPerSecTracker.calculate_display_tokens behavior)
        if usage.input_tokens > 0 {
            self.input_tokens = usage.input_tokens;
        }
        if let Some(cr) = usage.cache_read_input_tokens {
            self.cache_read_tokens = cr;
        }
        if let Some(cc) = usage.cache_creation_input_tokens {
            self.cache_creation_tokens = cc;
        }

        // Don't emit on MessageStart (causes bouncing)
    }

    /// Update from FinalResponse (OpenAI-compatible providers).
    ///
    /// For providers that only report usage in FinalResponse, this sets
    /// authoritative values for both input and output.
    ///
    /// Returns an update for display.
    pub fn update_from_final_response(&mut self, usage: &Usage) -> TokenDisplayUpdate {
        // Set authoritative values
        self.input_tokens = usage.input_tokens;
        self.output.update_from_usage(usage.output_tokens);

        if let Some(cr) = usage.cache_read_input_tokens {
            self.cache_read_tokens = cr;
        }
        if let Some(cc) = usage.cache_creation_input_tokens {
            self.cache_creation_tokens = cc;
        }

        self.current_display(self.rate.current_rate())
    }

    /// Get current display values without recording new data.
    fn current_display(&self, rate: Option<f64>) -> TokenDisplayUpdate {
        TokenDisplayUpdate {
            input_tokens: self.effective_input_tokens(),
            output_tokens: self.output.display_tokens(),
            cache_read_tokens: self.cache_read_tokens,
            cache_creation_tokens: self.cache_creation_tokens,
            tokens_per_second: rate,
        }
    }

    /// Get effective input tokens (use actual if available, else fallback to prev).
    ///
    /// This handles OpenAI-compatible providers that don't send Usage events
    /// during streaming - we fall back to the previous session's input tokens.
    fn effective_input_tokens(&self) -> u64 {
        if self.input_tokens > 0 {
            self.input_tokens
        } else {
            self.prev_input_tokens
        }
    }

    /// Get current display update (for final emit or debugging).
    pub fn current(&self) -> TokenDisplayUpdate {
        self.current_display(self.rate.current_rate())
    }

    /// Get the current tok/s rate.
    pub fn current_rate(&self) -> Option<f64> {
        self.rate.current_rate()
    }

    /// Get the current output tokens (display value).
    pub fn output_tokens(&self) -> u64 {
        self.output.display_tokens()
    }

    /// Get the current input tokens.
    pub fn input_tokens(&self) -> u64 {
        self.effective_input_tokens()
    }

    /// Get the cumulative base (output tokens from completed segments).
    pub fn cumulative_output_base(&self) -> u64 {
        self.output.cumulative_base()
    }

    /// Force the next `record_chunk()` to emit (reset throttle).
    pub fn force_next_emit(&mut self) {
        self.throttle.reset();
    }

    /// Check if output has authoritative data.
    pub fn has_authoritative_output(&self) -> bool {
        self.output.has_authoritative()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn make_usage(input: u64, output: u64, cache_read: Option<u64>, cache_creation: Option<u64>) -> Usage {
        Usage {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
            cache_read_input_tokens: cache_read,
            cache_creation_input_tokens: cache_creation,
        }
    }

    #[test]
    fn test_new_with_initial_values() {
        let display = StreamingTokenDisplay::new(1000, 500, 100, 50);

        assert_eq!(display.input_tokens(), 1000);
        assert_eq!(display.output_tokens(), 500);

        let current = display.current();
        assert_eq!(current.input_tokens, 1000);
        assert_eq!(current.output_tokens, 500);
        assert_eq!(current.cache_read_tokens, 100);
        assert_eq!(current.cache_creation_tokens, 50);
    }

    #[test]
    fn test_record_chunk_updates_output() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // Record a chunk
        display.output.record_chunk("Hello world test");

        // Output should have estimate now
        assert!(display.output_tokens() > 0);
    }

    #[test]
    fn test_record_chunk_throttled() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // First call should emit (throttle allows first)
        let result1 = display.record_chunk("Hello");
        assert!(result1.is_some());

        // Immediate second call should be throttled
        let result2 = display.record_chunk("World");
        assert!(result2.is_none());

        // Output still updated even when throttled
        assert!(display.output_tokens() > 0);
    }

    #[test]
    fn test_update_from_usage_sets_authoritative() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // First, record some chunks (estimate)
        display.output.record_chunk("Some text here");
        assert!(!display.has_authoritative_output());

        // Then receive Usage event
        let usage = make_usage(1500, 50, Some(200), Some(100));
        let update = display.update_from_usage(&usage);

        assert!(update.is_some());
        assert!(display.has_authoritative_output());

        let update = update.unwrap();
        assert_eq!(update.input_tokens, 1500);
        assert_eq!(update.cache_read_tokens, 200);
        assert_eq!(update.cache_creation_tokens, 100);
    }

    #[test]
    fn test_update_from_usage_always_emits() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // Record chunk (uses throttle)
        let _ = display.record_chunk("Hello");

        // Immediate Usage update should still emit (not throttled)
        let usage = make_usage(1500, 50, None, None);
        let update = display.update_from_usage(&usage);
        assert!(update.is_some());
    }

    #[test]
    fn test_start_new_segment_accumulates() {
        let mut display = StreamingTokenDisplay::new(1000, 100, 0, 0);

        // First segment: add some output
        display.output.record_tokens(50);
        display.output.update_from_usage(45);

        // Start new segment
        let usage = make_usage(1200, 0, Some(50), None);
        display.start_new_segment(&usage);

        // Cumulative should include previous segment (45 authoritative)
        // Plus initial 100 = 145
        assert_eq!(display.cumulative_output_base(), 100 + 45);
        assert_eq!(display.input_tokens(), 1200);
    }

    #[test]
    fn test_update_from_final_response() {
        let mut display = StreamingTokenDisplay::new(0, 0, 0, 0);

        // OpenAI-style: no Usage events during streaming
        display.output.record_chunk("Lots of text here for estimation");

        // FinalResponse arrives
        let usage = make_usage(5000, 200, Some(1000), Some(500));
        let update = display.update_from_final_response(&usage);

        assert_eq!(update.input_tokens, 5000);
        // Output is max(estimate, authoritative)
        assert!(update.output_tokens >= 200);
        assert_eq!(update.cache_read_tokens, 1000);
        assert_eq!(update.cache_creation_tokens, 500);
    }

    #[test]
    fn test_effective_input_fallback() {
        // Start with prev_input = 1000
        let display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // Input tokens should be 1000 (from prev)
        assert_eq!(display.input_tokens(), 1000);
    }

    #[test]
    fn test_effective_input_uses_actual() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // Update with actual input
        let usage = make_usage(2000, 0, None, None);
        display.start_new_segment(&usage);

        // Should use actual (2000), not prev (1000)
        assert_eq!(display.input_tokens(), 2000);
    }

    #[test]
    fn test_token_display_update_totals() {
        let update = TokenDisplayUpdate::new(1000, 500, 200, 100, Some(50.0));

        // total_input = input + cache_read + cache_creation
        assert_eq!(update.total_input(), 1300);

        // total_context = total_input + output
        assert_eq!(update.total_context(), 1800);
    }

    #[test]
    fn test_force_next_emit() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 0, 0);

        // First emit
        let _ = display.record_chunk("Hello");

        // Would be throttled
        let throttled = display.record_chunk("World");
        assert!(throttled.is_none());

        // Force next emit
        display.force_next_emit();

        // Now should emit
        let forced = display.record_chunk("!");
        assert!(forced.is_some());
    }

    #[test]
    fn test_current_rate_initially_none() {
        let display = StreamingTokenDisplay::new(1000, 0, 0, 0);
        assert!(display.current_rate().is_none());
    }

    #[test]
    fn test_multi_segment_flow() {
        let mut display = StreamingTokenDisplay::new(1000, 0, 50, 25);

        // Segment 1: User prompt + assistant response
        display.output.record_tokens(100);
        let usage1 = make_usage(1500, 95, Some(100), Some(50));
        display.update_from_usage(&usage1);

        // Tool call starts new segment
        let usage2 = make_usage(1600, 0, Some(150), Some(75));
        display.start_new_segment(&usage2);

        // Segment 2: Tool result + assistant response
        display.output.record_tokens(80);
        let usage3 = make_usage(2000, 75, Some(200), Some(100));
        let update = display.update_from_usage(&usage3);

        let update = update.unwrap();
        assert_eq!(update.input_tokens, 2000);
        // Output: cumulative(95) + max(80, 75) = 95 + 80 = 175
        assert_eq!(update.output_tokens, 95 + 80);
        assert_eq!(update.cache_read_tokens, 200);
        assert_eq!(update.cache_creation_tokens, 100);
    }

    #[test]
    fn test_with_custom_throttle() {
        let display = StreamingTokenDisplay::with_throttle_interval(
            1000,
            0,
            0,
            0,
            Duration::from_millis(500),
        );
        assert_eq!(display.throttle.interval(), Duration::from_millis(500));
    }

    #[test]
    fn test_start_new_segment_preserves_input_when_zero() {
        // BUG FIX: If start_new_segment is called with input_tokens=0,
        // it should preserve the previous input_tokens, not overwrite to 0
        let mut display = StreamingTokenDisplay::new(50000, 0, 0, 0);

        // Verify initial state
        assert_eq!(display.input_tokens(), 50000);

        // Start new segment with input_tokens=0 (edge case)
        let usage = make_usage(0, 0, None, None);
        display.start_new_segment(&usage);

        // Should preserve previous input_tokens (50000), not overwrite to 0
        assert_eq!(display.input_tokens(), 50000);
    }

    #[test]
    fn test_start_new_segment_updates_input_when_positive() {
        let mut display = StreamingTokenDisplay::new(50000, 0, 0, 0);

        // Start new segment with positive input_tokens
        let usage = make_usage(60000, 0, None, None);
        display.start_new_segment(&usage);

        // Should update to new value
        assert_eq!(display.input_tokens(), 60000);
    }
}
