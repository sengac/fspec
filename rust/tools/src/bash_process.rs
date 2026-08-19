//! Process group management and command spawning for the bash tool.
//!
//! Handles platform-specific process group setup (Unix) and
//! unified command spawning logic.
//!
//! # Windows shell wrapping (BUG-156)
//!
//! On Windows the user command is NEVER spawned directly. It is wrapped in a
//! real Windows shell (PowerShell preferred, `cmd /C` fallback) so the shell
//! performs PATHEXT resolution — bare names like `whoami` resolve to
//! `whoami.exe`. Process-tree termination uses `taskkill /PID <pid> /T`
//! (graceful) and `taskkill /PID <pid> /T /F` (forceful) via
//! [`WindowsProcessTreeKiller`], mirroring the Unix [`ProcessGroupKiller`]
//! guard pattern. Patterns adopted from VTCode (vtcode-bash-runner).

use super::error::ToolError;
use std::process::Stdio;
use tokio::process::Command;

#[cfg(windows)]
pub use crate::bash_process_windows::WindowsProcessTreeKiller;

/// Message returned to the caller when a command is aborted by the user.
pub const ABORT_MESSAGE: &str = "Command interrupted by user";

// ============================================================================
// Process Group Management (Unix)
// ============================================================================

/// Guard that kills the entire process group when dropped (Unix only).
///
/// This is necessary because `kill_on_drop(true)` only sends SIGKILL to the direct
/// child process, not to the process group. When the shell spawns child processes
/// (e.g., `npm run dev` spawning `node`), we need to kill the entire process group
/// to ensure all descendants are terminated.
///
/// On drop, sends SIGKILL to the negative PID (process group).
#[cfg(unix)]
pub struct ProcessGroupKiller {
    /// The process group ID (same as the shell's PID when using process_group(0))
    pgid: Option<u32>,
}

#[cfg(unix)]
impl ProcessGroupKiller {
    /// Create a new ProcessGroupKiller from a Child handle.
    pub fn new(child: &tokio::process::Child) -> Self {
        Self { pgid: child.id() }
    }

    /// Explicitly kill the process group
    #[allow(unsafe_code)]
    pub fn kill(&self) {
        if let Some(pgid) = self.pgid {
            // Send SIGKILL to the entire process group (negative PID)
            // This kills the shell AND all processes it spawned
            // Safety: kill() is safe to call with any PID/PGID
            // If the process group no longer exists, this returns an error which we ignore
            unsafe {
                libc::kill(-(pgid as i32), libc::SIGKILL);
            }
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessGroupKiller {
    fn drop(&mut self) {
        self.kill();
    }
}

// ============================================================================
// Shell Invocation Builders (BUG-156)
// ============================================================================
//
// Pure, platform-independent builders so the wrapping logic is unit-testable
// on any host. The cfg-gated spawn path consumes these.

/// Build the Windows PowerShell invocation for a user command.
///
/// Returns `(program, args)` where `program` is `powershell` and `args` are
/// `["-NoProfile", "-NonInteractive", "-Command", "<command>"]`. Running
/// inside a real Windows shell lets the shell perform PATHEXT resolution, so
/// bare command names (`whoami`, `cmd`, ...) resolve to their `.exe` forms.
/// The user command is passed via `-Command` — PowerShell does not accept
/// the command as a positional argument.
pub fn build_windows_shell_invocation(command: &str) -> (String, Vec<String>) {
    (
        "powershell".to_string(),
        vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-Command".to_string(),
            command.to_string(),
        ],
    )
}

/// Build the Windows `cmd /C` fallback invocation for a user command.
///
/// Used when PowerShell cannot be located. Returns `(program, args)` where
/// `program` is `cmd` and `args` are `["/C", "<command>"]`.
pub fn build_cmd_fallback_invocation(command: &str) -> (String, Vec<String>) {
    (
        "cmd".to_string(),
        vec!["/C".to_string(), command.to_string()],
    )
}

/// Build the Unix shell invocation for a user command.
///
/// Returns `(program, args)` where `program` is `sh` and `args` are
/// `["-c", "<command>"]`. This mirrors the historical Unix spawn path and is
/// pinned by tests to prevent Unix regressions.
pub fn build_unix_shell_invocation(command: &str) -> (String, Vec<String>) {
    (
        "sh".to_string(),
        vec!["-c".to_string(), command.to_string()],
    )
}

/// Build the `taskkill` argument vector for terminating a Windows process tree.
///
/// `forceful == false` → `["/PID", "<pid>", "/T"]` (graceful).
/// `forceful == true`  → `["/PID", "<pid>", "/T", "/F"]` (forceful).
pub fn taskkill_args(pid: u32, forceful: bool) -> Vec<String> {
    let mut args = vec!["/PID".to_string(), pid.to_string(), "/T".to_string()];
    if forceful {
        args.push("/F".to_string());
    }
    args
}

// ============================================================================
// Process Spawning
// ============================================================================

/// Spawn a shell command with proper configuration.
///
/// Handles platform-specific setup (process groups on Unix).
/// If `cwd` is provided, validates the directory exists and sets it as the working directory.
pub fn spawn_command(command: &str, cwd: Option<&str>) -> Result<tokio::process::Child, ToolError> {
    // Validate cwd exists if provided
    if let Some(dir) = cwd {
        if !std::path::Path::new(dir).is_dir() {
            return Err(ToolError::Validation {
                tool: "bash",
                message: format!("Directory not found: {dir}"),
            });
        }
    }

    #[cfg(unix)]
    {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0) // Create new process group for clean termination
            .kill_on_drop(true); // Fallback: kill direct child if guard fails

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        cmd.spawn().map_err(|e| ToolError::Execution {
            tool: "bash",
            message: format!("Failed to spawn command: {e}"),
        })
    }

    #[cfg(not(unix))]
    {
        // BUG-156: never spawn the raw command string on Windows. Wrap it in
        // a real Windows shell (PowerShell preferred, `cmd /C` fallback) so
        // the shell performs PATHEXT resolution.
        let (program, args) = build_windows_shell_invocation(command);
        let mut cmd = Command::new(&program);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true); // Fallback: kill direct child if guard fails

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        match cmd.spawn() {
            Ok(child) => Ok(child),
            Err(e) => {
                // PowerShell not found — fall back to cmd.exe.
                let (fallback_program, fallback_args) = build_cmd_fallback_invocation(command);
                let mut fallback = Command::new(&fallback_program);
                fallback
                    .args(&fallback_args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());
                if let Some(dir) = cwd {
                    fallback.current_dir(dir);
                }
                fallback.spawn().map_err(|fb_err| ToolError::Execution {
                    tool: "bash",
                    message: format!(
                        "Failed to spawn command via {program} ({e}) or {fallback_program} ({fb_err})"
                    ),
                })
            }
        }
    }
}

/// Take stdout and stderr handles from child process.
pub fn take_stdio_handles(
    child: &mut tokio::process::Child,
) -> Result<(tokio::process::ChildStdout, tokio::process::ChildStderr), ToolError> {
    let stdout = child.stdout.take().ok_or(ToolError::Execution {
        tool: "bash",
        message: "Failed to capture stdout".to_string(),
    })?;
    let stderr = child.stderr.take().ok_or(ToolError::Execution {
        tool: "bash",
        message: "Failed to capture stderr".to_string(),
    })?;
    Ok((stdout, stderr))
}
