//! GlobTool implementation using ignore crate
//!
//! Uses ignore crate for gitignore-aware file walking
//! and globset for glob pattern matching.

use crate::{
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
    Tool, ToolOutput, ToolParameters,
};
use anyhow::Result;
use async_trait::async_trait;
use globset::GlobBuilder;
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tracing::debug;

/// GlobTool for gitignore-aware file pattern matching
pub struct GlobTool {
    parameters: ToolParameters,
}

impl GlobTool {
    /// Create a new GlobTool
    pub fn new() -> Self {
        let mut properties = serde_json::Map::new();

        properties.insert(
            "pattern".to_string(),
            json!({
                "type": "string",
                "description": "The glob pattern to match files (e.g., '**/*.ts', '*.{js,ts}')"
            }),
        );

        properties.insert(
            "path".to_string(),
            json!({
                "type": "string",
                "description": "Directory to search in (default: current directory)"
            }),
        );

        Self {
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties,
                required: vec!["pattern".to_string()],
            },
        }
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

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "Find files by glob pattern. Respects .gitignore by default. Returns matching file paths sorted by modification time (newest first)."
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        debug!(tool = "Glob", input = ?args, "Executing tool");
        // Extract parameters
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");

        if pattern.is_empty() {
            return Ok(ToolOutput::error(
                "Error: pattern parameter is required".to_string(),
            ));
        }

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        // Build glob matcher
        let glob_matcher = match GlobBuilder::new(pattern).literal_separator(true).build() {
            Ok(g) => g.compile_matcher(),
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Error: Invalid glob pattern: {e}"
                )));
            }
        };

        // Build walker with gitignore support
        let walker = WalkBuilder::new(path)
            .hidden(false)
            .git_ignore(true)
            .build();

        // Collect matching files
        let mut files: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path();

            // Match against the pattern - try both full path and relative path
            let matches = glob_matcher.is_match(file_path)
                || file_path
                    .strip_prefix(path)
                    .map(|p| glob_matcher.is_match(p))
                    .unwrap_or(false)
                || file_path
                    .file_name()
                    .map(|n| glob_matcher.is_match(n))
                    .unwrap_or(false);

            if matches {
                let mtime = Self::get_mtime(&file_path.to_path_buf());
                files.push((file_path.to_path_buf(), mtime));
            }
        }

        // Sort by modification time (newest first)
        files.sort_by(|a, b| b.1.cmp(&a.1));

        // Format output
        if files.is_empty() {
            return Ok(ToolOutput::success("No matches found".to_string()));
        }

        let output: String = files
            .iter()
            .map(|(p, _)| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        // Apply truncation
        let lines = process_output_lines(&output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

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

        Ok(ToolOutput {
            content: final_output,
            truncated: was_truncated,
            is_error: false,
        })
    }
}

// ========================================
// Rig Tool Implementation (REFAC-004)
// ========================================

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
            description: "Find files by glob pattern. Respects .gitignore by default. Returns matching file paths sorted by modification time (newest first).".to_string(),
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
