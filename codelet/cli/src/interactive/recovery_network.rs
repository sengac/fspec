//! Network error retry recovery.
//!
//! Handles detection of transient network/connection errors during SSE streaming
//! and provides retry logic with exponential backoff. When the HTTP connection to
//! the API drops mid-stream (e.g., "error sending request for url"), the stream
//! loop retries the request using the full conversation history rather than
//! terminating with a fatal error.
//!
//! The retry happens at the stream_loop level (not the SSE level) because chat
//! completion APIs are stateless — SSE reconnection would lose accumulated state.

use std::time::Duration;

/// Maximum number of consecutive network error retries per stream.
/// After this many retries without receiving any successful data, the error
/// is reported to the user as non-recoverable.
///
/// Public for testing — tests must import the real constant, not hardcode the value.
pub const MAX_NETWORK_RETRIES: u32 = 3;

/// Base delay for exponential backoff (milliseconds).
/// Retry delays: 1s → 2s → 4s
const NETWORK_RETRY_BASE_DELAY_MS: u64 = 1000;

/// Calculate the retry delay for the given attempt number.
///
/// Uses exponential backoff: base_delay * 2^(attempt-1)
/// Attempt 1: 1s, Attempt 2: 2s, Attempt 3: 4s
pub fn network_retry_delay(attempt: u32) -> Duration {
    Duration::from_millis(NETWORK_RETRY_BASE_DELAY_MS * 2u64.pow(attempt.saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delays() {
        assert_eq!(network_retry_delay(1), Duration::from_millis(1000));
        assert_eq!(network_retry_delay(2), Duration::from_millis(2000));
        assert_eq!(network_retry_delay(3), Duration::from_millis(4000));
    }

    #[test]
    fn test_retry_delay_zero_attempt() {
        // Saturating sub prevents underflow
        assert_eq!(network_retry_delay(0), Duration::from_millis(1000));
    }

    #[test]
    fn test_max_retries_constant() {
        assert_eq!(MAX_NETWORK_RETRIES, 3);
    }
}
