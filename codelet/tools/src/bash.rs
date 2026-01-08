//! Bash tool implementation
//!
//! Executes shell commands with output truncation.
//! Supports streaming output to UI while buffering complete output for LLM.
//!
//! # Process Management
//!
//! Commands are spawned in their own process group using `process_group(0)` on Unix.
//! This allows killing the entire process tree (shell + children) when interrupted.
//!
//! When the async task is cancelled (e.g., user presses ESC), the `ProcessGroupKiller`
//! guard sends SIGKILL to the entire process group, ensuring all child processes are
//! terminated. This is necessary because `kill_on_drop(true)` only kills the direct
//! child process, not processes spawned by the shell.

use super::error::ToolError;
use super::limits::OutputLimits;
use super::truncation::{format_truncation_warning, process_output_lines, truncate_output};
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

/// Shared abort signal for bash tool cancellation
static BASH_ABORT_FLAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set the abort flag to request cancellation of running bash commands
pub fn request_bash_abort() {
    BASH_ABORT_FLAG.store(true, std::sync::atomic::Ordering::Release);
}

/// Clear the abort flag (call before starting a new command)
pub fn clear_bash_abort() {
    BASH_ABORT_FLAG.store(false, std::sync::atomic::Ordering::Release);
}

/// Check if abort has been requested
fn is_bash_abort_requested() -> bool {
    BASH_ABORT_FLAG.load(std::sync::atomic::Ordering::Acquire)
}

/// Guard that kills the entire process group when dropped (Unix only).
///
/// This is necessary because `kill_on_drop(true)` only sends SIGKILL to the direct
/// child process, not to the process group. When the shell spawns child processes
/// (e.g., `npm run dev` spawning `node`), we need to kill the entire process group
/// to ensure all descendants are terminated.
///
/// On drop, sends SIGKILL to the negative PID (process group).
#[cfg(unix)]
struct ProcessGroupKiller {
    /// The process group ID (same as the shell's PID when using process_group(0))
    pgid: Option<u32>,
}

#[cfg(unix)]
impl ProcessGroupKiller {
    /// Create a new ProcessGroupKiller from a Child handle.
    /// Returns None if the child's PID cannot be obtained.
    fn new(child: &tokio::process::Child) -> Self {
        Self { pgid: child.id() }
    }

    /// Explicitly kill the process group
    #[allow(unsafe_code)]
    fn kill(&self) {
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

/// Callback for streaming output chunks to UI
/// Receives each line of output as it's produced
pub type StreamCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// Bash tool for executing shell commands
pub struct BashTool;

impl BashTool {
    /// Create a new Bash tool instance
    pub fn new() -> Self {
        Self
    }

    /// Execute command with streaming output to UI
    ///
    /// Streams output line-by-line via callback while buffering complete output for LLM.
    /// UI sees full output in real-time; LLM gets truncated buffered result.
    ///
    /// # Arguments
    /// * `args` - Command arguments
    /// * `stream_callback` - Optional callback for streaming output chunks
    ///
    /// # Returns
    /// Complete buffered output (truncated if necessary) for LLM consumption
    pub async fn call_with_streaming(
        &self,
        args: BashArgs,
        stream_callback: Option<StreamCallback>,
    ) -> Result<String, ToolError> {
        if args.command.is_empty() {
            return Err(ToolError::Validation {
                tool: "bash",
                message: "command parameter is required".to_string(),
            });
        }

        // If no streaming callback, use the non-streaming path
        let Some(callback) = stream_callback else {
            return self.call(args).await;
        };

        // Spawn process with piped stdout/stderr for streaming
        // Use process_group(0) to put the shell in its own process group (Unix only)
        // and kill_on_drop(true) as a fallback to terminate the direct child
        #[cfg(unix)]
        let mut child = {
            Command::new("sh")
                .arg("-c")
                .arg(&args.command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .process_group(0) // Create new process group for clean termination
                .kill_on_drop(true) // Fallback: kill direct child if guard fails
                .spawn()
                .map_err(|e| ToolError::Execution {
                    tool: "bash",
                    message: format!("Failed to spawn command: {e}"),
                })?
        };

        #[cfg(not(unix))]
        let mut child = {
            Command::new("sh")
                .arg("-c")
                .arg(&args.command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true) // Kill process when Child is dropped
                .spawn()
                .map_err(|e| ToolError::Execution {
                    tool: "bash",
                    message: format!("Failed to spawn command: {e}"),
                })?
        };

        // Create a guard that will kill the entire process group when dropped.
        // This ensures that if the async task is cancelled (e.g., ESC pressed),
        // all child processes spawned by the shell are also terminated.
        #[cfg(unix)]
        let pg_killer = ProcessGroupKiller::new(&child);

        // Take stdout and stderr handles
        let stdout = child.stdout.take().ok_or(ToolError::Execution {
            tool: "bash",
            message: "Failed to capture stdout".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or(ToolError::Execution {
            tool: "bash",
            message: "Failed to capture stderr".to_string(),
        })?;

        // Buffer for complete output (for LLM)
        let output_buffer = Arc::new(Mutex::new(String::new()));
        let stderr_buffer = Arc::new(Mutex::new(String::new()));

        // Clear abort flag before starting
        clear_bash_abort();

        // Spawn task to read stdout line by line
        let stdout_buffer = output_buffer.clone();
        let stdout_callback = callback.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // Check abort flag
                if is_bash_abort_requested() {
                    break;
                }
                // Stream to UI immediately
                let line_with_newline = format!("{line}\n");
                stdout_callback(&line_with_newline);
                // Buffer for LLM
                stdout_buffer.lock().await.push_str(&line_with_newline);
            }
        });

        // Spawn task to read stderr
        let stderr_buf = stderr_buffer.clone();
        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // Check abort flag
                if is_bash_abort_requested() {
                    break;
                }
                stderr_buf.lock().await.push_str(&line);
                stderr_buf.lock().await.push('\n');
            }
        });

        // Poll for completion or abort
        loop {
            if is_bash_abort_requested() {
                // Kill the process group immediately
                #[cfg(unix)]
                pg_killer.kill();

                // Abort the reader tasks
                stdout_task.abort();
                stderr_task.abort();

                return Err(ToolError::Execution {
                    tool: "bash",
                    message: "Command interrupted by user".to_string(),
                });
            }

            // Check if tasks are done
            if stdout_task.is_finished() && stderr_task.is_finished() {
                break;
            }

            // Small sleep to avoid busy-waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Wait for process to exit
        let status = child.wait().await.map_err(|e| ToolError::Execution {
            tool: "bash",
            message: e.to_string(),
        })?;

        let stdout_content = output_buffer.lock().await.clone();
        let stderr_content = stderr_buffer.lock().await.clone();

        if status.success() {
            // Process successful output with truncation for LLM
            let lines = process_output_lines(&stdout_content);
            let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

            let mut final_output = truncate_result.output;
            let was_truncated =
                truncate_result.char_truncated || truncate_result.remaining_count > 0;

            if was_truncated {
                let warning = format_truncation_warning(
                    truncate_result.remaining_count,
                    "lines",
                    truncate_result.char_truncated,
                    OutputLimits::MAX_OUTPUT_CHARS,
                );
                final_output.push_str(&warning);
            }

            // Append stderr if present (even on success, some commands write warnings to stderr)
            if !stderr_content.trim().is_empty() {
                final_output.push_str("\nStderr: ");
                final_output.push_str(stderr_content.trim());
            }

            Ok(final_output)
        } else {
            // Command failed
            Err(ToolError::Execution {
                tool: "bash",
                message: format!(
                    "exit code {}\nStdout: {}\nStderr: {}",
                    status.code().unwrap_or(-1),
                    stdout_content.trim(),
                    stderr_content.trim()
                ),
            })
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

/// Arguments for Bash tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BashArgs {
    /// The bash command to execute
    pub command: String,
}

impl rig::tool::Tool for BashTool {
    const NAME: &'static str = "bash";

    type Error = ToolError;
    type Args = BashArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a bash command. Returns stdout or error message with stderr."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BashArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        use crate::tool_progress::emit_tool_progress;

        if args.command.is_empty() {
            return Err(ToolError::Validation {
                tool: "bash",
                message: "command parameter is required".to_string(),
            });
        }

        // TOOL-011: Use streaming execution with global callback
        // Spawn process with piped stdout/stderr for streaming
        // Use process_group(0) to put the shell in its own process group (Unix only)
        // and kill_on_drop(true) as a fallback to terminate the direct child
        #[cfg(unix)]
        let mut child = {
            Command::new("sh")
                .arg("-c")
                .arg(&args.command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .process_group(0) // Create new process group for clean termination
                .kill_on_drop(true) // Fallback: kill direct child if guard fails
                .spawn()
                .map_err(|e| ToolError::Execution {
                    tool: "bash",
                    message: format!("Failed to spawn command: {e}"),
                })?
        };

        #[cfg(not(unix))]
        let mut child = {
            Command::new("sh")
                .arg("-c")
                .arg(&args.command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true) // Kill process when Child is dropped
                .spawn()
                .map_err(|e| ToolError::Execution {
                    tool: "bash",
                    message: format!("Failed to spawn command: {e}"),
                })?
        };

        // Create a guard that will kill the entire process group when dropped.
        // This ensures that if the async task is cancelled (e.g., ESC pressed),
        // all child processes spawned by the shell are also terminated.
        #[cfg(unix)]
        let pg_killer = ProcessGroupKiller::new(&child);

        // Take stdout and stderr handles
        let stdout = child.stdout.take().ok_or(ToolError::Execution {
            tool: "bash",
            message: "Failed to capture stdout".to_string(),
        })?;
        let stderr = child.stderr.take().ok_or(ToolError::Execution {
            tool: "bash",
            message: "Failed to capture stderr".to_string(),
        })?;

        // Buffer for complete output (for LLM)
        let output_buffer = Arc::new(Mutex::new(String::new()));
        let stderr_buffer = Arc::new(Mutex::new(String::new()));

        // Clear abort flag before starting
        clear_bash_abort();

        // Spawn task to read stdout line by line and stream to UI
        let stdout_buffer = output_buffer.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // Check abort flag
                if is_bash_abort_requested() {
                    break;
                }
                let line_with_newline = format!("{line}\n");
                // TOOL-011: Stream to UI via global callback
                emit_tool_progress(&line_with_newline);
                // Buffer for LLM
                stdout_buffer.lock().await.push_str(&line_with_newline);
            }
        });

        // Spawn task to read stderr
        let stderr_buf = stderr_buffer.clone();
        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                // Check abort flag
                if is_bash_abort_requested() {
                    break;
                }
                stderr_buf.lock().await.push_str(&line);
                stderr_buf.lock().await.push('\n');
            }
        });

        // Poll for completion or abort
        loop {
            if is_bash_abort_requested() {
                // Kill the process group immediately
                #[cfg(unix)]
                pg_killer.kill();

                // Abort the reader tasks
                stdout_task.abort();
                stderr_task.abort();

                return Err(ToolError::Execution {
                    tool: "bash",
                    message: "Command interrupted by user".to_string(),
                });
            }

            // Check if tasks are done
            if stdout_task.is_finished() && stderr_task.is_finished() {
                break;
            }

            // Small sleep to avoid busy-waiting
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        // Wait for process to exit
        let status = child.wait().await.map_err(|e| ToolError::Execution {
            tool: "bash",
            message: e.to_string(),
        })?;

        let stdout_content = output_buffer.lock().await.clone();
        let stderr_content = stderr_buffer.lock().await.clone();

        if status.success() {
            // Process successful output with truncation for LLM
            let lines = process_output_lines(&stdout_content);
            let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

            let mut final_output = truncate_result.output;
            let was_truncated =
                truncate_result.char_truncated || truncate_result.remaining_count > 0;

            if was_truncated {
                let warning = format_truncation_warning(
                    truncate_result.remaining_count,
                    "lines",
                    truncate_result.char_truncated,
                    OutputLimits::MAX_OUTPUT_CHARS,
                );
                final_output.push_str(&warning);
            }

            // Append stderr if present (even on success, some commands write warnings to stderr)
            if !stderr_content.trim().is_empty() {
                final_output.push_str("\nStderr: ");
                final_output.push_str(stderr_content.trim());
            }

            Ok(final_output)
        } else {
            // Command failed
            Err(ToolError::Execution {
                tool: "bash",
                message: format!(
                    "exit code {}\nStdout: {}\nStderr: {}",
                    status.code().unwrap_or(-1),
                    stdout_content.trim(),
                    stderr_content.trim()
                ),
            })
        }
    }
}
