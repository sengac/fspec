//! GlobTool implementation using ignore crate
//!
//! Uses ignore crate for gitignore-aware file walking
//! and globset for glob pattern matching.
//! Uses tokio::fs for non-blocking async I/O.

use crate::{
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
};
use globset::GlobBuilder;
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::time::SystemTime;

/// GlobTool for gitignore-aware file pattern matching
pub struct GlobTool;

impl GlobTool {
    /// Create a new GlobTool
    pub fn new() -> Self {
        Self
    }

    /// Get file modification time for sorting (async, non-blocking)
    async fn get_mtime(path: &PathBuf) -> SystemTime {
        tokio::fs::metadata(path)
            .await
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

/// Arguments for Glob tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GlobArgs {
    /// The glob pattern to match files
    pub pattern: String,
    /// Directory to search in (optional, defaults to current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Error type for Glob tool
#[derive(Debug, thiserror::Error)]
pub enum GlobError {
    #[error("Pattern error: {0}")]
    PatternError(String),
    #[error("Path error: {0}")]
    PathError(String),
}

impl rig::tool::Tool for GlobTool {
    const NAME: &'static str = "glob";

    type Error = GlobError;
    type Args = GlobArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "glob".to_string(),
            description: "Fast file pattern matching tool that works with any codebase size. \
                Supports glob patterns like \"**/*.js\" or \"src/**/*.ts\". \
                Returns matching file paths one per line.\n\n\
                Usage:\n\
                - Pattern supports glob syntax: *, **, ?, {a,b}, [abc]\n\
                - Respects .gitignore by default\n\
                - Returns \"No matches found\" for patterns with no matching files\n\
                - Output is truncated at 30000 characters with truncation warning"
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GlobArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Build glob matcher
        let matcher = GlobBuilder::new(&args.pattern)
            .literal_separator(false)
            .build()
            .map(|g| g.compile_matcher())
            .map_err(|e| GlobError::PatternError(e.to_string()))?;

        // Get search path
        let search_path = args.path.as_deref().unwrap_or(".");
        let path = PathBuf::from(search_path);

        // Check if path exists (async, non-blocking)
        match tokio::fs::try_exists(&path).await {
            Ok(true) => {}
            Ok(false) => {
                return Err(GlobError::PathError(format!(
                    "Path does not exist: {search_path}"
                )));
            }
            Err(e) => {
                return Err(GlobError::PathError(format!("Error checking path: {e}")));
            }
        }

        // Collect matching files (sync walker is fast for directory traversal)
        let mut matches: Vec<PathBuf> = Vec::new();
        for entry in WalkBuilder::new(&path)
            .hidden(false)
            .git_ignore(true)
            .build()
            .flatten()
        {
            if entry.file_type().is_some_and(|ft| ft.is_file()) && matcher.is_match(entry.path()) {
                matches.push(entry.path().to_path_buf());
            }
        }

        // Fetch modification times asynchronously for all files
        let mut files_with_mtime: Vec<(PathBuf, SystemTime)> = Vec::with_capacity(matches.len());
        for path in matches {
            let mtime = Self::get_mtime(&path).await;
            files_with_mtime.push((path, mtime));
        }

        // Sort by modification time (newest first)
        files_with_mtime.sort_by(|a, b| b.1.cmp(&a.1));

        // Format output
        let lines: Vec<String> = files_with_mtime
            .iter()
            .map(|(p, _)| p.display().to_string())
            .collect();

        // Process and truncate output
        let output_lines = process_output_lines(&lines.join("\n"));
        let truncate_result = truncate_output(&output_lines, OutputLimits::MAX_OUTPUT_CHARS);

        let mut final_output = truncate_result.output;
        let was_truncated = truncate_result.char_truncated || truncate_result.remaining_count > 0;

        if was_truncated {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "files",
                truncate_result.char_truncated,
                OutputLimits::MAX_OUTPUT_CHARS,
            );
            final_output.push_str(&warning);
        }

        Ok(final_output)
    }
}
