//! Display Throttle
//!
//! Throttles UI updates to prevent flicker during rapid streaming.

use std::time::{Duration, Instant};

/// Throttles display updates to prevent UI flicker.
///
/// During streaming, chunks can arrive very rapidly (10-50ms intervals).
/// Updating the UI on every chunk causes visual flicker and wastes CPU.
/// This throttle ensures updates happen at a reasonable rate (e.g., 5 times/second).
///
/// ## Example
///
/// ```
/// use codelet_core::streaming_display::DisplayThrottle;
/// use std::time::Duration;
///
/// let mut throttle = DisplayThrottle::new(Duration::from_millis(200));
///
/// // First call always returns true
/// assert!(throttle.should_emit());
///
/// // Immediate second call returns false (throttled)
/// assert!(!throttle.should_emit());
///
/// // After waiting the interval, returns true again
/// // std::thread::sleep(Duration::from_millis(200));
/// // assert!(throttle.should_emit());
/// ```
#[derive(Debug, Clone)]
pub struct DisplayThrottle {
    /// Last time an emission was allowed
    last_emit: Option<Instant>,
    /// Minimum interval between emissions
    interval: Duration,
}

impl DisplayThrottle {
    /// Default throttle interval (200ms = 5 updates/second).
    pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(200);

    /// Create a new throttle with the specified interval.
    ///
    /// # Arguments
    /// * `interval` - Minimum time between allowed emissions
    pub fn new(interval: Duration) -> Self {
        Self {
            last_emit: None,
            interval,
        }
    }

    /// Create a throttle with the default interval (200ms).
    pub fn with_default_interval() -> Self {
        Self::new(Self::DEFAULT_INTERVAL)
    }

    /// Check if enough time has passed to emit.
    ///
    /// Returns `true` if emission should proceed, `false` if throttled.
    /// When returning `true`, updates the last emit time.
    pub fn should_emit(&mut self) -> bool {
        let now = Instant::now();
        match self.last_emit {
            Some(last) if now.duration_since(last) < self.interval => false,
            _ => {
                self.last_emit = Some(now);
                true
            }
        }
    }

    /// Check if would emit without updating state.
    ///
    /// Useful for checking throttle state without side effects.
    pub fn would_emit(&self) -> bool {
        let now = Instant::now();
        match self.last_emit {
            Some(last) => now.duration_since(last) >= self.interval,
            None => true,
        }
    }

    /// Force the next `should_emit()` to return true.
    ///
    /// Useful when you want to force an immediate update regardless of throttle.
    pub fn reset(&mut self) {
        self.last_emit = None;
    }

    /// Get the throttle interval.
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Update the throttle interval.
    ///
    /// Does not reset the last emit time.
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Get time until next emission is allowed.
    ///
    /// Returns `Duration::ZERO` if emission is allowed now.
    pub fn time_until_next_emit(&self) -> Duration {
        let now = Instant::now();
        match self.last_emit {
            Some(last) => {
                let elapsed = now.duration_since(last);
                if elapsed >= self.interval {
                    Duration::ZERO
                } else {
                    self.interval - elapsed
                }
            }
            None => Duration::ZERO,
        }
    }
}

impl Default for DisplayThrottle {
    fn default() -> Self {
        Self::with_default_interval()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_first_call_always_emits() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(100));
        assert!(throttle.should_emit());
    }

    #[test]
    fn test_immediate_second_call_throttled() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(100));
        throttle.should_emit(); // First call
        assert!(!throttle.should_emit()); // Immediate second call
    }

    #[test]
    fn test_emits_after_interval() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(50));
        throttle.should_emit(); // First call

        sleep(Duration::from_millis(60)); // Wait longer than interval

        assert!(throttle.should_emit()); // Should emit now
    }

    #[test]
    fn test_would_emit_doesnt_change_state() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(100));
        throttle.should_emit(); // First call, updates state

        // would_emit should return false (still throttled)
        assert!(!throttle.would_emit());
        // State unchanged, should_emit still returns false
        assert!(!throttle.should_emit());
    }

    #[test]
    fn test_reset_allows_immediate_emit() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(1000));
        throttle.should_emit(); // First call

        // Would normally be throttled
        assert!(!throttle.should_emit());

        // Reset allows immediate emit
        throttle.reset();
        assert!(throttle.should_emit());
    }

    #[test]
    fn test_interval_getter() {
        let throttle = DisplayThrottle::new(Duration::from_millis(250));
        assert_eq!(throttle.interval(), Duration::from_millis(250));
    }

    #[test]
    fn test_set_interval() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(100));
        throttle.set_interval(Duration::from_millis(500));
        assert_eq!(throttle.interval(), Duration::from_millis(500));
    }

    #[test]
    fn test_time_until_next_emit_when_can_emit() {
        let throttle = DisplayThrottle::new(Duration::from_millis(100));
        // Never emitted, so can emit now
        assert_eq!(throttle.time_until_next_emit(), Duration::ZERO);
    }

    #[test]
    fn test_time_until_next_emit_when_throttled() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(100));
        throttle.should_emit(); // Emit now

        // Should have some time until next emit
        let time_remaining = throttle.time_until_next_emit();
        assert!(time_remaining > Duration::ZERO);
        assert!(time_remaining <= Duration::from_millis(100));
    }

    #[test]
    fn test_default_uses_default_interval() {
        let throttle = DisplayThrottle::default();
        assert_eq!(throttle.interval(), DisplayThrottle::DEFAULT_INTERVAL);
    }

    #[test]
    fn test_with_default_interval() {
        let throttle = DisplayThrottle::with_default_interval();
        assert_eq!(throttle.interval(), Duration::from_millis(200));
    }

    #[test]
    fn test_multiple_emissions_over_time() {
        let mut throttle = DisplayThrottle::new(Duration::from_millis(30));
        let mut emit_count = 0;

        // First emission
        if throttle.should_emit() {
            emit_count += 1;
        }

        // Try to emit rapidly - should be throttled
        for _ in 0..5 {
            if throttle.should_emit() {
                emit_count += 1;
            }
        }
        assert_eq!(emit_count, 1);

        // Wait and emit again
        sleep(Duration::from_millis(40));
        if throttle.should_emit() {
            emit_count += 1;
        }
        assert_eq!(emit_count, 2);
    }
}
