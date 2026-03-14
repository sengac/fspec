//! Process spawning — pipe and PTY modes.
//!
//! Handles the creation of child processes with stdin forwarding,
//! stdout/stderr reading into a shared output buffer, and buffer capping.

use super::UNIFIED_EXEC_OUTPUT_MAX_BYTES;
use crate::error::ToolError;
use super::types::ExecCommand;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, Notify};

/// Return type for spawned processes: child handle, stdin sender, output buffer, notify.
pub type SpawnResult = (Child, mpsc::Sender<Vec<u8>>, Arc<Mutex<Vec<u8>>>, Arc<Notify>);

/// Cap the output buffer to `UNIFIED_EXEC_OUTPUT_MAX_BYTES`, discarding oldest bytes.
fn cap_output_buffer(buf: &mut Vec<u8>) {
    if buf.len() > UNIFIED_EXEC_OUTPUT_MAX_BYTES {
        let excess = buf.len() - UNIFIED_EXEC_OUTPUT_MAX_BYTES;
        buf.drain(..excess);
    }
}

/// Spawn an async reader task that reads from a stream into the shared buffer.
fn spawn_reader_task(
    stream: Option<impl tokio::io::AsyncRead + Unpin + Send + 'static>,
    buf_ref: Arc<Mutex<Vec<u8>>>,
    notify_ref: Arc<Notify>,
) {
    tokio::spawn(async move {
        if let Some(stream) = stream {
            let mut reader = tokio::io::BufReader::new(stream);
            let mut chunk = vec![0u8; 4096];
            loop {
                match tokio::io::AsyncReadExt::read(&mut reader, &mut chunk).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let mut buf = buf_ref.lock().await;
                        buf.extend_from_slice(&chunk[..n]);
                        cap_output_buffer(&mut buf);
                        drop(buf);
                        notify_ref.notify_waiters();
                    }
                    Err(_) => break,
                }
            }
        }
    });
}

/// Spawn a process using pipes (non-PTY).
pub fn spawn_pipe_process(
    command: &ExecCommand,
    cwd: Option<&str>,
) -> Result<SpawnResult, ToolError> {
    let mut cmd = match command {
        ExecCommand::Shell(s) => {
            let mut c = Command::new("sh");
            c.arg("-c").arg(s);
            c
        }
        ExecCommand::Argv(args) => {
            let mut c = Command::new(&args[0]);
            if args.len() > 1 {
                c.args(&args[1..]);
            }
            c
        }
    };

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(unix)]
    {
        cmd.process_group(0);
    }

    if let Some(dir) = cwd {
        if !std::path::Path::new(dir).is_dir() {
            return Err(ToolError::Validation {
                tool: "unified_exec",
                message: format!("Directory not found: {dir}"),
            });
        }
        cmd.current_dir(dir);
    }

    let mut child = cmd.spawn().map_err(|e| ToolError::Execution {
        tool: "unified_exec",
        message: format!("Failed to spawn: {e}"),
    })?;

    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let output_notify = Arc::new(Notify::new());

    // Set up stdin forwarding channel
    let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(64);
    let mut child_stdin = child.stdin.take();
    tokio::spawn(async move {
        while let Some(data) = stdin_rx.recv().await {
            if let Some(ref mut stdin) = child_stdin {
                let _ = stdin.write_all(&data).await;
                let _ = stdin.flush().await;
            }
        }
    });

    // Set up stdout and stderr readers (merged into same output buffer)
    spawn_reader_task(
        child.stdout.take(),
        Arc::clone(&output_buffer),
        Arc::clone(&output_notify),
    );
    spawn_reader_task(
        child.stderr.take(),
        Arc::clone(&output_buffer),
        Arc::clone(&output_notify),
    );

    Ok((child, stdin_tx, output_buffer, output_notify))
}

/// Spawn a process using a PTY.
///
/// **Current limitation:** True PTY allocation requires the `portable-pty` crate
/// (used by VTCode) or `nix::pty` (Unix-only). Until a PTY dependency is added,
/// this falls back to pipe-mode with stdin/stdout piped, which provides the same
/// session management (yield-and-resume, write, poll, close) but does not allocate
/// a pseudo-terminal. Programs that check `isatty()` will see `false`.
///
/// The `tty` flag is still stored on the ProcessEntry so that:
/// 1. `list` action reports which sessions intended PTY mode.
/// 2. Future PTY implementation can be swapped in here without changing callers.
/// 3. Codex facades can check `tty` to gate `write_stdin` behavior.
pub fn spawn_pty_process(
    command: &ExecCommand,
    cwd: Option<&str>,
) -> Result<SpawnResult, ToolError> {
    // Fallback to pipe-mode — see doc comment above for rationale.
    // When portable-pty is added, this will call the PTY spawn path instead.
    spawn_pipe_process(command, cwd)
}
