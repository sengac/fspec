//! Status display for agent processing
//!
//! Shows elapsed time and interruption instructions during agent execution.

use std::time::Instant;

/// Status display for agent processing
pub struct StatusDisplay {
    start_time: Instant,
}

impl StatusDisplay {
    /// Create new status display
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    /// Reset start time
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    /// Get current elapsed seconds
    pub fn elapsed_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Format status message
    pub fn format_status(&self) -> String {
        format!(
            "🔄 Processing request ({}s • ESC to interrupt)",
            self.elapsed_seconds()
        )
    }
}

impl Default for StatusDisplay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_status_display_elapsed() {
        let status = StatusDisplay::new();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(status.elapsed_seconds() == 0); // Less than 1 second

        let msg = status.format_status();
        assert!(msg.contains("🔄 Processing request"));
        assert!(msg.contains("ESC to interrupt"));
    }
}
