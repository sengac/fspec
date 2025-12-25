//! LS tool implementation
//!
//! Lists directory contents with file metadata (permissions, size, modification time).
//! Uses tokio::fs for non-blocking async I/O.

use crate::{
    error::ToolError,
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Permission bit masks and their corresponding characters (Unix only)
#[cfg(unix)]
const PERMISSION_BITS: &[(u32, char)] = &[
    (0o400, 'r'),
    (0o200, 'w'),
    (0o100, 'x'),
    (0o040, 'r'),
    (0o020, 'w'),
    (0o010, 'x'),
    (0o004, 'r'),
    (0o002, 'w'),
    (0o001, 'x'),
];

/// LS tool for listing directory contents
pub struct LsTool;

impl LsTool {
    /// Create a new LS tool instance
    pub fn new() -> Self {
        Self
    }

    /// Format file mode as permission string (e.g., drwxr-xr-x or -rw-r--r--)
    #[cfg(unix)]
    fn format_mode(mode: u32, is_directory: bool) -> String {
        let type_char = if is_directory { 'd' } else { '-' };
        let permissions: String = PERMISSION_BITS
            .iter()
            .map(|(bit, ch)| if mode & bit != 0 { *ch } else { '-' })
            .collect();
        format!("{type_char}{permissions}")
    }

    /// Format file mode as permission string (Windows version - simplified)
    #[cfg(windows)]
    fn format_mode(is_readonly: bool, is_directory: bool) -> String {
        let type_char = if is_directory { 'd' } else { '-' };
        // Windows doesn't have Unix-style permissions, show simplified view
        let permissions = if is_readonly {
            "r--r--r--"
        } else {
            "rw-rw-rw-"
        };
        format!("{type_char}{permissions}")
    }

    /// Format date as YYYY-MM-DD HH:MM
    fn format_date(time: std::time::SystemTime) -> String {
        use std::time::UNIX_EPOCH;

        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs() as i64;

        // Simple date formatting without external dependencies
        // Calculate date components from Unix timestamp
        let days = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;

        // Calculate year, month, day from days since epoch (1970-01-01)
        let (year, month, day) = Self::days_to_ymd(days);

        format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}")
    }

    /// Convert days since Unix epoch to (year, month, day)
    fn days_to_ymd(days: i64) -> (i32, u32, u32) {
        // Algorithm based on Howard Hinnant's date algorithms
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u32;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if m <= 2 { y + 1 } else { y } as i32;
        (year, m, d)
    }
}

impl Default for LsTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

/// Arguments for LS tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct LsArgs {
    /// Directory to list (optional, defaults to current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl rig::tool::Tool for LsTool {
    const NAME: &'static str = "ls";

    type Error = ToolError;
    type Args = LsArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "ls".to_string(),
            description: "List directory contents with file metadata. \
                Returns formatted output showing permissions, size, modification time, and name for each entry.\n\n\
                Usage:\n\
                - Directories are listed first, then files, alphabetically within each group\n\
                - Output format: 'drwxr-xr-x  4096  2025-01-15 10:30  dirname/' for directories\n\
                - Output format: '-rw-r--r--  1234  2025-01-15 10:30  filename.ts' for files\n\
                - Returns \"Directory not found\" for non-existent paths\n\
                - Output is truncated at 30000 characters with truncation warning".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(LsArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let dir_path = args.path.as_deref().unwrap_or(".");
        let path = Path::new(dir_path);

        // Check if path exists (async)
        match tokio::fs::try_exists(path).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(ToolError::NotFound {
                    tool: "ls",
                    message: format!("Directory not found: {dir_path}"),
                })
            }
            Err(e) => {
                return Err(ToolError::File {
                    tool: "ls",
                    message: e.to_string(),
                })
            }
        }

        // Check if path is a directory (async)
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| ToolError::File {
                tool: "ls",
                message: e.to_string(),
            })?;
        if !metadata.is_dir() {
            return Err(ToolError::Validation {
                tool: "ls",
                message: format!("Not a directory: {dir_path}"),
            });
        }

        // Read directory entries (async)
        let mut read_dir = tokio::fs::read_dir(path)
            .await
            .map_err(|e| ToolError::File {
                tool: "ls",
                message: e.to_string(),
            })?;

        // Collect entries with metadata (or None for permission errors)
        let mut dirs: Vec<(String, Option<std::fs::Metadata>)> = Vec::new();
        let mut files: Vec<(String, Option<std::fs::Metadata>)> = Vec::new();
        let mut skipped_count: usize = 0;

        loop {
            let entry = match read_dir.next_entry().await {
                Ok(Some(e)) => e,
                Ok(None) => break, // No more entries
                Err(_) => {
                    skipped_count += 1;
                    continue; // Skip entries we can't read
                }
            };
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry
                .file_type()
                .await
                .map(|ft| ft.is_dir())
                .unwrap_or(false);
            let metadata = entry.metadata().await.ok();

            if is_dir {
                dirs.push((name, metadata));
            } else {
                files.push((name, metadata));
            }
        }

        // Sort alphabetically
        dirs.sort_by(|a, b| a.0.cmp(&b.0));
        files.sort_by(|a, b| a.0.cmp(&b.0));

        // Check for empty directory
        if dirs.is_empty() && files.is_empty() {
            return Ok("(empty directory)".to_string());
        }

        // Format output: directories first, then files
        let mut formatted_lines: Vec<String> = Vec::new();

        for (name, metadata) in &dirs {
            let line = if let Some(meta) = metadata {
                let size = meta.len();
                let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                #[cfg(unix)]
                let permissions = Self::format_mode(meta.permissions().mode(), true);
                #[cfg(windows)]
                let permissions = Self::format_mode(meta.permissions().readonly(), true);
                let date = Self::format_date(mtime);
                format!("{permissions}  {size:>8}  {date}  {name}/")
            } else {
                // Handle permission errors gracefully
                format!("d---------  ????????  ????-??-?? ??:??  {name}/")
            };
            formatted_lines.push(line);
        }

        for (name, metadata) in &files {
            let line = if let Some(meta) = metadata {
                let size = meta.len();
                let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
                #[cfg(unix)]
                let permissions = Self::format_mode(meta.permissions().mode(), false);
                #[cfg(windows)]
                let permissions = Self::format_mode(meta.permissions().readonly(), false);
                let date = Self::format_date(mtime);
                format!("{permissions}  {size:>8}  {date}  {name}")
            } else {
                // Handle permission errors gracefully
                format!("----------  ????????  ????-??-?? ??:??  {name}")
            };
            formatted_lines.push(line);
        }

        // Apply truncation
        let output = formatted_lines.join("\n");
        let lines = process_output_lines(&output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

        let mut final_output = truncate_result.output;
        let was_truncated = truncate_result.char_truncated || truncate_result.remaining_count > 0;

        if was_truncated {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "entries",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            final_output.push_str(&warning);
        }

        // Report skipped entries if any
        if skipped_count > 0 {
            final_output.push_str(&format!(
                "\n(Note: {skipped_count} entries skipped due to permission errors)"
            ));
        }

        Ok(final_output)
    }
}
