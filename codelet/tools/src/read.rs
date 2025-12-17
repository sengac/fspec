//! Read tool implementation
//!
//! Reads file contents with line numbers, supporting offset and limit.

use super::error::ToolError;
use super::limits::OutputLimits;
use super::truncation::{format_truncation_warning, truncate_line_default};
use super::validation::{read_file_contents, require_absolute_path, require_file_exists};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Read tool for reading file contents
pub struct ReadTool;

impl ReadTool {
    /// Create a new Read tool instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

/// Arguments for Read tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ReadArgs {
    /// Absolute path to the file to read
    pub file_path: String,
    /// 1-based line number to start reading from (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Number of lines to read (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

impl rig::tool::Tool for ReadTool {
    const NAME: &'static str = "read";

    type Error = ToolError;
    type Args = ReadArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "read".to_string(),
            description: "Reads a file from the local filesystem. You can access any file directly by using this tool.\n\n\
                Usage:\n\
                - The file_path parameter must be an absolute path, not a relative path\n\
                - By default, it reads up to 2000 lines starting from the beginning of the file\n\
                - You can optionally specify a line offset and limit (especially handy for long files), but it's recommended to read the whole file by not providing these parameters\n\
                - Any lines longer than 2000 characters will be truncated\n\
                - Results are returned using cat -n format, with line numbers starting at 1".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ReadArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&args.file_path).map_err(|e| ToolError::Validation {
            tool: "read",
            message: e.content,
        })?;

        // Check file exists (async)
        require_file_exists(path, &args.file_path)
            .await
            .map_err(|e| ToolError::Validation {
                tool: "read",
                message: e.content,
            })?;

        // Read file content (async)
        let content = read_file_contents(path)
            .await
            .map_err(|e| ToolError::File {
                tool: "read",
                message: e.content,
            })?;

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // Parse offset and limit
        let offset = args.offset.unwrap_or(1);
        let limit = args.limit.unwrap_or(OutputLimits::MAX_LINES);

        // Calculate range (offset is 1-based)
        let start_idx = offset.saturating_sub(1);

        // Apply line limit
        let effective_limit = limit.min(OutputLimits::MAX_LINES);
        let end_idx = (start_idx + effective_limit).min(total_lines);

        // Format lines with numbers and truncate long lines
        let mut output_lines: Vec<String> = Vec::new();
        for (idx, line) in lines[start_idx..end_idx].iter().enumerate() {
            let line_num = start_idx + idx + 1; // 1-based line numbers
            let truncated_line = truncate_line_default(line);
            output_lines.push(format!("{line_num}: {truncated_line}"));
        }

        // Check if we need to truncate due to line limit
        let lines_after_range = total_lines.saturating_sub(end_idx);
        let was_truncated = end_idx < total_lines && lines_after_range > 0;

        // Build output
        let mut output = output_lines.join("\n");

        if was_truncated {
            let remaining = total_lines - end_idx;
            let warning =
                format_truncation_warning(remaining, "lines", true, OutputLimits::MAX_OUTPUT_CHARS);
            output.push('\n');
            output.push_str(&warning);
        }

        Ok(output)
    }
}
