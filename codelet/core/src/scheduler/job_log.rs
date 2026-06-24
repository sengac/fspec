//! Schedule Job Log — SCHED-012
//!
//! Append-only JSONL log for scheduler lifecycle events.
//! Writes to `spec/schedule-log.jsonl` in the project directory.
//! Gracefully handles write failures (warn via tracing, never crash).

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::warn;

/// Maximum entries before rotation triggers.
const ROTATION_THRESHOLD: usize = 2000;

/// Number of entries to keep after rotation.
const ROTATION_KEEP: usize = 1000;

/// A single log entry in the schedule job log.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLogEntry {
    pub timestamp: String,
    pub event: String,
    pub schedule: String,
    pub job_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Append a log entry to the JSONL file. Triggers rotation if the file
/// exceeds `ROTATION_THRESHOLD` entries.
///
/// Never panics or returns an error — write failures are logged as warnings.
pub async fn append_log_entry(log_path: &Path, entry: &JobLogEntry) {
    // Serialize the entry to a single JSON line
    let line = match serde_json::to_string(entry) {
        Ok(l) => l,
        Err(e) => {
            warn!("Failed to serialize job log entry: {}", e);
            return;
        }
    };

    // Check rotation before appending
    if let Err(e) = maybe_rotate(log_path).await {
        warn!("Job log rotation failed for {}: {}", log_path.display(), e);
        // Continue to try appending anyway
    }

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            warn!(
                "Failed to create directory for job log {}: {}",
                log_path.display(),
                e
            );
            return;
        }
    }

    // Append the line
    use tokio::io::AsyncWriteExt;
    match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .await
    {
        Ok(mut file) => {
            let mut buf = line.into_bytes();
            buf.push(b'\n');
            if let Err(e) = file.write_all(&buf).await {
                warn!(
                    "Failed to write job log entry to {}: {}",
                    log_path.display(),
                    e
                );
            }
            // Ensure data is flushed to disk before returning
            if let Err(e) = file.flush().await {
                warn!(
                    "Failed to flush job log entry to {}: {}",
                    log_path.display(),
                    e
                );
            }
        }
        Err(e) => {
            warn!("Failed to open job log file {}: {}", log_path.display(), e);
        }
    }
}

/// Rotate the log file if it exceeds `ROTATION_THRESHOLD` entries.
/// Keeps the most recent `ROTATION_KEEP` entries.
async fn maybe_rotate(log_path: &Path) -> Result<(), std::io::Error> {
    // Read existing content — missing file is fine (nothing to rotate)
    let content = match tokio::fs::read_to_string(log_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.len() <= ROTATION_THRESHOLD {
        return Ok(());
    }

    // Keep the most recent ROTATION_KEEP entries
    let start = lines.len().saturating_sub(ROTATION_KEEP);
    let kept: Vec<&str> = lines[start..].to_vec();
    let mut output = kept.join("\n");
    output.push('\n');

    tokio::fs::write(log_path, output).await
}
