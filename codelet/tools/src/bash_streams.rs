//! Stream reader tasks and abort waiting for the bash tool.
//!
//! Spawns async tasks to read stdout/stderr from child processes,
//! optionally streaming output to UI callbacks.

use crate::bash_abort::is_bash_abort_requested;
use crate::bash_output::StreamBuffers;
use crate::error::ToolError;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use uuid::Uuid;

#[cfg(unix)]
use crate::bash_process::ProcessGroupKiller;

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

/// Spawn a task to read stdout, buffer it, and optionally stream to UI.
///
/// The `mode` parameter controls how lines are relayed:
/// - `ToolProgress`: calls `emit_tool_progress(session_id, line, false)`
/// - `Callback(cb)`: calls `cb(line)`
/// - `None`: only buffers
pub fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    buffer: Arc<Mutex<String>>,
    mode: StdoutStreamMode,
    session_id: Uuid,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if is_bash_abort_requested(session_id) {
                break;
            }
            let line_with_newline = format!("{line}\n");
            // Stream to UI based on mode
            match &mode {
                StdoutStreamMode::ToolProgress => {
                    crate::tool_progress::emit_tool_progress(
                        session_id,
                        &line_with_newline,
                        false,
                    );
                }
                StdoutStreamMode::Callback(cb) => {
                    cb(&line_with_newline);
                }
                StdoutStreamMode::None => {}
            }
            // Buffer for final result
            buffer.lock().await.push_str(&line_with_newline);
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
    let stdout_task = spawn_stdout_reader(
        stdout,
        buffers.stdout_handle(),
        stdout_mode,
        session_id,
    );
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

/// Wait for reader tasks with abort checking (non-Unix).
///
/// Returns Err if aborted, Ok(()) if completed normally.
#[cfg(not(unix))]
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
