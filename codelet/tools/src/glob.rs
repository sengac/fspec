//! GlobTool implementation using ignore crate
//!
//! Uses ignore crate for gitignore-aware file walking
//! and globset for glob pattern matching.

use crate::{
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
};
use globset::GlobBuilder;
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// GlobTool for gitignore-aware file pattern matching
pub struct GlobTool;

impl GlobTool {
    /// Create a new GlobTool
    pub fn new() -> Self {
        Self
    }

    /// Get file modification time for sorting
    fn get_mtime(path: &PathBuf) -> SystemTime {
        fs::metadata(path)
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

        if !path.exists() {
            return Err(GlobError::PathError(format!(
                "Path does not exist: {search_path}"
            )));
        }

        // Collect matching files
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

        // Sort by modification time (newest first)
        matches.sort_by(|a, b| {
            let a_time = GlobTool::get_mtime(a);
            let b_time = GlobTool::get_mtime(b);
            b_time.cmp(&a_time)
        });

        // Format output
        let lines: Vec<String> = matches.iter().map(|p| p.display().to_string()).collect();

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
