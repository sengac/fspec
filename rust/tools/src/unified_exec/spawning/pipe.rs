//! Pipe-mode process spawning (REFAC-045: moved from `spawning.rs`).

use super::*;

/// Spawn a process using pipes (non-PTY).
pub fn spawn_pipe_process(
    command: &ExecCommand,
    cwd: Option<&str>,
) -> Result<SpawnResult, ToolError> {
    let mut cmd = match command {
        ExecCommand::Shell(s) => {
            // BUG-172: platform shell wrap — `sh -c` on Unix (unchanged),
            // the BUG-156 PowerShell wrap on Windows (never spawn `sh`).
            let (program, args) = crate::bash_process::platform_shell_invocation(s, OS);
            let mut c = Command::new(&program);
            c.args(&args);
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

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            // BUG-172 (R1): on Windows the PowerShell spawn fails when
            // PowerShell cannot be located — fall back to `cmd /C <cmd>`
            // (the same fallback the BUG-156 spawn path used before
            // TOOL-022 P4 moved this path). Unix and Argv spawns fail
            // as before (no fallback).
            let (fallback_program, fallback_args) = match (command, OS) {
                (ExecCommand::Shell(s), "windows") => {
                    crate::bash_process::windows_shell_fallback_invocation(s)
                }
                _ => {
                    return Err(ToolError::Execution {
                        tool: "unified_exec",
                        message: format!("Failed to spawn: {e}"),
                    })
                }
            };
            let mut fallback = Command::new(&fallback_program);
            fallback
                .args(&fallback_args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .env("TERM", SPAWN_TERM);
            apply_non_pager_env(&mut fallback);
            #[cfg(unix)]
            {
                fallback.process_group(0);
            }
            if let Some(dir) = cwd {
                fallback.current_dir(dir);
            }
            fallback.spawn().map_err(|fb_err| ToolError::Execution {
                tool: "unified_exec",
                message: format!(
                    "Failed to spawn via powershell ({e}) or {fallback_program} ({fb_err})"
                ),
            })?
        }
    };

    let output_buffer = Arc::new(Mutex::new(Vec::new()));
    let output_notify = Arc::new(Notify::new());
    // TOOL-022: last-output timestamp shared with the reader tasks.
    let last_output_micros = Arc::new(AtomicU64::new(now_micros()));
    // TOOL-022 P4: platform kill handle (G1) — full process-tree
    // termination on ESC abort from the BashTool delegation loop.
    let kill_handle = build_kill_handle(&child);

    // Set up stdin forwarding channel
    // BUG-362: every write is acknowledged with its real io result.
    let (stdin_tx, mut stdin_rx) = stdin_channel();
    let mut child_stdin = child.stdin.take();
    tokio::spawn(async move {
        while let Some(StdinWrite { bytes, ack }) = stdin_rx.recv().await {
            let result = match child_stdin.as_mut() {
                Some(stdin) => match stdin.write_all(&bytes).await {
                    Ok(()) => stdin.flush().await.map_err(|error| error.to_string()),
                    Err(error) => Err(error.to_string()),
                },
                None => Err("the command has no stdin pipe".to_string()),
            };
            acknowledge(ack, result);
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
        None,
    ))
}
