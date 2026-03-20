//! Agent Lifecycle Hooks — Shell Command Executor
//!
//! Low-level async command execution with stdin piping, stdout/stderr capture,
//! timeout enforcement, and process kill on timeout.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::warn;

use super::engine::HookContext;

/// Internal result from executing a single shell command.
pub(crate) struct CommandResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

/// Execute a shell command with JSON payload on stdin, capturing stdout/stderr.
pub(crate) async fn execute_command(
    command: &str,
    payload_json: &str,
    timeout_secs: u64,
    ctx: &HookContext,
    global_shell: Option<&str>,
) -> CommandResult {
    let (shell, shell_arg) = parse_shell(global_shell);

    let mut child = match Command::new(&shell)
        .arg(&shell_arg)
        .arg(command)
        .current_dir(&ctx.cwd)
        .env("FSPEC_PROJECT_DIR", &ctx.cwd)
        .env("FSPEC_SESSION_ID", &ctx.session_id)
        .env("FSPEC_HOOK_EVENT", event_name_from_payload(payload_json))
        .env("FSPEC_TRANSCRIPT_PATH", &ctx.transcript_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            warn!("Failed to spawn hook command: {e}");
            return CommandResult {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Failed to spawn: {e}"),
                timed_out: false,
            };
        }
    };

    // Write payload to stdin, then close the pipe
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload_json.as_bytes()).await;
        let _ = stdin.flush().await;
        drop(stdin);
    }

    // Take stdout/stderr handles before waiting
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

    // Wait with timeout
    let timeout_duration = Duration::from_secs(timeout_secs);
    match tokio::time::timeout(timeout_duration, child.wait()).await {
        Ok(Ok(status)) => {
            let stdout = read_stdout(stdout_handle).await;
            let stderr = read_stderr(stderr_handle).await;
            CommandResult {
                exit_code: status.code(),
                stdout,
                stderr,
                timed_out: false,
            }
        }
        Ok(Err(e)) => {
            warn!("Hook command failed: {e}");
            CommandResult {
                exit_code: None,
                stdout: String::new(),
                stderr: format!("Command execution failed: {e}"),
                timed_out: false,
            }
        }
        Err(_) => {
            // Timeout — kill the child process
            let _ = child.kill().await;
            let _ = child.wait().await;
            CommandResult {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                timed_out: true,
            }
        }
    }
}

/// Parse shell config string (e.g., "bash -c") into (program, flag) tuple.
fn parse_shell(global_shell: Option<&str>) -> (String, String) {
    match global_shell {
        Some(s) => {
            let parts: Vec<&str> = s.splitn(2, ' ').collect();
            if parts.len() == 2 {
                (parts[0].to_string(), parts[1].to_string())
            } else {
                (s.to_string(), "-c".to_string())
            }
        }
        None => ("sh".to_string(), "-c".to_string()),
    }
}

/// Extract the hook_event_name from the JSON payload for FSPEC_HOOK_EVENT env var.
fn event_name_from_payload(payload_json: &str) -> &str {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload_json) {
        if let Some(name) = v.get("hook_event_name").and_then(|n| n.as_str()) {
            return match name {
                "SessionStart" => "SessionStart",
                "SessionEnd" => "SessionEnd",
                "UserPromptSubmit" => "UserPromptSubmit",
                "PreToolUse" => "PreToolUse",
                "PostToolUse" => "PostToolUse",
                "Notification" => "Notification",
                _ => "Unknown",
            };
        }
    }
    "Unknown"
}

/// Read all bytes from an optional stdout handle into a String.
async fn read_stdout(handle: Option<tokio::process::ChildStdout>) -> String {
    match handle {
        Some(mut reader) => {
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        }
        None => String::new(),
    }
}

/// Read all bytes from an optional stderr handle into a String.
async fn read_stderr(handle: Option<tokio::process::ChildStderr>) -> String {
    match handle {
        Some(mut reader) => {
            let mut buf = Vec::new();
            let _ = reader.read_to_end(&mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        }
        None => String::new(),
    }
}
