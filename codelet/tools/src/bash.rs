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
//!
//! # Output Formatting
//!
//! Output is returned without "Stdout:" or "Stderr:" labels for cleaner LLM consumption.
//! - On success: stdout content, with stderr appended if present
//! - On failure: Clear error message with exit code, followed by any output

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

// ============================================================================
// Abort Signal Management
// ============================================================================

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
struct ProcessGroupKiller {
    /// The process group ID (same as the shell's PID when using process_group(0))
    pgid: Option<u32>,
}

#[cfg(unix)]
impl ProcessGroupKiller {
    /// Create a new ProcessGroupKiller from a Child handle.
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

// ============================================================================
// Output Formatting (SOLID: Single Responsibility)
// ============================================================================

/// Holds the raw output from a bash command execution.
///
/// Separates data capture from formatting (Single Responsibility Principle).
/// Provides composable formatting methods for different output needs.
struct BashOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    success: bool,
}

/// Marker for stderr content to enable red styling in UI
const STDERR_MARKER: &str = "⚠stderr⚠";

impl BashOutput {
    /// Create from process output and exit status
    fn from_execution(
        stdout: String,
        stderr: String,
        status: std::process::ExitStatus,
    ) -> Self {
        Self {
            stdout,
            stderr,
            exit_code: status.code(),
            success: status.success(),
        }
    }

    /// Format output for successful command execution.
    ///
    /// Returns stdout with truncation applied, and stderr appended if present.
    /// No labels like "Stderr:" are used - just clean content.
    fn format_success(&self) -> String {
        // Apply truncation to stdout
        let lines = process_output_lines(&self.stdout);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

        let mut output = truncate_result.output;
        let was_truncated =
            truncate_result.char_truncated || truncate_result.remaining_count > 0;

        if was_truncated {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "lines",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            output.push_str(&warning);
        }

        // Append stderr if present (warnings/diagnostics from successful commands)
        self.append_stderr_if_present(&mut output);

        output
    }

    /// Format output for failed command execution.
    ///
    /// Returns a clear error message with exit code, followed by combined output.
    /// No labels like "Stdout:" or "Stderr:" - just clean content.
    fn format_error(&self) -> String {
        let code = self.exit_code.unwrap_or(-1);

        // Combine stdout and stderr for error context
        let combined = self.combine_outputs();

        if combined.is_empty() {
            format!("Command failed with exit code {code}")
        } else {
            format!("Command failed with exit code {code}\n{combined}")
        }
    }

    /// Append stderr to output if present (helper for DRY)
    /// Marks stderr lines with STDERR_MARKER for red styling in UI
    fn append_stderr_if_present(&self, output: &mut String) {
        let stderr_trimmed = self.stderr.trim();
        if !stderr_trimmed.is_empty() {
            // Ensure there's a newline before stderr content
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            // Mark each stderr line with marker for red rendering
            for line in stderr_trimmed.lines() {
                output.push_str(STDERR_MARKER);
                output.push_str(line);
                output.push('\n');
            }
        }
    }

    /// Combine stdout and stderr into a single string (helper for DRY)
    /// Marks stderr lines with STDERR_MARKER for red styling in UI
    fn combine_outputs(&self) -> String {
        let stdout_trimmed = self.stdout.trim();
        let stderr_trimmed = self.stderr.trim();

        match (stdout_trimmed.is_empty(), stderr_trimmed.is_empty()) {
            (true, true) => String::new(),
            (false, true) => stdout_trimmed.to_string(),
            (true, false) => {
                // Mark each stderr line
                stderr_trimmed
                    .lines()
                    .map(|line| format!("{STDERR_MARKER}{line}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            (false, false) => {
                // stdout unchanged, stderr marked
                let marked_stderr = stderr_trimmed
                    .lines()
                    .map(|line| format!("{STDERR_MARKER}{line}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{stdout_trimmed}\n{marked_stderr}")
            }
        }
    }

    /// Convert to Result based on success status
    fn into_result(self) -> Result<String, ToolError> {
        if self.success {
            Ok(self.format_success())
        } else {
            Err(ToolError::Execution {
                tool: "bash",
                message: self.format_error(),
            })
        }
    }
}

// ============================================================================
// Stream Buffers (DRY: Reusable component)
// ============================================================================

/// Manages stdout and stderr buffers for command execution.
///
/// Encapsulates buffer creation and content extraction (DRY principle).
struct StreamBuffers {
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
}

impl StreamBuffers {
    fn new() -> Self {
        Self {
            stdout: Arc::new(Mutex::new(String::new())),
            stderr: Arc::new(Mutex::new(String::new())),
        }
    }

    fn stdout_handle(&self) -> Arc<Mutex<String>> {
        Arc::clone(&self.stdout)
    }

    fn stderr_handle(&self) -> Arc<Mutex<String>> {
        Arc::clone(&self.stderr)
    }

    async fn extract(self) -> (String, String) {
        let stdout = self.stdout.lock().await.clone();
        let stderr = self.stderr.lock().await.clone();
        (stdout, stderr)
    }
}

// ============================================================================
// Process Spawning (DRY: Unified spawn logic)
// ============================================================================

/// Spawn a shell command with proper configuration.
///
/// Handles platform-specific setup (process groups on Unix).
fn spawn_command(command: &str) -> Result<tokio::process::Child, ToolError> {
    #[cfg(unix)]
    {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0) // Create new process group for clean termination
            .kill_on_drop(true) // Fallback: kill direct child if guard fails
            .spawn()
            .map_err(|e| ToolError::Execution {
                tool: "bash",
                message: format!("Failed to spawn command: {e}"),
            })
    }

    #[cfg(not(unix))]
    {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Kill process when Child is dropped
            .spawn()
            .map_err(|e| ToolError::Execution {
                tool: "bash",
                message: format!("Failed to spawn command: {e}"),
            })
    }
}

/// Take stdout and stderr handles from child process.
fn take_stdio_handles(
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

// ============================================================================
// Stream Reader Tasks (DRY: Unified task spawning)
// ============================================================================

/// Spawn a task to read stdout and optionally stream to callback.
fn spawn_stdout_reader(
    stdout: tokio::process::ChildStdout,
    buffer: Arc<Mutex<String>>,
    callback: Option<StreamCallback>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if is_bash_abort_requested() {
                break;
            }
            let line_with_newline = format!("{line}\n");
            // Stream to callback if provided
            if let Some(ref cb) = callback {
                cb(&line_with_newline);
            }
            // Buffer for final result
            buffer.lock().await.push_str(&line_with_newline);
        }
    })
}

/// Spawn a task to read stderr into buffer and optionally stream to callback with is_stderr flag.
fn spawn_stderr_reader(
    stderr: tokio::process::ChildStderr,
    buffer: Arc<Mutex<String>>,
    stream_to_ui: bool,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        use crate::tool_progress::emit_tool_progress;
        
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if is_bash_abort_requested() {
                break;
            }
            let line_with_newline = format!("{line}\n");
            // Stream to UI with is_stderr=true if enabled
            if stream_to_ui {
                emit_tool_progress(&line_with_newline, true);
            }
            let mut buf = buffer.lock().await;
            buf.push_str(&line_with_newline);
        }
    })
}

/// Wait for reader tasks with abort checking.
///
/// Returns Err if aborted, Ok(()) if completed normally.
#[cfg(unix)]
async fn wait_for_tasks_with_abort(
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
    pg_killer: &ProcessGroupKiller,
) -> Result<(), ToolError> {
    loop {
        if is_bash_abort_requested() {
            pg_killer.kill();
            stdout_task.abort();
            stderr_task.abort();
            return Err(ToolError::Execution {
                tool: "bash",
                message: "Command interrupted by user".to_string(),
            });
        }

        if stdout_task.is_finished() && stderr_task.is_finished() {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

#[cfg(not(unix))]
async fn wait_for_tasks_with_abort(
    stdout_task: tokio::task::JoinHandle<()>,
    stderr_task: tokio::task::JoinHandle<()>,
) -> Result<(), ToolError> {
    loop {
        if is_bash_abort_requested() {
            stdout_task.abort();
            stderr_task.abort();
            return Err(ToolError::Execution {
                tool: "bash",
                message: "Command interrupted by user".to_string(),
            });
        }

        if stdout_task.is_finished() && stderr_task.is_finished() {
            return Ok(());
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

// ============================================================================
// BashTool Implementation
// ============================================================================

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

        // Spawn process
        let mut child = spawn_command(&args.command)?;

        // Create process group killer guard (Unix only)
        #[cfg(unix)]
        let pg_killer = ProcessGroupKiller::new(&child);

        // Take stdio handles
        let (stdout, stderr) = take_stdio_handles(&mut child)?;

        // Set up buffers
        let buffers = StreamBuffers::new();
        clear_bash_abort();

        // Spawn reader tasks with streaming callback
        let stdout_task = spawn_stdout_reader(
            stdout,
            buffers.stdout_handle(),
            Some(callback),
        );
        // Don't stream stderr to the provided callback (it only handles stdout)
        let stderr_task = spawn_stderr_reader(stderr, buffers.stderr_handle(), false);

        // Wait for completion with abort checking
        #[cfg(unix)]
        wait_for_tasks_with_abort(stdout_task, stderr_task, &pg_killer).await?;
        #[cfg(not(unix))]
        wait_for_tasks_with_abort(stdout_task, stderr_task).await?;

        // Wait for process exit
        let status = child.wait().await.map_err(|e| ToolError::Execution {
            tool: "bash",
            message: e.to_string(),
        })?;

        // Build and format output
        let (stdout_content, stderr_content) = buffers.extract().await;
        BashOutput::from_execution(stdout_content, stderr_content, status).into_result()
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// rig::tool::Tool Implementation
// ============================================================================

/// Arguments for Bash tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct BashArgs {
    /// The bash command to execute
    pub command: String,
}

impl rig::tool::Tool for BashTool {
    const NAME: &'static str = "Bash";

    type Error = ToolError;
    type Args = BashArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "Bash".to_string(),
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

        // Spawn process
        let mut child = spawn_command(&args.command)?;

        // Create process group killer guard (Unix only)
        #[cfg(unix)]
        let pg_killer = ProcessGroupKiller::new(&child);

        // Take stdio handles
        let (stdout, stderr) = take_stdio_handles(&mut child)?;

        // Set up buffers
        let buffers = StreamBuffers::new();
        clear_bash_abort();

        // Spawn reader tasks with global progress callback
        let stdout_buffer = buffers.stdout_handle();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if is_bash_abort_requested() {
                    break;
                }
                let line_with_newline = format!("{line}\n");
                // TOOL-011: Stream to UI via global callback (stdout)
                emit_tool_progress(&line_with_newline, false);
                // Buffer for LLM
                stdout_buffer.lock().await.push_str(&line_with_newline);
            }
        });
        // Stream stderr to UI with is_stderr=true for red styling
        let stderr_task = spawn_stderr_reader(stderr, buffers.stderr_handle(), true);

        // Wait for completion with abort checking
        #[cfg(unix)]
        wait_for_tasks_with_abort(stdout_task, stderr_task, &pg_killer).await?;
        #[cfg(not(unix))]
        wait_for_tasks_with_abort(stdout_task, stderr_task).await?;

        // Wait for process exit
        let status = child.wait().await.map_err(|e| ToolError::Execution {
            tool: "bash",
            message: e.to_string(),
        })?;

        // Build and format output
        let (stdout_content, stderr_content) = buffers.extract().await;
        BashOutput::from_execution(stdout_content, stderr_content, status).into_result()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_output_format_success_stdout_only() {
        let output = BashOutput {
            stdout: "hello world\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        assert_eq!(result, "hello world\n");
    }

    #[test]
    fn test_bash_output_format_success_with_stderr() {
        let output = BashOutput {
            stdout: "output\n".to_string(),
            stderr: "warning: something\n".to_string(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        assert!(result.contains("output"));
        assert!(result.contains("warning: something"));
        // Should NOT contain "Stderr:" label
        assert!(!result.contains("Stderr:"));
    }

    #[test]
    fn test_bash_output_format_error_no_output() {
        let output = BashOutput {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(1),
            success: false,
        };
        let result = output.format_error();
        assert_eq!(result, "Command failed with exit code 1");
    }

    #[test]
    fn test_bash_output_format_error_with_stderr() {
        let output = BashOutput {
            stdout: String::new(),
            stderr: "file not found\n".to_string(),
            exit_code: Some(2),
            success: false,
        };
        let result = output.format_error();
        assert!(result.contains("Command failed with exit code 2"));
        assert!(result.contains("file not found"));
        // Should NOT contain "Stderr:" label
        assert!(!result.contains("Stderr:"));
        assert!(!result.contains("Stdout:"));
    }

    #[test]
    fn test_bash_output_format_error_with_both() {
        let output = BashOutput {
            stdout: "partial output\n".to_string(),
            stderr: "error details\n".to_string(),
            exit_code: Some(1),
            success: false,
        };
        let result = output.format_error();
        assert!(result.contains("Command failed with exit code 1"));
        assert!(result.contains("partial output"));
        assert!(result.contains("error details"));
        // Should NOT contain labels
        assert!(!result.contains("Stderr:"));
        assert!(!result.contains("Stdout:"));
    }

    #[test]
    fn test_bash_output_into_result_success() {
        let output = BashOutput {
            stdout: "test\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.into_result();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bash_output_into_result_failure() {
        let output = BashOutput {
            stdout: String::new(),
            stderr: "error\n".to_string(),
            exit_code: Some(1),
            success: false,
        };
        let result = output.into_result();
        assert!(result.is_err());
    }

    // ========== Stderr Marker Tests ==========

    #[test]
    fn test_bash_output_stderr_marked_in_success() {
        let output = BashOutput {
            stdout: "stdout line\n".to_string(),
            stderr: "stderr line\n".to_string(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        // Stderr should be marked with STDERR_MARKER
        assert!(result.contains("⚠stderr⚠stderr line"));
        // Stdout should NOT be marked
        assert!(result.contains("stdout line"));
        assert!(!result.contains("⚠stderr⚠stdout"));
    }

    #[test]
    fn test_bash_output_stderr_marked_in_error() {
        let output = BashOutput {
            stdout: "stdout line\n".to_string(),
            stderr: "stderr line\n".to_string(),
            exit_code: Some(1),
            success: false,
        };
        let result = output.format_error();
        // Stderr should be marked with STDERR_MARKER
        assert!(result.contains("⚠stderr⚠stderr line"));
        // Stdout should NOT be marked
        assert!(result.contains("stdout line"));
        assert!(!result.contains("⚠stderr⚠stdout"));
    }

    #[test]
    fn test_bash_output_stderr_only_marked() {
        // When only stderr, it should still be marked
        let output = BashOutput {
            stdout: String::new(),
            stderr: "only stderr\n".to_string(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        assert!(result.contains("⚠stderr⚠only stderr"));
    }

    #[test]
    fn test_bash_output_multiline_stderr_all_marked() {
        // Each line of stderr should be marked individually
        let output = BashOutput {
            stdout: "stdout\n".to_string(),
            stderr: "error line 1\nerror line 2\nerror line 3\n".to_string(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        assert!(result.contains("⚠stderr⚠error line 1"));
        assert!(result.contains("⚠stderr⚠error line 2"));
        assert!(result.contains("⚠stderr⚠error line 3"));
        // Stdout should NOT be marked
        assert!(result.contains("stdout"));
        assert!(!result.contains("⚠stderr⚠stdout"));
    }

    #[test]
    fn test_bash_output_no_stderr_no_marker() {
        // When no stderr, no marker should appear
        let output = BashOutput {
            stdout: "just stdout\n".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        assert!(!result.contains("⚠stderr⚠"));
        assert!(result.contains("just stdout"));
    }

    #[test]
    fn test_bash_output_empty_stderr_no_marker() {
        // Whitespace-only stderr should not produce markers
        let output = BashOutput {
            stdout: "stdout\n".to_string(),
            stderr: "   \n  \n".to_string(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.format_success();
        assert!(!result.contains("⚠stderr⚠"));
    }

    #[test]
    fn test_bash_output_stderr_marker_constant() {
        // Verify the marker constant matches what TypeScript expects
        assert_eq!(STDERR_MARKER, "⚠stderr⚠");
    }

    #[test]
    fn test_bash_output_error_with_only_stderr() {
        // Error case with only stderr should mark it
        let output = BashOutput {
            stdout: String::new(),
            stderr: "fatal error\n".to_string(),
            exit_code: Some(1),
            success: false,
        };
        let result = output.format_error();
        assert!(result.contains("Command failed with exit code 1"));
        assert!(result.contains("⚠stderr⚠fatal error"));
    }

    #[test]
    fn test_bash_output_into_result_preserves_markers() {
        // Verify into_result() preserves stderr markers
        let output = BashOutput {
            stdout: "out\n".to_string(),
            stderr: "err\n".to_string(),
            exit_code: Some(0),
            success: true,
        };
        let result = output.into_result().unwrap();
        assert!(result.contains("⚠stderr⚠err"));
        assert!(result.contains("out"));
        assert!(!result.contains("⚠stderr⚠out"));
    }
}
