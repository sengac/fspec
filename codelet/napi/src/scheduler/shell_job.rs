//! Shell job execution — SCHED-005
//!
//! When the scheduler determines a shell-type schedule should fire,
//! execute the configured command via `sh -c` in the project directory.
//! Captures stdout and stderr separately, returns exit code.

use super::types::ScheduleEntry;
use anyhow::{anyhow, Result};
use tracing::info;
use tokio::process::Command;

/// Result of a shell job execution.
#[derive(Debug, Clone)]
pub struct ShellJobResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Execute a shell job from a schedule entry.
///
/// Validates the shell config, then spawns `sh -c "<command>"` in the
/// project directory. Returns a `ShellJobResult` on successful execution
/// (even if exit code is non-zero), or an error for config issues.
pub async fn trigger_shell_job(
    name: &str,
    project_path: &str,
    entry: &ScheduleEntry,
) -> Result<ShellJobResult> {
    // Validate shell config exists
    let shell_config = entry
        .shell
        .as_ref()
        .ok_or_else(|| anyhow!("Schedule '{}': missing shell configuration", name))?;

    // Validate command is not empty
    let command = &shell_config.command;
    if command.trim().is_empty() {
        return Err(anyhow!(
            "Schedule '{}': shell command is empty",
            name
        ));
    }

    info!(
        "Shell job '{}': executing '{}' in {}",
        name, command, project_path
    );

    // Execute via sh -c
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(project_path)
        .output()
        .await
        .map_err(|e| anyhow!("Schedule '{}': failed to spawn shell: {}", name, e))?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    info!(
        "Shell job '{}': exit_code={}, stdout={} bytes, stderr={} bytes",
        name,
        exit_code,
        stdout.len(),
        stderr.len()
    );

    Ok(ShellJobResult {
        exit_code,
        stdout,
        stderr,
    })
}
