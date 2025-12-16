//! Bash tool implementation
//!
//! Executes shell commands with output truncation and timeout support.

use super::limits::OutputLimits;
use super::truncation::{format_truncation_warning, process_output_lines, truncate_output};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Default timeout for command execution (120 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Bash tool for executing shell commands
pub struct BashTool {
    timeout: Duration,
}

impl BashTool {
    /// Create a new Bash tool instance with default timeout
    pub fn new() -> Self {
        Self::with_timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }

    /// Create a new Bash tool instance with custom timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
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

/// Error type for Bash tool
#[derive(Debug, thiserror::Error)]
pub enum BashError {
    #[error("Command execution error: {0}")]
    ExecutionError(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
}

impl rig::tool::Tool for BashTool {
    const NAME: &'static str = "bash";

    type Error = BashError;
    type Args = BashArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "bash".to_string(),
            description: "Execute a bash command. Returns stdout or error message with stderr. \
                Commands timeout after 120 seconds by default."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(BashArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        if args.command.is_empty() {
            return Err(BashError::ExecutionError(
                "command parameter is required".to_string(),
            ));
        }

        // Execute command with timeout
        let result = timeout(
            self.timeout,
            Command::new("sh").arg("-c").arg(&args.command).output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if output.status.success() {
                    // Process successful output
                    let lines = process_output_lines(&stdout);
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
                    Err(BashError::ExecutionError(format!(
                        "exit code {}\nStdout: {}\nStderr: {}",
                        output.status.code().unwrap_or(-1),
                        stdout.trim(),
                        stderr.trim()
                    )))
                }
            }
            Ok(Err(e)) => Err(BashError::ExecutionError(e.to_string())),
            Err(_) => Err(BashError::TimeoutError(format!(
                "Command timed out after {} seconds",
                self.timeout.as_secs()
            ))),
        }
    }
}
