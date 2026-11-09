//! PTY-mode process spawning (REFAC-045: moved from `spawning.rs`).

use super::*;

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
    let pair = pty_system.openpty(size).map_err(|e| ToolError::Execution {
        tool: "unified_exec",
        message: format!("Failed to open PTY: {e}"),
    })?;

    // BUG-172: the PTY child IS the platform terminal shell (a PTY needs a
    // real shell as its controlling child — `sh` on Unix, the BUG-156
    // PowerShell wrap on Windows). The command string (Shell as-is, Argv
    // space-joined) is passed via the platform shell invocation (R1).
    let command_string = match command {
        ExecCommand::Shell(s) => s.clone(),
        ExecCommand::Argv(args) => {
            // Re-exec through the requested argv inside the PTY.
            let mut joined = String::new();
            for a in &args[0..1] {
                joined.push_str(a);
            }
            for a in &args[1..] {
                joined.push(' ');
                joined.push_str(a);
            }
            joined
        }
    };
    let (pty_program, pty_args) =
        crate::bash_process::platform_shell_invocation(&command_string, OS);
    let mut builder = CommandBuilder::new(pty_program);
    for a in &pty_args {
        builder.arg(a);
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
    // returns a one-shot result instead of a session_id). The platform
    // blocking-sleep invocation (BUG-172 R3: `sleep` on Unix, PowerShell
    // `Start-Sleep` on Windows — there is no `sleep.exe`) blocks for
    // effectively the full i32 range; the background task kills it when
    // the PTY child exits or the session ends (kill_on_drop also covers
    // process exit).
    let (anchor_program, anchor_args) = crate::bash_process::pty_liveness_anchor_invocation(OS);
    let mut anchor_cmd = tokio::process::Command::new(&anchor_program);
    anchor_cmd
        .args(&anchor_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let anchor = anchor_cmd.spawn().map_err(|e| ToolError::Execution {
        tool: "unified_exec",
        message: format!("Failed to spawn liveness anchor: {e}"),
    })?;

    let anchor_pid = anchor.id();
    let pty_killer = pty_child.clone_killer();
    let pty_exit: crate::unified_exec::PtyExitSlot = Arc::new(std::sync::Mutex::new(None));
    let recorded = Arc::clone(&pty_exit);
    tokio::spawn(async move {
        // Block until the PTY child exits (blocking wait on a blocking
        // thread), record its REAL status (BUG-368), then kill the anchor
        // so the store's `try_wait` reports the exit.
        let exit = match tokio::task::spawn_blocking(move || pty_child.wait()).await {
            Ok(Ok(status)) => crate::unified_exec::ExecExit::from_pty(&status),
            Ok(Err(error)) => {
                tracing::error!(%error, "cannot wait for the PTY command");
                crate::unified_exec::ExecExit::unavailable()
            }
            Err(error) => {
                tracing::error!(%error, "the PTY wait task failed");
                crate::unified_exec::ExecExit::unavailable()
            }
        };
        match recorded.lock() {
            Ok(mut slot) => *slot = Some(exit),
            Err(_) => tracing::error!("PTY exit slot poisoned; the exit status is unavailable"),
        }
        kill_anchor_pid(anchor_pid);
    });

    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let output_notify = Arc::new(Notify::new());
    let last_output_micros = Arc::new(AtomicU64::new(now_micros()));

    // PTY output: OS-thread reader over the master (the portable-pty
    // master is blocking-only) -> bounded channel -> the shared buffer,
    // the same shape the pipe readers use.
    let master_read = master
        .try_clone_reader()
        .map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to clone PTY reader: {e}"),
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
    // BUG-362: every write is acknowledged with its real io result.
    let (stdin_tx, mut stdin_rx) = stdin_channel();
    let pty_writer =
        std::sync::Mutex::new(master.take_writer().map_err(|e| ToolError::Execution {
            tool: "unified_exec",
            message: format!("Failed to take PTY writer: {e}"),
        })?);
    let pty_writer = Arc::new(pty_writer);
    tokio::spawn(async move {
        while let Some(StdinWrite { bytes, ack }) = stdin_rx.recv().await {
            // The PTY writer is blocking-only: hand each write to a
            // blocking thread (the bounded 64-deep queue keeps bursts
            // rare; writes are serialized on the std mutex).
            let writer = Arc::clone(&pty_writer);
            let written = tokio::task::spawn_blocking(move || {
                use std::io::Write as _;
                let mut w = writer
                    .lock()
                    .map_err(|error| format!("PTY writer lock poisoned: {error}"))?;
                w.write_all(&bytes)
                    .and_then(|()| w.flush())
                    .map_err(|error| error.to_string())
            })
            .await;
            let result = match written {
                Ok(result) => result,
                Err(error) => Err(format!("PTY write task failed: {error}")),
            };
            acknowledge(ack, result);
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
        Some(pty_exit),
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
        // BUG-365: safe signalling; failures are logged, not dropped.
        crate::process_signal::kill_logged(crate::process_signal::kill_process(pid));
    }
    #[cfg(not(unix))]
    {
        match std::process::Command::new("taskkill")
            .args(crate::bash_process::taskkill_args(pid, true))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => tracing::error!(pid, %status, "taskkill of the liveness anchor failed"),
            Err(error) => {
                tracing::error!(pid, %error, "cannot run taskkill for the liveness anchor")
            }
        }
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
