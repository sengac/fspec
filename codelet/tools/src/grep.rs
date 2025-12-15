//! GrepTool implementation using ripgrep's grep crates
//!
//! Uses grep-regex and grep-searcher for content search,
//! and ignore crate for gitignore-aware file walking.

use crate::{
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
    Tool, ToolOutput, ToolParameters,
};
use anyhow::Result;
use async_trait::async_trait;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::{SearcherBuilder, Sink, SinkContext, SinkContextKind, SinkMatch};
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tracing::debug;

/// Output mode for grep results
#[derive(Debug, Clone, Copy, PartialEq)]
enum OutputMode {
    /// Return only file paths of matching files (default)
    FilesWithMatches,
    /// Return matching lines with file:line:content format
    Content,
    /// Return match counts per file in file:count format
    Count,
}

impl OutputMode {
    fn from_str(s: &str) -> Self {
        match s {
            "content" => Self::Content,
            "count" => Self::Count,
            _ => Self::FilesWithMatches,
        }
    }
}

/// GrepTool for content search using ripgrep crates
pub struct GrepTool {
    parameters: ToolParameters,
}

impl GrepTool {
    /// Create a new GrepTool
    pub fn new() -> Self {
        let mut properties = serde_json::Map::new();

        properties.insert(
            "pattern".to_string(),
            json!({
                "type": "string",
                "description": "The regex pattern to search for"
            }),
        );

        properties.insert(
            "path".to_string(),
            json!({
                "type": "string",
                "description": "Directory or file to search in (default: current directory)"
            }),
        );

        properties.insert(
            "output_mode".to_string(),
            json!({
                "type": "string",
                "enum": ["files_with_matches", "content", "count"],
                "description": "Output format: files_with_matches (default), content (with line numbers), count (match counts per file)"
            }),
        );

        properties.insert(
            "-i".to_string(),
            json!({
                "type": "boolean",
                "description": "Case-insensitive search"
            }),
        );

        properties.insert(
            "multiline".to_string(),
            json!({
                "type": "boolean",
                "description": "Enable multiline mode where patterns can span lines"
            }),
        );

        properties.insert(
            "-A".to_string(),
            json!({
                "type": "integer",
                "description": "Number of lines to show after each match (content mode only)"
            }),
        );

        properties.insert(
            "-B".to_string(),
            json!({
                "type": "integer",
                "description": "Number of lines to show before each match (content mode only)"
            }),
        );

        properties.insert(
            "-C".to_string(),
            json!({
                "type": "integer",
                "description": "Number of lines to show before and after each match (content mode only)"
            }),
        );

        properties.insert(
            "glob".to_string(),
            json!({
                "type": "string",
                "description": "Glob pattern to filter files (e.g., '*.ts', '*.{js,ts}')"
            }),
        );

        properties.insert(
            "type".to_string(),
            json!({
                "type": "string",
                "description": "File type filter (e.g., 'rust', 'js', 'py')"
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

    /// Build the regex matcher with options
    fn build_matcher(
        pattern: &str,
        case_insensitive: bool,
        multiline: bool,
    ) -> Result<grep_regex::RegexMatcher, ToolOutput> {
        let mut builder = RegexMatcherBuilder::new();
        builder.case_insensitive(case_insensitive);

        if multiline {
            builder.multi_line(true);
            builder.dot_matches_new_line(true);
        }

        builder
            .build(pattern)
            .map_err(|e| ToolOutput::error(format!("Error: Invalid regex pattern: {e}")))
    }

    /// Search a single file and collect matches
    fn search_file(
        &self,
        path: &Path,
        matcher: &grep_regex::RegexMatcher,
        output_mode: OutputMode,
        context_before: usize,
        context_after: usize,
        multiline: bool,
    ) -> Result<FileSearchResult, ToolOutput> {
        let mut result = FileSearchResult {
            path: path.to_string_lossy().to_string(),
            matches: Vec::new(),
            count: 0,
        };

        let mut searcher = SearcherBuilder::new()
            .before_context(context_before)
            .after_context(context_after)
            .line_number(true)
            .multi_line(multiline)
            .build();

        // Custom sink that captures both matches and context
        struct ContentSink<'a> {
            result: &'a mut FileSearchResult,
            output_mode: OutputMode,
        }

        impl Sink for ContentSink<'_> {
            type Error = std::io::Error;

            fn matched(
                &mut self,
                _searcher: &grep_searcher::Searcher,
                mat: &SinkMatch<'_>,
            ) -> std::result::Result<bool, Self::Error> {
                self.result.count += 1;
                if self.output_mode == OutputMode::Content {
                    if let Ok(line) = std::str::from_utf8(mat.bytes()) {
                        self.result.matches.push(MatchLine {
                            line_num: mat.line_number().unwrap_or(0),
                            content: line.trim_end().to_string(),
                            is_context: false,
                        });
                    }
                }
                Ok(true)
            }

            fn context(
                &mut self,
                _searcher: &grep_searcher::Searcher,
                ctx: &SinkContext<'_>,
            ) -> std::result::Result<bool, Self::Error> {
                if self.output_mode == OutputMode::Content {
                    // Only capture before/after context, not other types
                    if matches!(ctx.kind(), SinkContextKind::Before | SinkContextKind::After) {
                        if let Ok(line) = std::str::from_utf8(ctx.bytes()) {
                            self.result.matches.push(MatchLine {
                                line_num: ctx.line_number().unwrap_or(0),
                                content: line.trim_end().to_string(),
                                is_context: true,
                            });
                        }
                    }
                }
                Ok(true)
            }
        }

        let mut sink = ContentSink {
            result: &mut result,
            output_mode,
        };

        let search_result = searcher.search_path(matcher, path, &mut sink);

        match search_result {
            Ok(()) => Ok(result),
            Err(_) => {
                // Binary files or read errors - skip silently
                Ok(result)
            }
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Search result for a single file
struct FileSearchResult {
    path: String,
    matches: Vec<MatchLine>,
    count: u64,
}

/// A single matching line
struct MatchLine {
    line_num: u64,
    content: String,
    #[allow(dead_code)] // Reserved for future context line formatting
    is_context: bool,
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        "Search file contents using regex patterns. Uses ripgrep crates for fast, gitignore-aware searching."
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        debug!(tool = "Grep", input = ?args, "Executing tool");
        // Extract parameters
        let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("");

        if pattern.is_empty() {
            return Ok(ToolOutput::error(
                "Error: pattern parameter is required".to_string(),
            ));
        }

        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        let output_mode = args
            .get("output_mode")
            .and_then(|v| v.as_str())
            .map(OutputMode::from_str)
            .unwrap_or(OutputMode::FilesWithMatches);

        let case_insensitive = args.get("-i").and_then(serde_json::Value::as_bool).unwrap_or(false);

        let multiline = args
            .get("multiline")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        // Context lines (only used in content mode)
        let context_c = args
            .get("-C")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(0);
        let context_before = args
            .get("-B")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(context_c);
        let context_after = args
            .get("-A")
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(context_c);

        let glob_pattern = args.get("glob").and_then(|v| v.as_str());
        let file_type = args.get("type").and_then(|v| v.as_str());

        // Build matcher
        let matcher = match Self::build_matcher(pattern, case_insensitive, multiline) {
            Ok(m) => m,
            Err(e) => return Ok(e),
        };

        // Build walker with gitignore support
        let mut walker_builder = WalkBuilder::new(path);
        walker_builder.hidden(false).git_ignore(true);

        // Add glob filter if specified
        if let Some(glob) = glob_pattern {
            let glob_matcher = globset::GlobBuilder::new(glob)
                .literal_separator(true)
                .build()
                .map(|g| g.compile_matcher());

            if let Ok(glob_matcher) = glob_matcher {
                walker_builder.filter_entry(move |entry| {
                    if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                        return true; // Always descend into directories
                    }
                    // Match against full path or filename
                    let matches_full = glob_matcher.is_match(entry.path());
                    let matches_name = entry
                        .path()
                        .file_name()
                        .map(|n| glob_matcher.is_match(n))
                        .unwrap_or(false);
                    matches_full || matches_name
                });
            }
        }

        // Add type filter if specified
        if let Some(file_type) = file_type {
            let mut types_builder = ignore::types::TypesBuilder::new();
            types_builder.add_defaults();
            types_builder.select(file_type);
            if let Ok(types) = types_builder.build() {
                walker_builder.types(types);
            }
        }

        let walker = walker_builder.build();

        // Collect results
        let mut results: Vec<FileSearchResult> = Vec::new();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                continue;
            }

            let file_path = entry.path();
            if let Ok(result) = self.search_file(
                file_path,
                &matcher,
                output_mode,
                context_before,
                context_after,
                multiline,
            ) {
                if result.count > 0 {
                    results.push(result);
                }
            }
        }

        // Format output based on mode
        if results.is_empty() {
            return Ok(ToolOutput::success("No matches found".to_string()));
        }

        let output = match output_mode {
            OutputMode::FilesWithMatches => results
                .iter()
                .map(|r| r.path.clone())
                .collect::<Vec<_>>()
                .join("\n"),
            OutputMode::Content => {
                let mut lines = Vec::new();
                for result in &results {
                    for m in &result.matches {
                        lines.push(format!("{}:{}:{}", result.path, m.line_num, m.content));
                    }
                }
                lines.join("\n")
            }
            OutputMode::Count => results
                .iter()
                .map(|r| format!("{}:{}", r.path, r.count))
                .collect::<Vec<_>>()
                .join("\n"),
        };

        // Apply truncation
        let lines = process_output_lines(&output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

        let mut final_output = truncate_result.output;
        let was_truncated = truncate_result.char_truncated || truncate_result.remaining_count > 0;

        if was_truncated {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "lines",
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

/// Arguments for Grep tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct GrepArgs {
    /// The regex pattern to search for
    pub pattern: String,
    /// Directory or file to search in (optional, defaults to current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Output mode: files_with_matches, content, or count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<String>,
}

/// Error type for Grep tool
#[derive(Debug, thiserror::Error)]
pub enum GrepError {
    #[error("Pattern error: {0}")]
    PatternError(String),
    #[error("Search error: {0}")]
    SearchError(String),
}

impl rig::tool::Tool for GrepTool {
    const NAME: &'static str = "grep";

    type Error = GrepError;
    type Args = GrepArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "grep".to_string(),
            description: "Search for regex pattern in files. Supports multiple output modes and respects .gitignore.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(GrepArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Delegate to existing execute method by converting args to Value
        let mut value_map = serde_json::Map::new();
        value_map.insert(
            "pattern".to_string(),
            serde_json::Value::String(args.pattern),
        );
        if let Some(path) = args.path {
            value_map.insert("path".to_string(), serde_json::Value::String(path));
        }
        if let Some(mode) = args.output_mode {
            value_map.insert("output_mode".to_string(), serde_json::Value::String(mode));
        }

        let result = self
            .execute(serde_json::Value::Object(value_map))
            .await
            .map_err(|e| GrepError::SearchError(e.to_string()))?;

        if result.is_error {
            Err(GrepError::SearchError(result.content))
        } else {
            Ok(result.content)
        }
    }
}
