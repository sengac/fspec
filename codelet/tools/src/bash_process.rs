//! Process group management and command spawning for the bash tool.
//!
//! Handles platform-specific process group setup (Unix) and
//! unified command spawning logic.

use super::error::ToolError;
use std::process::Stdio;
use tokio::process::Command;

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
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true); // Kill process when Child is dropped

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        cmd.spawn().map_err(|e| ToolError::Execution {
            tool: "bash",
            message: format!("Failed to spawn command: {e}"),
        })
    }
}

/// Take stdout and stderr handles from child process.
pub fn take_stdio_handles(
    child: &mut tokio::process::Child,
) -> Result<
    (
        tokio::process::ChildStdout,
        tokio::process::ChildStderr,
    ),
    ToolError,
> {
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
