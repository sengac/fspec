//! Schedule Job Log — SCHED-012
//!
//! Append-only JSONL log for scheduler lifecycle events.
//! Writes to `spec/schedule-log.jsonl` in the project directory.
//! Gracefully handles write failures (warn via tracing, never crash).
//! BUG-498: appends and rotations hold the spec lock on `spec/`.

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
/// BUG-498: the rotation check, the rotation itself (an atomic replace) and
/// the append all run while holding the spec lock on the log's directory
/// (`spec/`), the lock every fspec writer takes, so concurrent appends never
/// interleave and a rotation never drops an entry another writer appended.
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
    let path = log_path.to_path_buf();
    let appended = tokio::task::spawn_blocking(move || append_locked(&path, &line)).await;
    match appended {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!(
            "Failed to write job log entry to {}: {}",
            log_path.display(),
            e
        ),
        Err(e) => warn!(
            "Job log writer for {} did not finish: {}",
            log_path.display(),
            e
        ),
    }
}

/// Rotate if needed, then append `line`, holding the spec lock throughout.
fn append_locked(log_path: &Path, line: &str) -> std::io::Result<()> {
    let dir = log_path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let _lock = codelet_common::spec_lock::SpecLock::acquire(dir)?;
    if let Err(e) = maybe_rotate(log_path) {
        warn!("Job log rotation failed for {}: {}", log_path.display(), e);
        // Continue to try appending anyway
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let mut buf = line.as_bytes().to_vec();
    buf.push(b'\n');
    file.write_all(&buf)?;
    file.flush()
}

/// Rotate the log file if it exceeds `ROTATION_THRESHOLD` entries.
/// Keeps the most recent `ROTATION_KEEP` entries (atomic replace; the caller
/// holds the spec lock).
fn maybe_rotate(log_path: &Path) -> Result<(), std::io::Error> {
    // Read existing content — missing file is fine (nothing to rotate)
    let content = match std::fs::read_to_string(log_path) {
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

    codelet_common::spec_lock::replace_file_atomically(log_path, output.as_bytes())
}
