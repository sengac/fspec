//! Bash tool implementation
//!
//! Executes shell commands with output truncation.
//! Supports streaming output to UI while buffering complete output for LLM.

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
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&args.command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::Execution {
                tool: "bash",
                message: format!("Failed to spawn command: {e}"),
            })?;

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

        // Spawn task to read stdout line by line
        let stdout_buffer = output_buffer.clone();
        let stdout_callback = callback.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
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
                stderr_buf.lock().await.push_str(&line);
                stderr_buf.lock().await.push('\n');
            }
        });

        // Wait for stdout/stderr tasks to complete
        let _ = stdout_task.await;
        let _ = stderr_task.await;
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
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&args.command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| ToolError::Execution {
                tool: "bash",
                message: format!("Failed to spawn command: {e}"),
            })?;

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

        // Spawn task to read stdout line by line and stream to UI
        let stdout_buffer = output_buffer.clone();
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
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
                stderr_buf.lock().await.push_str(&line);
                stderr_buf.lock().await.push('\n');
            }
        });

        // Wait for stdout/stderr tasks to complete
        let _ = stdout_task.await;
        let _ = stderr_task.await;
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
