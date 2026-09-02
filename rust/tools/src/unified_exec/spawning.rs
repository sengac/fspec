//! Process spawning — pipe and PTY modes.
//!
//! Handles the creation of child processes with stdin forwarding,
//! stdout/stderr reading into a shared output buffer, and buffer capping.

use super::process_store::{now_micros, ChildHandle};
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
/// abort).
pub type SpawnResult = (
    Child,
    mpsc::Sender<Vec<u8>>,
    Arc<Mutex<Vec<u8>>>,
    Arc<Notify>,
    Arc<AtomicU64>,
    ChildHandle,
);

/// TOOL-022 P4: platform-appropriate `TERM` for spawned processes.
const SPAWN_TERM: &str = "xterm-256color";

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
        #[allow(unsafe_code)]
        {
            let pid = child.id();
            ChildHandle {
                kill: Arc::new(move || {
                    if let Some(pid) = pid {
                        // SIGKILL to the whole process group (negative PID);
                        // the spawn used process_group(0) so pid == PGID.
                        // Mirrors bash_process.rs::ProcessGroupKiller.
                        unsafe {
                            libc::kill(-(pid as i32), libc::SIGKILL);
                        }
                    }
                }),
            }
        }
    }
    #[cfg(not(unix))]
    {
        let pid = child.id();
        ChildHandle {
            kill: Arc::new(move || {
                use crate::bash_process::taskkill_args;
                let status = std::process::Command::new("taskkill")
                    .args(taskkill_args(pid, true))
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                if let Err(e) = status {
                    tracing::warn!("failed to run taskkill for pid {pid}: {e}");
                }
            }),
        }
    }
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
        .kill_on_drop(true)
        .env("TERM", SPAWN_TERM);
    apply_non_pager_env(&mut cmd);

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
    // TOOL-022: last-output timestamp shared with the reader tasks.
    let last_output_micros = Arc::new(AtomicU64::new(now_micros()));
    // TOOL-022 P4: platform kill handle (G1) — full process-tree
    // termination on ESC abort from the BashTool delegation loop.
    let kill_handle = build_kill_handle(&child);

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

    // Set up stdout and stderr readers (merged into the same output
    // buffer; the stderr stream is line-tagged with the marker so the
    // Bash layer can split it back out — TOOL-022 P4).
    spawn_reader_task(
        child.stdout.take(),
        Arc::clone(&output_buffer),
        Arc::clone(&output_notify),
        Arc::clone(&last_output_micros),
        false,
    );
    spawn_reader_task(
        child.stderr.take(),
        Arc::clone(&output_buffer),
        Arc::clone(&output_notify),
        Arc::clone(&last_output_micros),
        true,
    );

    Ok((
        child,
        stdin_tx,
        output_buffer,
        output_notify,
        last_output_micros,
        kill_handle,
    ))
}

/// Spawn a process using a real PTY (TOOL-022 P3, G8/FIX-1).
///
/// Allocated via `portable-pty` (already a workspace dependency, used by
/// `bridge_pty.rs`): the shell is spawned with the PTY slave as its
/// controlling terminal so `isatty()` is true, and pagers/`git`/REPLs
/// behave like in a terminal.
///
/// Session machinery is identical to pipe mode — the same
/// `ProcessEntry` shape. Liveness: a tokio `Child` anchor (the PTY child
/// is a blocking `Box<dyn Child>`, not a tokio `Child` — the reaper and
/// `try_wait`/`quiet_secs` observe the anchor, and a watcher task reaps
/// the PTY child in the background; the anchor is killed when the PTY
/// child exits or when the session ends). PTY env:
/// `TERM=xterm-256color` + the G2 pager-suppression triple.
pub fn spawn_pty_process(
    command: &ExecCommand,
    cwd: Option<&str>,
) -> Result<SpawnResult, ToolError> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};

    let pty_system = native_pty_system();
    let size = PtySize {
        rows: 40,
        cols: 200,
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = pty_system
        .openpty(size)
        .map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to open PTY: {e}"),
        })?;

    let mut builder = CommandBuilder::new("sh");
    match command {
        ExecCommand::Shell(s) => {
            builder.arg("-c");
            builder.arg(s);
        }
        ExecCommand::Argv(args) => {
            // Re-exec through the requested argv inside the PTY.
            builder.arg("-c");
            let mut joined = String::new();
            for a in &args[0..1] {
                joined.push_str(a);
            }
            for a in &args[1..] {
                joined.push(' ');
                joined.push_str(a);
            }
            builder.arg(&joined);
        }
    }
    if let Some(dir) = cwd {
        if !std::path::Path::new(dir).is_dir() {
            return Err(ToolError::Validation {
                tool: "unified_exec",
                message: format!("Directory not found: {dir}"),
            });
        }
        builder.cwd(dir);
    }
    builder.env("TERM", SPAWN_TERM);
    builder.env("COLORTERM", "truecolor");
    builder.env("PAGER", "cat");
    builder.env("GIT_PAGER", "cat");
    builder.env("NO_COLOR", "1");

    let (pty_child, master) = (pair.slave, pair.master);
    let mut pty_child = pty_child
        .spawn_command(builder)
        .map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to spawn PTY process: {e}"),
        })?;

    // Liveness anchor: a real tokio `Child` whose wait state the reaper /
    // `try_wait` / `quiet_secs` observe. The anchor must BLOCK until killed
    // — `true` exits instantly, which would make `try_wait` report EXITED
    // for every PTY session right after spawn (TOOL-022 P4 regression
    // surface: the reaper then removes the session and the `run` action
    // returns a one-shot result instead of a session_id). `sleep` with the
    // max representable seconds blocks for effectively forever; the
    // background task kills it when the PTY child exits or the session
    // ends (kill_on_drop also covers process exit).
    let mut anchor_cmd = tokio::process::Command::new("sleep");
    anchor_cmd
        .arg("2147483647")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let anchor = anchor_cmd.spawn().map_err(|e| {
        ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to spawn liveness anchor: {e}"),
        }
    })?;

    let anchor_pid = anchor.id();
    let pty_killer = pty_child.clone_killer();
    tokio::spawn(async move {
        // Block until the PTY child exits (blocking wait on a blocking
        // thread), then kill the anchor so the store's `try_wait`
        // reports the exit.
        let _ = tokio::task::spawn_blocking(move || pty_child.wait())
        .await;
        kill_anchor_pid(anchor_pid);
    });

    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let output_notify = Arc::new(Notify::new());
    let last_output_micros = Arc::new(AtomicU64::new(now_micros()));

    // PTY output: OS-thread reader over the master (the portable-pty
    // master is blocking-only) -> bounded channel -> the shared buffer,
    // the same shape the pipe readers use.
    let master_read = master.try_clone_reader().map_err(|e| {
        ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to clone PTY reader: {e}"),
        }
    })?;
    let (pty_tx, pty_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    std::thread::Builder::new()
        .name("unified-exec-pty-read".into())
        .spawn(move || {
            use std::io::Read;
            let mut reader = master_read;
            let mut chunk = vec![0u8; 8192];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        if pty_tx.send(chunk[..n].to_vec()).is_err() {
                            break; // receiver dropped - session gone
                        }
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to spawn PTY reader thread: {e}"),
        })?;
    let pty_rx_ref = pty_rx;
    let notify_ref = Arc::clone(&output_notify);
    let last_output_ref = Arc::clone(&last_output_micros);
    let output_ref = Arc::clone(&output_buffer);
    // One blocking pump thread owns the std receiver; it forwards each
    // chunk over a tokio mpsc (capacity 256 — the PTY reader only
    // pushes at terminal speed, so the bounded channel never
    // backpressures the pump thread for any realistic session).
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);
    std::thread::Builder::new()
        .name("unified-exec-pty-pump".into())
        .spawn(move || {
            loop {
                match pty_rx_ref.recv_timeout(std::time::Duration::from_millis(500)) {
                    Ok(bytes) => {
                        if tx.try_send(bytes).is_err() {
                            break; // receiver dropped - session gone
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to spawn PTY pump thread: {e}"),
        })?;
    tokio::spawn(async move {
        while let Some(bytes) = rx.recv().await {
            let mut buf = output_ref.lock().await;
            buf.extend_from_slice(&bytes);
            cap_output_buffer(&mut buf);
            drop(buf);
            last_output_ref.store(now_micros(), Ordering::Relaxed);
            notify_ref.notify_waiters();
        }
    });

    // PTY stdin: same mpsc shape as pipe mode.
    let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(64);
    let pty_writer = std::sync::Mutex::new(
        master
            .take_writer()
            .map_err(|e| ToolError::Execution {
                tool: "unified_exec",
                message: format!("Failed to take PTY writer: {e}"),
            })?,
    );
    let pty_writer = Arc::new(pty_writer);
    tokio::spawn(async move {
        while let Some(data) = stdin_rx.recv().await {
            // The PTY writer is blocking-only: hand each write to a
            // blocking thread (the bounded 64-deep queue keeps bursts
            // rare; writes are serialized on the std mutex).
            let writer = Arc::clone(&pty_writer);
            let _ = tokio::task::spawn_blocking(move || {
                use std::io::Write as _;
                if let Ok(mut w) = writer.lock() {
                    let _ = w.write_all(&data);
                    let _ = w.flush();
                }
            })
            .await;
        }
    });

    let kill_handle = build_pty_kill_handle(pty_killer, anchor_pid);

    Ok((
        anchor,
        stdin_tx,
        output_buffer,
        output_notify,
        last_output_micros,
        kill_handle,
    ))
}

/// Kill the anchor pid (used by the PTY waiter after the PTY child
/// exits, and by the session kill path). Reaping is handled by the
/// reaper via the store entry; the kill only guarantees the anchor
/// process is dead so `try_wait` flips to `Some(status)`.
fn kill_anchor_pid(pid: Option<u32>) {
    let Some(pid) = pid else {
        return;
    };
    #[cfg(unix)]
    {
        #[allow(unsafe_code)]
        {
            let _ = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        }
    }
    #[cfg(not(unix))]
    {
        let _ = std::process::Command::new("taskkill")
            .args(crate::bash_process::taskkill_args(pid, true))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

/// TOOL-022 P4/P3: kill handle for a PTY session — kills the PTY child
/// (portable-pty) AND the anchor (so the store's liveness check flips).
fn build_pty_kill_handle(
    pty_killer: Box<dyn portable_pty::ChildKiller + Send + Sync>,
    anchor_pid: Option<u32>,
) -> ChildHandle {
    let killer = Arc::new(std::sync::Mutex::new(pty_killer));
    ChildHandle {
        kill: Arc::new(move || {
            if let Ok(mut child) = killer.lock() {
                let _ = child.kill();
            }
            kill_anchor_pid(anchor_pid);
        }),
    }
}
