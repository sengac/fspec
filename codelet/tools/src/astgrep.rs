//! AstGrepTool implementation using native ast-grep Rust crates
//!
//! Uses ast-grep-core and ast-grep-language for AST-based pattern matching,
//! and ignore crate for gitignore-aware file walking.

use crate::{
    error::ToolError,
    limits::OutputLimits,
    truncation::{format_truncation_warning, process_output_lines, truncate_output},
    ToolOutput,
};
use anyhow::Result;
use ast_grep_language::{LanguageExt, SupportLang};
use ignore::WalkBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// AstGrepTool for AST-based code pattern matching
pub struct AstGrepTool;

impl AstGrepTool {
    /// Create a new AstGrepTool
    pub fn new() -> Self {
        Self
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

    /// Search a single file for pattern matches (async, non-blocking file read)
    /// Returns (matches, was_skipped) - was_skipped is true if file couldn't be read or pattern failed
    async fn search_file(
        path: &Path,
        pattern: &str,
        lang: SupportLang,
    ) -> (Vec<MatchResult>, bool) {
        // Read file content (async, non-blocking)
        let source = match tokio::fs::read_to_string(path).await {
            Ok(content) => content,
            Err(_) => return (Vec::new(), true), // File was skipped due to read error
        };

        // Wrap AST operations in catch_unwind to prevent panics from crashing the runtime
        // ast-grep can panic on certain malformed patterns or edge cases
        let path_str = path.to_string_lossy().to_string();
        let pattern_owned = pattern.to_string();

        let search_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut results = Vec::new();
            let ast_grep = lang.ast_grep(&source);
            let root = ast_grep.root();
            let matches = root.find_all(pattern_owned.as_str());

            for node_match in matches {
                // Get position (0-based, convert to 1-based)
                let start_pos = node_match.start_pos();
                let line = start_pos.line() + 1; // Convert to 1-based
                let column = start_pos.column(&node_match) + 1; // Convert to 1-based
                let text = node_match.text().to_string();

                results.push(MatchResult {
                    file: path_str.clone(),
                    line,
                    column,
                    text,
                });
            }
            results
        }));

        match search_result {
            Ok(results) => (results, false),
            Err(_) => (Vec::new(), true), // Pattern matching panicked - skip this file
        }
    }

    /// Internal execute method for rig tool delegation
    async fn execute(&self, args: Value) -> Result<ToolOutput> {
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

        // Check for multi-line patterns - ast-grep doesn't support patterns spanning multiple statements
        if pattern.contains('\n') {
            return Ok(ToolOutput::error(format!(
                "Error: Multi-line patterns are not supported. Pattern: \"{}\"\n\n\
                ast-grep patterns must match a single AST node. Multi-line patterns \
                that span multiple statements (like struct with attributes) cannot be matched.\n\n\
                Suggestions:\n\
                - Search for just the struct: 'struct $NAME'\n\
                - Search for just the derive: '#[derive($_)]'\n\
                - Use grep for multi-line text searches\n\n\
                Try the ast-grep playground: https://ast-grep.github.io/playground.html",
                pattern.replace('\n', "\\n")
            )));
        }

        // Get valid extensions for this language
        let extensions = Self::get_extensions(lang);

        // Collect all file paths to search (sync walker is fast, respects gitignore)
        let mut files_to_search: Vec<std::path::PathBuf> = Vec::new();

        for search_path in &search_paths {
            let path = Path::new(search_path);

            // Check if path exists (async)
            match tokio::fs::try_exists(path).await {
                Ok(true) => {}
                Ok(false) | Err(_) => continue,
            }

            // Check if it's a file or directory (async)
            let metadata = match tokio::fs::metadata(path).await {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.is_file() {
                // Single file
                files_to_search.push(path.to_path_buf());
            } else {
                // Walk directory (sync walker is optimized for this)
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

                    files_to_search.push(file_path.to_path_buf());
                }
            }
        }

        // Search all files (async file reads)
        let mut all_matches: Vec<MatchResult> = Vec::new();
        let mut skipped_count: usize = 0;
        for file_path in &files_to_search {
            let (matches, was_skipped) = Self::search_file(file_path, pattern, lang).await;
            all_matches.extend(matches);
            if was_skipped {
                skipped_count += 1;
            }
        }

        // Format output
        if all_matches.is_empty() {
            let mut msg = "No matches found".to_string();
            if skipped_count > 0 {
                msg.push_str(&format!(
                    "\n(Note: {skipped_count} files skipped due to read errors)"
                ));
            }
            return Ok(ToolOutput::success(msg));
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

        // Report skipped files if any
        if skipped_count > 0 {
            final_output.push_str(&format!(
                "\n(Note: {skipped_count} files skipped due to read errors)"
            ));
        }

        Ok(ToolOutput {
            content: final_output,
            truncated: was_truncated,
            is_error: false,
        })
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

// rig::tool::Tool implementation

/// Arguments for AstGrep tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepArgs {
    /// The AST pattern to search for (must be valid syntax for the target language).
    /// Use $NAME for single-node wildcards, $$$ARGS for multi-node wildcards.
    /// Examples: 'fn $NAME($_)' for Rust functions, 'function $NAME($$$ARGS)' for JS functions.
    pub pattern: String,
    /// Programming language to search. Supported: rust, typescript, tsx, javascript, python,
    /// go, java, c, cpp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell.
    pub language: String,
    /// Directory or file to search in (optional, defaults to current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl rig::tool::Tool for AstGrepTool {
    const NAME: &'static str = "astgrep";

    type Error = ToolError;
    type Args = AstGrepArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "astgrep".to_string(),
            description: "AST-based code search that finds code by syntax structure, not text. \
                Far fewer false positives than grep for structural searches.\n\n\
                PATTERN SYNTAX:\n\
                - Pattern must be valid syntax in the target language\n\
                - Use $NAME for single-node wildcard (matches one AST node)\n\
                - Use $$$ARGS for multi-node wildcard (matches zero or more nodes)\n\n\
                EXAMPLES:\n\
                - Rust functions: pattern='fn $NAME($_)' language='rust'\n\
                - Rust structs: pattern='struct $NAME' language='rust'\n\
                - JS functions: pattern='function $NAME($$$ARGS)' language='javascript'\n\
                - TS async: pattern='async function $NAME($$$ARGS)' language='typescript'\n\
                - Python def: pattern='def $NAME($$$ARGS):' language='python'\n\
                - Method calls: pattern='$OBJ.$METHOD($$$ARGS)' language='typescript'\n\n\
                Returns file:line:column:matched_text format."
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
        value_map.insert(
            "language".to_string(),
            serde_json::Value::String(args.language),
        );
        if let Some(path) = args.path {
            value_map.insert("path".to_string(), serde_json::Value::String(path));
        }

        let result = self
            .execute(serde_json::Value::Object(value_map))
            .await
            .map_err(|e| ToolError::Execution {
                tool: "astgrep",
                message: e.to_string(),
            })?;

        if result.is_error {
            Err(ToolError::Execution {
                tool: "astgrep",
                message: result.content,
            })
        } else {
            Ok(result.content)
        }
    }
}
