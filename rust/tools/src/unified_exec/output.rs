//! Output collection and truncation utilities.

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// TOOL-022 P4: drain cadence for the interrupt check inside
/// [`collect_output_until_deadline_interruptible`] — matches the pre-P4
/// Bash wait loop's ~50ms abort-flag polling.
pub const INTERRUPT_CHECK_EVERY_MS: u64 = 50;

/// Like [`collect_output_until_deadline`], but after EVERY drain (and
/// every `INTERRUPT_CHECK_EVERY_MS` tick) it consults `interrupt`:
/// when the closure returns `true` the collection stops early and the
/// output captured so far is returned.
///
/// TOOL-022 P4: the BashTool delegation loop uses this so an ESC abort
/// (per-session flag) is observed within ~50ms even mid-window — the
/// pre-P4 wait loop polled the flag at ~50ms cadence, and the bounded
/// poll windows alone would otherwise delay the kill by a whole window.
pub async fn collect_output_until_deadline_interruptible(
    output_buffer: &Arc<Mutex<Vec<u8>>>,
    output_notify: &Arc<Notify>,
    yield_time_ms: u64,
    interrupt: &Arc<dyn Fn() -> bool + Send + Sync>,
) -> Vec<u8> {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(yield_time_ms);
    let mut collected = Vec::new();
    let tick = tokio::time::Duration::from_millis(INTERRUPT_CHECK_EVERY_MS);

    loop {
        // Check if deadline passed — the drain (below) is the final one
        // when it does.
        let now = tokio::time::Instant::now();
        if now >= deadline {
            drain_into(output_buffer, &mut collected).await;
            break;
        }

        // Wait for new output, the interrupt ticker, or the deadline;
        // whichever wakes first, drain what the reader tasks have
        // buffered (an early notification must NOT skip its drain —
        // that is how output would be lost).
        let remaining = deadline - now;
        let wait = tick.min(remaining);
        tokio::select! {
            _ = output_notify.notified() => {}
            _ = tokio::time::sleep(wait) => {}
        }
        drain_into(output_buffer, &mut collected).await;

        // Early-out: abort flag flipped, or the deadline passed while
        // draining.
        if interrupt() || tokio::time::Instant::now() >= deadline {
            break;
        }
    }

    collected
}

/// Drain the shared output buffer into `collected` (clearing it).
async fn drain_into(output_buffer: &Arc<Mutex<Vec<u8>>>, collected: &mut Vec<u8>) {
    let mut buf = output_buffer.lock().await;
    collected.extend_from_slice(&buf);
    buf.clear();
}

/// Truncate output string to reasonable size for LLM consumption.
pub fn truncate_output_str(output: &str) -> String {
    const MAX_OUTPUT_CHARS: usize = 30_000;
    if output.len() <= MAX_OUTPUT_CHARS {
        output.to_string()
    } else {
        let truncated = &output[..MAX_OUTPUT_CHARS];
        format!("{truncated}\n\n... [output truncated at {MAX_OUTPUT_CHARS} characters]")
    }
}
