//! AstGrepTool implementation using native ast-grep Rust crates
//!
//! Uses ast-grep-core and ast-grep-language for AST-based pattern matching,
//! and ignore crate for gitignore-aware file walking.

use crate::{
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
    Tool, ToolOutput, ToolParameters,
};
use anyhow::Result;
use ast_grep_language::{LanguageExt, SupportLang};
use async_trait::async_trait;
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use tracing::debug;

/// AstGrepTool for AST-based code pattern matching
pub struct AstGrepTool {
    parameters: ToolParameters,
}

impl AstGrepTool {
    /// Create a new AstGrepTool
    pub fn new() -> Self {
        let mut properties = serde_json::Map::new();

        properties.insert(
            "pattern".to_string(),
            json!({
                "type": "string",
                "description": "AST pattern to search for. Use $VAR for single node capture, $$$VAR for multiple nodes."
            }),
        );

        properties.insert(
            "language".to_string(),
            json!({
                "type": "string",
                "description": "Programming language (typescript, javascript, rust, python, go, java, c, cpp, etc.)"
            }),
        );

        properties.insert(
            "path".to_string(),
            json!({
                "type": "string",
                "description": "Directory to search in (default: current directory)"
            }),
        );

        properties.insert(
            "paths".to_string(),
            json!({
                "type": "array",
                "items": { "type": "string" },
                "description": "List of specific paths to search"
            }),
        );

        Self {
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties,
                required: vec!["pattern".to_string(), "language".to_string()],
            },
        }
    }

    /// Parse language string to SupportLang
    fn parse_language(lang: &str) -> Option<SupportLang> {
        // Try parsing the language string
        lang.to_lowercase().parse::<SupportLang>().ok()
    }

    /// Get file extensions for a language
    fn get_extensions(lang: SupportLang) -> Vec<&'static str> {
        match lang {
            SupportLang::TypeScript => vec!["ts"],
            SupportLang::Tsx => vec!["tsx"],
            SupportLang::JavaScript => vec!["js", "mjs", "cjs"],
            SupportLang::Rust => vec!["rs"],
            SupportLang::Python => vec!["py"],
            SupportLang::Go => vec!["go"],
            SupportLang::Java => vec!["java"],
            SupportLang::C => vec!["c", "h"],
            SupportLang::Cpp => vec!["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
            SupportLang::CSharp => vec!["cs"],
            SupportLang::Ruby => vec!["rb"],
            SupportLang::Kotlin => vec!["kt", "kts"],
            SupportLang::Swift => vec!["swift"],
            SupportLang::Scala => vec!["scala"],
            SupportLang::Php => vec!["php"],
            SupportLang::Bash => vec!["sh", "bash"],
            SupportLang::Html => vec!["html", "htm"],
            SupportLang::Css => vec!["css"],
            SupportLang::Json => vec!["json"],
            SupportLang::Yaml => vec!["yaml", "yml"],
            SupportLang::Lua => vec!["lua"],
            SupportLang::Elixir => vec!["ex", "exs"],
            SupportLang::Haskell => vec!["hs"],
            _ => vec![],
        }
    }

    /// Search a single file for pattern matches
    fn search_file(&self, path: &Path, pattern: &str, lang: SupportLang) -> Vec<MatchResult> {
        let mut results = Vec::new();

        // Read file content
        let source = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return results, // Skip files that can't be read
        };

        // Create AST and search for pattern
        let ast_grep = lang.ast_grep(&source);
        let root = ast_grep.root();
        let matches = root.find_all(pattern);

        for node_match in matches {
            // Get position (0-based, convert to 1-based)
            let start_pos = node_match.start_pos();
            let line = start_pos.line() + 1; // Convert to 1-based
            let column = start_pos.column(&node_match) + 1; // Convert to 1-based
            let text = node_match.text().to_string();

            results.push(MatchResult {
                file: path.to_string_lossy().to_string(),
                line,
                column,
                text,
            });
        }

        results
    }
}

impl Default for AstGrepTool {
    fn default() -> Self {
        Self::new()
    }
}

/// A single match result
struct MatchResult {
    file: String,
    line: usize,
    column: usize,
    text: String,
}

#[async_trait]
impl Tool for AstGrepTool {
    fn name(&self) -> &str {
        "AstGrep"
    }

    fn description(&self) -> &str {
        "Search code using AST-based pattern matching. Uses meta-variables ($VAR, $$$VAR) for structural patterns."
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        debug!(tool = "AstGrep", input = ?args, "Executing tool");
        // Extract required parameters
        let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => p,
            _ => {
                return Ok(ToolOutput::error(
                    "Error: pattern parameter is required".to_string(),
                ))
            }
        };

        let language_str = match args.get("language").and_then(|v| v.as_str()) {
            Some(l) if !l.is_empty() => l,
            _ => {
                return Ok(ToolOutput::error(
                    "Error: language parameter is required".to_string(),
                ))
            }
        };

        // Parse language
        let lang = match Self::parse_language(language_str) {
            Some(l) => l,
            None => {
                return Ok(ToolOutput::error(format!(
                    "Error: Unsupported language '{language_str}'. Supported languages include: typescript, javascript, rust, python, go, java, c, cpp, ruby, kotlin, swift, etc."
                )))
            }
        };

        // Get search paths
        let search_paths: Vec<String> = if let Some(paths) = args.get("paths") {
            if let Some(arr) = paths.as_array() {
                arr.iter()
                    .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                    .collect()
            } else {
                vec![]
            }
        } else if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            vec![path.to_string()]
        } else {
            vec![".".to_string()]
        };

        // Check for obviously invalid patterns (malformed braces)
        if pattern.contains("{{{") || pattern.contains("}}}") {
            return Ok(ToolOutput::error(format!(
                "Error: Invalid AST pattern syntax. Pattern: \"{pattern}\"\n\n\
                The pattern could not be parsed as valid {language_str} syntax.\n\n\
                Pattern syntax guide:\n\
                - Use $VAR for single AST node (e.g., 'function $NAME()')\n\
                - Use $$$VAR for multiple nodes (e.g., 'function $NAME($$$ARGS)')\n\
                - Pattern must be valid {language_str} syntax\n\
                - Meta-variables must start with $ and uppercase letter\n\n\
                Try the ast-grep playground: https://ast-grep.github.io/playground.html"
            )));
        }

        // Get valid extensions for this language
        let extensions = Self::get_extensions(lang);

        // Collect all matches
        let mut all_matches: Vec<MatchResult> = Vec::new();

        for search_path in &search_paths {
            let path = Path::new(search_path);
            if !path.exists() {
                continue;
            }

            if path.is_file() {
                // Search single file
                let matches = self.search_file(path, pattern, lang);
                all_matches.extend(matches);
            } else {
                // Walk directory
                let walker = WalkBuilder::new(path)
                    .hidden(false)
                    .git_ignore(true)
                    .build();

                for entry in walker.flatten() {
                    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                        continue;
                    }

                    let file_path = entry.path();

                    // Check extension
                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

                    if !extensions.contains(&ext) {
                        continue;
                    }

                    let matches = self.search_file(file_path, pattern, lang);
                    all_matches.extend(matches);
                }
            }
        }

        // Format output
        if all_matches.is_empty() {
            return Ok(ToolOutput::success("No matches found".to_string()));
        }

        // Format as file:line:column:text
        let output_lines: Vec<String> = all_matches
            .iter()
            .map(|m| {
                // Take first line of match text for display
                let first_line = m.text.lines().next().unwrap_or("");
                format!("{}:{}:{}:{}", m.file, m.line, m.column, first_line)
            })
            .collect();

        let output = output_lines.join("\n");

        // Apply truncation
        let lines = process_output_lines(&output);
        let truncate_result = truncate_output(&lines, OutputLimits::MAX_OUTPUT_CHARS);

        let mut final_output = truncate_result.output;
        let was_truncated = truncate_result.char_truncated || truncate_result.remaining_count > 0;

        if was_truncated {
            let warning = format_truncation_warning(
                truncate_result.remaining_count,
                "matches",
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

/// Arguments for AstGrep tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepArgs {
    /// The AST pattern to search for
    pub pattern: String,
    /// Language to parse (e.g., "rust", "typescript", "python")
    pub lang: String,
    /// Directory or file to search in (optional, defaults to current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Error type for AstGrep tool
#[derive(Debug, thiserror::Error)]
pub enum AstGrepError {
    #[error("Language error: {0}")]
    LanguageError(String),
    #[error("Pattern error: {0}")]
    PatternError(String),
    #[error("Search error: {0}")]
    SearchError(String),
}

impl rig::tool::Tool for AstGrepTool {
    const NAME: &'static str = "astgrep";

    type Error = AstGrepError;
    type Args = AstGrepArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "astgrep".to_string(),
            description:
                "Search for code patterns using AST-based matching. Supports 27+ languages."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AstGrepArgs))
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
        value_map.insert("lang".to_string(), serde_json::Value::String(args.lang));
        if let Some(path) = args.path {
            value_map.insert("path".to_string(), serde_json::Value::String(path));
        }

        let result = self
            .execute(serde_json::Value::Object(value_map))
            .await
            .map_err(|e| AstGrepError::SearchError(e.to_string()))?;

        if result.is_error {
            Err(AstGrepError::SearchError(result.content))
        } else {
            Ok(result.content)
        }
    }
}
