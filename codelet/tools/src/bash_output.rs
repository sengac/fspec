//! Output formatting for bash command results.
//!
//! Separates data capture from formatting (Single Responsibility Principle).
//! Provides composable formatting methods for different output scenarios.

use super::error::ToolError;
use super::limits::OutputLimits;
use super::truncation::{format_truncation_warning, process_output_lines, truncate_output};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Marker for stderr content to enable red styling in UI
pub const STDERR_MARKER: &str = "⚠stderr⚠";

/// Holds the raw output from a bash command execution.
///
/// Separates data capture from formatting (Single Responsibility Principle).
/// Provides composable formatting methods for different output needs.
pub struct BashOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

impl BashOutput {
    /// Create from process output and exit status
    pub fn from_execution(
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
    pub fn format_success(&self) -> String {
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
    pub fn format_error(&self) -> String {
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
    pub fn append_stderr_if_present(&self, output: &mut String) {
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
    pub fn combine_outputs(&self) -> String {
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
    pub fn into_result(self) -> Result<String, ToolError> {
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

/// Manages stdout and stderr buffers for command execution.
///
/// Encapsulates buffer creation and content extraction (DRY principle).
#[derive(Default)]
pub struct StreamBuffers {
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
}

impl StreamBuffers {
    /// Create new empty stream buffers.
    pub fn new() -> Self {
        Self {
            stdout: Arc::new(Mutex::new(String::new())),
            stderr: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Get a clone of the stdout buffer handle.
    pub fn stdout_handle(&self) -> Arc<Mutex<String>> {
        Arc::clone(&self.stdout)
    }

    /// Get a clone of the stderr buffer handle.
    pub fn stderr_handle(&self) -> Arc<Mutex<String>> {
        Arc::clone(&self.stderr)
    }

    /// Extract the buffered stdout and stderr content.
    pub async fn extract(self) -> (String, String) {
        let stdout = self.stdout.lock().await.clone();
        let stderr = self.stderr.lock().await.clone();
        (stdout, stderr)
    }
}
