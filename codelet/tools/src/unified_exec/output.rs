//! Output collection and truncation utilities.

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

/// Collect output from the buffer until deadline, draining what we read.
///
/// Waits for new output notifications and drains the shared buffer into a local
/// collection until `yield_time_ms` has elapsed. Returns the collected output
/// as a UTF-8 string (lossy conversion for binary output).
pub async fn collect_output_until_deadline(
    output_buffer: &Arc<Mutex<Vec<u8>>>,
    output_notify: &Arc<Notify>,
    yield_time_ms: u64,
) -> String {
    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(yield_time_ms);
    let mut collected = Vec::new();

    loop {
        // Drain current buffer
        {
            let mut buf = output_buffer.lock().await;
            if !buf.is_empty() {
                collected.extend_from_slice(&buf);
                buf.clear();
            }
        }

        // Check if deadline passed
        let now = tokio::time::Instant::now();
        if now >= deadline {
            break;
        }

        // Wait for more output or timeout
        let remaining = deadline - now;
        tokio::select! {
            _ = output_notify.notified() => {
                // New output available, loop around to drain
            }
            _ = tokio::time::sleep(remaining) => {
                // Deadline reached — do one final drain
                let mut buf = output_buffer.lock().await;
                if !buf.is_empty() {
                    collected.extend_from_slice(&buf);
                    buf.clear();
                }
                break;
            }
        }
    }

    String::from_utf8_lossy(&collected).to_string()
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
