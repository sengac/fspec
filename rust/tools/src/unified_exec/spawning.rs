//! Process spawning — pipe and PTY modes.
//!
//! Handles the creation of child processes with stdin forwarding,
//! stdout/stderr reading into a shared output buffer, and buffer capping.

use super::process_store::{now_micros, ChildHandle};
use super::stdin_write::{acknowledge, stdin_channel, StdinSender, StdinWrite};
use super::types::ExecCommand;
use super::UNIFIED_EXEC_OUTPUT_MAX_BYTES;
use crate::error::ToolError;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, Mutex, Notify};

/// Return type for spawned processes: child handle, stdin sender, output buffer,
/// notify, the TOOL-022 last-output timestamp (monotonic micros), and the
/// TOOL-022 P4 platform kill handle (G1: full process-tree termination on
/// abort), and (BUG-368) a PTY session's exit slot.
pub type SpawnResult = (
    Child,
    StdinSender,
    Arc<Mutex<Vec<u8>>>,
    Arc<Notify>,
    Arc<AtomicU64>,
    ChildHandle,
    Option<super::exec_exit::PtyExitSlot>,
);

/// TOOL-022 P4: platform-appropriate `TERM` for spawned processes.
const SPAWN_TERM: &str = "xterm-256color";

/// BUG-172: target OS name for platform shell selection
/// (`std::env::consts::OS` — "unix" on Linux/macOS, "windows" on Windows).
const OS: &str = std::env::consts::OS;

/// TOOL-022 P4 (G2): pager suppression env applied to every spawned
/// child so `git`/`man`/`less` never block on a pager.
fn apply_non_pager_env(cmd: &mut Command) {
    cmd.env("PAGER", "cat")
        .env("GIT_PAGER", "cat")
        .env("NO_COLOR", "1");
}

/// Cap the output buffer to `UNIFIED_EXEC_OUTPUT_MAX_BYTES`, discarding oldest bytes.
fn cap_output_buffer(buf: &mut Vec<u8>) {
    if buf.len() > UNIFIED_EXEC_OUTPUT_MAX_BYTES {
        let excess = buf.len() - UNIFIED_EXEC_OUTPUT_MAX_BYTES;
        buf.drain(..excess);
    }
}

/// Spawn an async reader task that reads from a stream into the shared buffer.
///
/// TOOL-022: each successful read refreshes `last_output_micros` — the
/// single source of truth for the `quiet_seconds` timing fact.
///
/// TOOL-022 P4: `mark_stderr_lines` — when true (the STDERR stream),
/// every line is prefixed with the `⚠stderr⚠` marker so the Bash
/// layer can split the merged buffer back into (stdout, stderr)
/// (`bash_output::split_merged_output`). The STDOUT stream is buffered
/// raw (binary payloads survive for the BUG-142 guard).
fn spawn_reader_task(
    stream: Option<impl tokio::io::AsyncRead + Unpin + Send + 'static>,
    buf_ref: Arc<Mutex<Vec<u8>>>,
    notify_ref: Arc<Notify>,
    last_output_micros: Arc<std::sync::atomic::AtomicU64>,
    mark_stderr_lines: bool,
) {
    tokio::spawn(async move {
        if let Some(stream) = stream {
            if mark_stderr_lines {
                // Line-oriented: prefix each stderr line with the
                // marker. A final partial line (no trailing newline)
                // is flushed on EOF.
                let mut reader = tokio::io::BufReader::new(stream);
                let mut chunk = vec![0u8; 4096];
                let mut line: Vec<u8> = Vec::with_capacity(256);
                loop {
                    match tokio::io::AsyncReadExt::read(&mut reader, &mut chunk).await {
                        Ok(0) => {
                            // EOF — flush the trailing partial line.
                            if !line.is_empty() {
                                let mut out = Vec::with_capacity(line.len() + 16);
                                out.extend_from_slice(crate::bash_output::STDERR_MARKER.as_bytes());
                                out.extend_from_slice(&line);
                                let mut buf = buf_ref.lock().await;
                                buf.extend_from_slice(&out);
                                cap_output_buffer(&mut buf);
                                drop(buf);
                                last_output_micros.store(now_micros(), Ordering::Relaxed);
                                notify_ref.notify_waiters();
                            }
                            break;
                        }
                        Ok(n) => {
                            line.extend_from_slice(&chunk[..n]);
                            // Flush complete lines (up to the last \n).
                            let mut start = 0;
                            while let Some(pos) = line[start..].iter().position(|b| *b == b'\n') {
                                let end = start + pos + 1;
                                let mut out = Vec::with_capacity(end + 16);
                                out.extend_from_slice(crate::bash_output::STDERR_MARKER.as_bytes());
                                out.extend_from_slice(&line[start..end]);
                                let mut buf = buf_ref.lock().await;
                                buf.extend_from_slice(&out);
                                cap_output_buffer(&mut buf);
                                drop(buf);
                                start = end;
                            }
                            if start > 0 {
                                line.drain(..start);
                            }
                            last_output_micros.store(now_micros(), Ordering::Relaxed);
                            notify_ref.notify_waiters();
                        }
                        Err(_) => break,
                    }
                }
            } else {
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
                            last_output_micros.store(now_micros(), Ordering::Relaxed);
                            notify_ref.notify_waiters();
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });
}

/// TOOL-022 P4 (G1): build the platform kill handle that terminates
/// the whole process tree — SIGKILL to the process group on Unix
/// (the pipe spawn uses `process_group(0)`, so the shell's PID IS the
/// PGID), `taskkill /PID <pid> /T /F` on Windows (same helper as
/// `bash_process_windows.rs`).
fn build_kill_handle(child: &Child) -> ChildHandle {
    #[cfg(unix)]
    {
        let pid = child.id();
        ChildHandle {
            kill: Arc::new(move || {
                if let Some(pid) = pid {
                    // SIGKILL to the whole process group; the spawn used
                    // process_group(0) so pid == PGID (BUG-365: safe API,
                    // failures logged).
                    crate::process_signal::kill_logged(crate::process_signal::kill_process_group(
                        pid,
                    ));
                }
            }),
        }
    }
    #[cfg(not(unix))]
    {
        let pid = child.id();
        ChildHandle {
            kill: Arc::new(move || {
                use crate::bash_process::taskkill_args;
                // `child.id()` is `Option<u32>`; guard like the unix branch so we
                // never build `taskkill` args from a missing pid.
                if let Some(pid) = pid {
                    let status = std::process::Command::new("taskkill")
                        .args(taskkill_args(pid, true))
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();
                    if let Err(e) = status {
                        tracing::warn!("failed to run taskkill for pid {pid}: {e}");
                    }
                }
            }),
        }
    }
}

mod pipe;
mod pty;

pub use pipe::spawn_pipe_process;
pub use pty::spawn_pty_process;
