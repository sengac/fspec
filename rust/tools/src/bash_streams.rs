//! Stream reader tasks and abort waiting for the bash tool.
//!
//! Spawns async tasks to read stdout/stderr from child processes,
//! optionally streaming output to UI callbacks.
//!
//! stdout is read as raw bytes (line-delimited on `\n`) rather than via the
//! UTF-8 `lines()` iterator. This is required for BUG-142: binary payloads
//! such as PNG/JPEG/PDF must reach the captured buffer intact so the
//! binary-output guard can detect and suppress them before they hit the model.
//! Streaming callbacks still receive decoded (lossy-UTF-8) text chunks.

use crate::bash_abort::is_bash_abort_requested;
use crate::bash_output::StreamBuffers;
use crate::error::ToolError;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use uuid::Uuid;

#[cfg(unix)]
use crate::bash_process::ProcessGroupKiller;
#[cfg(windows)]
use crate::bash_process::WindowsProcessTreeKiller;

/// Callback for streaming output chunks to UI.
/// Receives each line of output as it's produced.
pub type StreamCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Determines how stdout lines are streamed to the UI.
pub enum StdoutStreamMode {
    /// Stream via `emit_tool_progress` (used by the rig::tool::Tool `call()` path).
    ToolProgress,
    /// Stream via an explicit callback (used by `call_with_streaming()`).
    Callback(StreamCallback),
    /// Don't stream to UI (buffer only).
    None,
}

/// Maximum raw stdout bytes captured for the binary-output guard / final
/// result. Matches the existing logical output limits — we don't need to
/// capture gigabytes of binary to decide it's binary.
const STDOUT_CAPTURE_LIMIT: usize = 1024 * 1024; // 1 MiB

/// Spawn a task to read stdout as raw bytes, buffer it, and optionally stream to UI.
///
/// Lines are delimited by `\n` (the byte) so arbitrary binary payloads are
/// preserved; for UI streaming, each line is decoded via `String::from_utf8_lossy`.
///
/// The `mode` parameter controls how lines are relayed:
/// - `ToolProgress`: calls `emit_tool_progress(session_id, line, false)`
/// - `Callback(cb)`: calls `cb(line)`
/// - `None`: only buffers
pub fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    buffer: Arc<Mutex<Vec<u8>>>,
    mode: StdoutStreamMode,
    session_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        let mut line_buf: Vec<u8> = Vec::new();
        loop {
            if is_bash_abort_requested(session_id) {
                break;
            }
            line_buf.clear();
            match reader.read_until(b'\n', &mut line_buf).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Stream to UI based on mode — use lossy decoding so UI sees
                    // something even if bytes aren't valid UTF-8.
                    let as_str = String::from_utf8_lossy(&line_buf);
                    match &mode {
                        StdoutStreamMode::ToolProgress => {
                            crate::tool_progress::emit_tool_progress(session_id, &as_str, false);
                        }
                        StdoutStreamMode::Callback(cb) => {
                            cb(&as_str);
                        }
                        StdoutStreamMode::None => {}
                    }
                    // Buffer raw bytes for final result (with a size cap so
                    // pathological producers can't OOM us).
                    let mut buf = buffer.lock().await;
                    if buf.len() < STDOUT_CAPTURE_LIMIT {
                        let remaining = STDOUT_CAPTURE_LIMIT - buf.len();
                        let take = line_buf.len().min(remaining);
                        buf.extend_from_slice(&line_buf[..take]);
                    }
                    // If read_until didn't end on a newline we've reached EOF
                    // mid-line; loop will exit on the next read.
                }
                Err(_) => break,
            }
        }
    })
}

/// Spawn a task to read stderr into buffer and optionally stream to UI with is_stderr flag.
pub fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    buffer: Arc<Mutex<String>>,
    stream_to_ui: bool,
    session_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if is_bash_abort_requested(session_id) {
                break;
            }
            let line_with_newline = format!("{line}\n");
            // Stream to UI with is_stderr=true if enabled
            if stream_to_ui {
                crate::tool_progress::emit_tool_progress(session_id, &line_with_newline, true);
            }
            let mut buf = buffer.lock().await;
            buf.push_str(&line_with_newline);
        }
    })
}

/// Convenience: spawn both reader tasks from a `StreamBuffers` and stdio handles.
///
/// Returns `(stdout_task, stderr_task)`.
pub fn spawn_readers(
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    buffers: &StreamBuffers,
    stdout_mode: StdoutStreamMode,
    stream_stderr_to_ui: bool,
    session_id: Uuid,
) -> (tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>) {
    let stdout_task = spawn_stdout_reader(stdout, buffers.stdout_handle(), stdout_mode, session_id);
    let stderr_task = spawn_stderr_reader(
        stderr,
        buffers.stderr_handle(),
        stream_stderr_to_ui,
        session_id,
    );
    (stdout_task, stderr_task)
}

/// Wait for reader tasks with abort checking (Unix).
///
/// Returns Err if aborted, Ok(()) if completed normally.
#[cfg(unix)]
pub async fn wait_for_tasks_with_abort(
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
    pg_killer: &ProcessGroupKiller,
    session_id: Uuid,
) -> Result<(), ToolError> {
    loop {
        if is_bash_abort_requested(session_id) {
            pg_killer.kill();
            stdout_task.abort();
            stderr_task.abort();
            return Err(ToolError::Execution {
                tool: "bash",
                message: "Command interrupted by user".to_string(),
            });
        }

        if stdout_task.is_finished() && stderr_task.is_finished() {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Wait for reader tasks with abort checking (Windows).
///
/// Returns Err if aborted, Ok(()) if completed normally.
/// On abort, the entire process tree (shell + children) is killed via
/// `taskkill /PID <pid> /T /F` through the [`WindowsProcessTreeKiller`] guard.
#[cfg(windows)]
pub async fn wait_for_tasks_with_abort(
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
    tree_killer: &WindowsProcessTreeKiller,
    session_id: Uuid,
) -> Result<(), ToolError> {
    loop {
        if is_bash_abort_requested(session_id) {
            tree_killer.kill();
            stdout_task.abort();
            stderr_task.abort();
            return Err(ToolError::Execution {
                tool: "bash",
                message: crate::bash_process::ABORT_MESSAGE.to_string(),
            });
        }

        if stdout_task.is_finished() && stderr_task.is_finished() {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

/// Wait for reader tasks with abort checking (non-Unix, non-Windows).
///
/// Returns Err if aborted, Ok(()) if completed normally.
#[cfg(not(any(unix, windows)))]
pub async fn wait_for_tasks_with_abort(
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
    session_id: Uuid,
) -> Result<(), ToolError> {
    loop {
        if is_bash_abort_requested(session_id) {
            stdout_task.abort();
            stderr_task.abort();
            return Err(ToolError::Execution {
                tool: "bash",
                message: "Command interrupted by user".to_string(),
            });
        }

        if stdout_task.is_finished() && stderr_task.is_finished() {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}
