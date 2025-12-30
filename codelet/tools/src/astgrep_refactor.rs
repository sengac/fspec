//! AstGrepRefactorTool implementation for AST-based code refactoring
//!
//! Provides two modes:
//! - Extract mode: Extract matched code to a target file
//! - Replace mode: Replace matched code in-place with replacement text
//!
//! Feature: spec/features/ast-code-refactor-tool-for-codelet.feature

use crate::{error::ToolError, ToolOutput};
use anyhow::Result;
use ast_grep_language::{LanguageExt, SupportLang};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// AstGrepRefactorTool for AST-based code refactoring
pub struct AstGrepRefactorTool;

impl AstGrepRefactorTool {
    /// Create a new AstGrepRefactorTool
    pub fn new() -> Self {
        Self
    }

    /// Parse language string to SupportLang
    fn parse_language(lang: &str) -> Option<SupportLang> {
        lang.to_lowercase().parse::<SupportLang>().ok()
    }

    /// Get list of supported languages for error messages
    fn supported_languages() -> &'static str {
        "typescript, tsx, javascript, rust, python, go, java, c, cpp, csharp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell"
    }

    /// Refactor operation - extract matched code to target file or replace in-place
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

        let source_file = match args.get("source_file").and_then(|v| v.as_str()) {
            Some(f) if !f.is_empty() => f,
            _ => {
                return Ok(ToolOutput::error(
                    "Error: source_file parameter is required".to_string(),
                ))
            }
        };

        let target_file = args.get("target_file").and_then(|v| v.as_str());
        let replacement = args.get("replacement").and_then(|v| v.as_str());

        // Validate: exactly one of target_file or replacement must be provided
        match (target_file, replacement) {
            (None, None) => {
                return Ok(ToolOutput::error(
                    "Error: Either target_file (for extract mode) or replacement (for replace mode) must be provided".to_string(),
                ))
            }
            (Some(_), Some(_)) => {
                return Ok(ToolOutput::error(
                    "Error: Cannot specify both target_file and replacement. Use one mode only.".to_string(),
                ))
            }
            _ => {}
        }

        // Parse language
        let lang = match Self::parse_language(language_str) {
            Some(l) => l,
            None => {
                return Ok(ToolOutput::error(format!(
                    "Error: Unsupported language '{}'. Supported languages: {}",
                    language_str,
                    Self::supported_languages()
                )))
            }
        };

        // Check source file exists
        let source_path = Path::new(source_file);
        if !source_path.exists() {
            return Ok(ToolOutput::error(format!(
                "Error: Source file not found: {}",
                source_file
            )));
        }

        // Read source file
        let source_content = match tokio::fs::read_to_string(source_path).await {
            Ok(content) => content,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to read source file: {}",
                    e
                )))
            }
        };

        // Find matches using ast-grep
        let pattern_owned = pattern.to_string();
        let source_for_search = source_content.clone();

        let search_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let ast_grep = lang.ast_grep(&source_for_search);
            let root = ast_grep.root();
            let matches: Vec<_> = root
                .find_all(pattern_owned.as_str())
                .map(|m| {
                    let start_pos = m.start_pos();
                    let end_pos = m.end_pos();
                    let text = m.text().to_string();
                    let start_byte = m.range().start;
                    let end_byte = m.range().end;
                    (
                        start_pos.line() + 1,
                        start_pos.column(&m) + 1,
                        end_pos.line() + 1,
                        text,
                        start_byte,
                        end_byte,
                    )
                })
                .collect();
            matches
        }));

        let matches = match search_result {
            Ok(m) => m,
            Err(_) => {
                return Ok(ToolOutput::error(format!(
                    "Error: Pattern matching failed. Pattern may be invalid for {} syntax: {}",
                    language_str, pattern
                )))
            }
        };

        // Validate exactly one match
        match matches.len() {
            0 => {
                return Ok(ToolOutput::error(format!(
                    "Error: No matches found for pattern '{}' in {}",
                    pattern, source_file
                )))
            }
            1 => {} // Exactly one match - proceed
            n => {
                let locations: Vec<String> = matches
                    .iter()
                    .map(|(line, col, _, text, _, _)| {
                        let first_line = text.lines().next().unwrap_or("");
                        format!("  {}:{}:{}: {}", source_file, line, col, first_line)
                    })
                    .collect();
                return Ok(ToolOutput::error(format!(
                    "Error: Multiple matches found ({count}). Refactor requires exactly one match:\n{locations}",
                    count = n,
                    locations = locations.join("\n")
                )));
            }
        }

        let (line, column, _end_line, matched_text, start_byte, end_byte) = &matches[0];

        if let Some(target) = target_file {
            // Extract mode: Move code to target file
            let target_path = Path::new(target);

            // Read existing target content (for append mode)
            let existing_content = if target_path.exists() {
                match tokio::fs::read_to_string(target_path).await {
                    Ok(content) => content,
                    Err(e) => {
                        return Ok(ToolOutput::error(format!(
                            "Error: Failed to read target file: {}",
                            e
                        )))
                    }
                }
            } else {
                String::new()
            };

            // Remove matched code from source
            let mut new_source = source_content.clone();
            new_source.replace_range(*start_byte..*end_byte, "");

            // Clean up blank lines (collapse multiple consecutive newlines)
            new_source = Self::cleanup_blank_lines(&new_source);

            // Write updated source file
            if let Err(e) = tokio::fs::write(source_path, &new_source).await {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to write source file: {}",
                    e
                )));
            }

            // Append to target file
            let new_target = if existing_content.is_empty() {
                matched_text.clone()
            } else {
                format!("{}\n\n{}", existing_content.trim_end(), matched_text)
            };

            if let Err(e) = tokio::fs::write(target_path, &new_target).await {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to write target file: {}",
                    e
                )));
            }

            // Return success result
            let result = json!({
                "success": true,
                "mode": "extract",
                "moved_code": matched_text,
                "source_file": source_file,
                "target_file": target,
                "match_location": {
                    "line": line,
                    "column": column
                }
            });

            Ok(ToolOutput::success(result.to_string()))
        } else if let Some(repl) = replacement {
            // Replace mode: Replace code in-place
            let mut new_source = source_content.clone();
            new_source.replace_range(*start_byte..*end_byte, repl);

            // Write updated source file
            if let Err(e) = tokio::fs::write(source_path, &new_source).await {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to write source file: {}",
                    e
                )));
            }

            // Return success result
            let result = json!({
                "success": true,
                "mode": "replace",
                "original_code": matched_text,
                "replacement_code": repl,
                "source_file": source_file,
                "match_location": {
                    "line": line,
                    "column": column
                }
            });

            Ok(ToolOutput::success(result.to_string()))
        } else {
            unreachable!("Already validated that one of target_file or replacement is set")
        }
    }

    /// Clean up consecutive blank lines, collapsing to single blank line
    fn cleanup_blank_lines(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut prev_was_blank = false;

        for line in lines {
            let is_blank = line.trim().is_empty();
            if is_blank {
                if !prev_was_blank {
                    result.push("");
                }
                prev_was_blank = true;
            } else {
                result.push(line);
                prev_was_blank = false;
            }
        }

        // Join with newlines and ensure trailing newline
        let mut output = result.join("\n");
        if !output.is_empty() && !output.ends_with('\n') {
            output.push('\n');
        }
        output
    }
}

impl Default for AstGrepRefactorTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

/// Arguments for AstGrepRefactor tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepRefactorArgs {
    /// The AST pattern to match (must be valid syntax for the target language).
    /// Use $NAME for single-node wildcards, $$$ARGS for multi-node wildcards.
    /// Pattern must match exactly ONE node in the source file.
    pub pattern: String,
    /// Programming language. Supported: rust, typescript, tsx, javascript, python,
    /// go, java, c, cpp, ruby, kotlin, swift, scala, php, bash, html, css, json, yaml, lua, elixir, haskell.
    pub language: String,
    /// Path to the source file to refactor
    pub source_file: String,
    /// Path to target file for extraction (mutually exclusive with replacement).
    /// If file exists, matched code will be APPENDED.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_file: Option<String>,
    /// Replacement text for in-place replacement (mutually exclusive with target_file).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// Result of a refactor operation
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepRefactorResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The mode used: "extract" or "replace"
    pub mode: String,
    /// The code that was moved (extract mode) or replaced (replace mode)
    pub moved_code: Option<String>,
    /// The original code (replace mode only)
    pub original_code: Option<String>,
    /// The replacement code (replace mode only)
    pub replacement_code: Option<String>,
    /// Source file path
    pub source_file: String,
    /// Target file path (extract mode only)
    pub target_file: Option<String>,
}

impl rig::tool::Tool for AstGrepRefactorTool {
    const NAME: &'static str = "astgrep_refactor";

    type Error = ToolError;
    type Args = AstGrepRefactorArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "astgrep_refactor".to_string(),
            description: "AST-based code refactoring tool. Finds code by AST pattern and either \
                extracts it to a new file (extract mode) or replaces it in-place (replace mode).\n\n\
                MODES:\n\
                - Extract: Set target_file to move matched code to a new file (appends if exists)\n\
                - Replace: Set replacement to replace matched code in-place\n\n\
                CONSTRAINTS:\n\
                - Pattern must match EXACTLY ONE node (errors if 0 or multiple matches)\n\
                - Must specify either target_file OR replacement, not both\n\n\
                PATTERN SYNTAX:\n\
                - Use $NAME for single-node wildcard\n\
                - Use $$$ARGS for multi-node wildcard\n\n\
                EXAMPLES:\n\
                - Extract function: pattern='fn $NAME($$$ARGS) { $$$BODY }' target_file='extracted.rs'\n\
                - Replace function: pattern='fn old() { $$$BODY }' replacement='fn new() { println!(\"new\"); }'"
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(AstGrepRefactorArgs))
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
        value_map.insert(
            "source_file".to_string(),
            serde_json::Value::String(args.source_file),
        );
        if let Some(target) = args.target_file {
            value_map.insert("target_file".to_string(), serde_json::Value::String(target));
        }
        if let Some(repl) = args.replacement {
            value_map.insert("replacement".to_string(), serde_json::Value::String(repl));
        }

        let result = self
            .execute(serde_json::Value::Object(value_map))
            .await
            .map_err(|e| ToolError::Execution {
                tool: "astgrep_refactor",
                message: e.to_string(),
            })?;

        if result.is_error {
            Err(ToolError::Execution {
                tool: "astgrep_refactor",
                message: result.content,
            })
        } else {
            Ok(result.content)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_blank_lines() {
        let input = "line1\n\n\n\nline2\n\nline3";
        let result = AstGrepRefactorTool::cleanup_blank_lines(input);
        assert!(!result.contains("\n\n\n"), "Should not have 3+ consecutive newlines");
        assert!(result.contains("line1"), "Should preserve content");
        assert!(result.contains("line2"), "Should preserve content");
        assert!(result.contains("line3"), "Should preserve content");
    }

    #[test]
    fn test_parse_language() {
        assert!(AstGrepRefactorTool::parse_language("rust").is_some());
        assert!(AstGrepRefactorTool::parse_language("typescript").is_some());
        assert!(AstGrepRefactorTool::parse_language("invalid").is_none());
    }
}
