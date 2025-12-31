//! AstGrepRefactorTool implementation for AST-based code refactoring
//!
//! Provides two modes:
//! - Extract mode: Extract matched code to a target file
//! - Replace mode: Replace matched code in-place with replacement text
//!
//! Feature: spec/features/ast-code-refactor-tool-for-codelet.feature

use crate::{error::ToolError, ToolOutput};
use anyhow::Result;
use ast_grep_core::meta_var::MetaVariable;
use ast_grep_language::{LanguageExt, SupportLang};
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Case conversion types for Convert transform
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CaseType {
    /// All lowercase
    LowerCase,
    /// ALL UPPERCASE
    UpperCase,
    /// First letter uppercase
    Capitalize,
    /// firstWordLower
    CamelCase,
    /// words_with_underscores
    SnakeCase,
    /// words-with-dashes
    KebabCase,
    /// FirstWordUpper
    PascalCase,
}

/// Separator options for word splitting in case conversion
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Separator {
    /// Split on case transitions (camelCase → camel, Case)
    CaseChange,
    /// Split on underscore
    Underscore,
    /// Split on dash
    Dash,
    /// Split on dot
    Dot,
    /// Split on slash
    Slash,
    /// Split on space
    Space,
}

/// Substring transform - extract portion of captured text
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubstringTransform {
    /// Source variable (e.g., "$NAME")
    pub source: String,
    /// Start character index (0-based, negative counts from end)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_char: Option<i32>,
    /// End character index (negative counts from end)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_char: Option<i32>,
}

/// Replace transform - regex find/replace on captured text
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceTransform {
    /// Source variable (e.g., "$NAME")
    pub source: String,
    /// Regex pattern to find
    pub replace: String,
    /// Replacement string
    pub by: String,
}

/// Convert transform - case conversion
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConvertTransform {
    /// Source variable (e.g., "$NAME")
    pub source: String,
    /// Target case type
    pub to_case: CaseType,
    /// Optional separators for word splitting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub separated_by: Option<Vec<Separator>>,
}

/// Transform definition - one of substring, replace, or convert
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Transform {
    /// Extract substring from captured variable
    Substring(SubstringTransform),
    /// Regex find/replace on captured variable
    Replace(ReplaceTransform),
    /// Case conversion on captured variable
    Convert(ConvertTransform),
}

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

        // Extract new optional parameters
        let transforms: Option<HashMap<String, Transform>> = args
            .get("transforms")
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let batch = args
            .get("batch")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let preview = args
            .get("preview")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        // Validate: exactly one of target_file or replacement must be provided
        let is_extract_mode = target_file.is_some();
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

        // Validate extract mode restrictions
        if is_extract_mode {
            if transforms.is_some() {
                return Ok(ToolOutput::error(
                    "Error: Transforms are not supported in extract mode. Use replace mode for transforms.".to_string(),
                ));
            }
            if batch {
                return Ok(ToolOutput::error(
                    "Error: Batch mode is not supported in extract mode. Use replace mode for batch operations.".to_string(),
                ));
            }
        }

        // Validate transforms if provided (check for cyclic dependencies and invalid regex early)
        if let Some(ref t) = transforms {
            if let Err(e) = Self::detect_cyclic_dependencies(t) {
                return Ok(ToolOutput::error(format!("Error: {e}")));
            }
            // Validate regex patterns early
            for (name, transform) in t {
                if let Transform::Replace(ref r) = transform {
                    if let Err(e) = Regex::new(&r.replace) {
                        return Ok(ToolOutput::error(format!(
                            "Error: Invalid regex in transform '{name}': {e}"
                        )));
                    }
                }
            }
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
                "Error: Source file not found: {source_file}"
            )));
        }

        // Read source file
        let source_content = match tokio::fs::read_to_string(source_path).await {
            Ok(content) => content,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to read source file: {e}"
                )))
            }
        };

        // Find matches using ast-grep
        let pattern_owned = pattern.to_string();
        let source_for_search = source_content.clone();

        // Struct to hold match data including captured meta-variables
        struct MatchData {
            line: usize,
            column: usize,
            text: String,
            start_byte: usize,
            end_byte: usize,
            variables: HashMap<String, String>,
        }

        let search_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let ast_grep = lang.ast_grep(&source_for_search);
            let root = ast_grep.root();
            let matches: Vec<_> = root
                .find_all(pattern_owned.as_str())
                .map(|m| {
                    let start_pos = m.start_pos();
                    let text = m.text().to_string();
                    let start_byte = m.range().start;
                    let end_byte = m.range().end;

                    // Extract ALL captured meta-variables using ast-grep's native API
                    let mut variables: HashMap<String, String> = HashMap::new();
                    let env = m.get_env();

                    // Get all matched single variables (e.g., $NAME, $TYPE, $EXPR)
                    // and multi-variables (e.g., $$$ARGS, $$$BODY)
                    for meta_var in env.get_matched_variables() {
                        match meta_var {
                            MetaVariable::Capture(name, _) => {
                                if let Some(node) = env.get_match(&name) {
                                    variables.insert(name, node.text().to_string());
                                }
                            }
                            MetaVariable::MultiCapture(name) => {
                                let nodes = env.get_multiple_matches(&name);
                                if !nodes.is_empty() {
                                    // Join multiple captures with ", " for multi-variables
                                    let var_text: String = nodes
                                        .iter()
                                        .map(ast_grep_core::Node::text)
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    variables.insert(name, var_text);
                                }
                            }
                            // Dropped ($_) and Multiple ($$$) are unnamed wildcards - skip them
                            MetaVariable::Dropped(_) | MetaVariable::Multiple => {}
                        }
                    }

                    MatchData {
                        line: start_pos.line() + 1,
                        column: start_pos.column(&m) + 1,
                        text,
                        start_byte,
                        end_byte,
                        variables,
                    }
                })
                .collect();
            matches
        }));

        let matches = match search_result {
            Ok(m) => m,
            Err(_) => {
                return Ok(ToolOutput::error(format!(
                    "Error: Pattern matching failed. Pattern may be invalid for {language_str} syntax: {pattern}"
                )))
            }
        };

        // Validate match count based on mode
        if matches.is_empty() {
            return Ok(ToolOutput::error(format!(
                "Error: No matches found for pattern '{pattern}' in {source_file}"
            )));
        }

        // For non-batch mode, require exactly one match
        if !batch && matches.len() > 1 {
            let locations: Vec<String> = matches
                .iter()
                .map(|m| {
                    let first_line = m.text.lines().next().unwrap_or("");
                    format!("  {}:{}:{}: {}", source_file, m.line, m.column, first_line)
                })
                .collect();
            return Ok(ToolOutput::error(format!(
                "Error: Multiple matches found ({count}). Refactor requires exactly one match:\n{locations}",
                count = matches.len(),
                locations = locations.join("\n")
            )));
        }

        if let Some(target) = target_file {
            // Extract mode: Move code to target file (single match only)
            let match_data = &matches[0];
            let target_path = Path::new(target);

            // Read existing target content (for append mode)
            let existing_content = if target_path.exists() {
                match tokio::fs::read_to_string(target_path).await {
                    Ok(content) => content,
                    Err(e) => {
                        return Ok(ToolOutput::error(format!(
                            "Error: Failed to read target file: {e}"
                        )))
                    }
                }
            } else {
                String::new()
            };

            // Preview mode: return what would happen without modifying files
            if preview {
                let result = json!({
                    "success": true,
                    "mode": "extract",
                    "preview": true,
                    "moved_code": match_data.text,
                    "source_file": source_file,
                    "target_file": target,
                    "match_location": {
                        "line": match_data.line,
                        "column": match_data.column
                    },
                    "matches": [{
                        "location": format!("{}:{}:{}", source_file, match_data.line, match_data.column),
                        "original": match_data.text,
                        "replacement": "[moved to target file]"
                    }]
                });
                return Ok(ToolOutput::success(result.to_string()));
            }

            // Remove matched code from source
            let mut new_source = source_content.clone();
            new_source.replace_range(match_data.start_byte..match_data.end_byte, "");

            // Clean up blank lines (collapse multiple consecutive newlines)
            new_source = Self::cleanup_blank_lines(&new_source);

            // Write updated source file
            if let Err(e) = tokio::fs::write(source_path, &new_source).await {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to write source file: {e}"
                )));
            }

            // Append to target file
            let new_target = if existing_content.is_empty() {
                match_data.text.clone()
            } else {
                format!("{}\n\n{}", existing_content.trim_end(), match_data.text)
            };

            if let Err(e) = tokio::fs::write(target_path, &new_target).await {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to write target file: {e}"
                )));
            }

            // Return success result
            let result = json!({
                "success": true,
                "mode": "extract",
                "moved_code": match_data.text,
                "source_file": source_file,
                "target_file": target,
                "match_location": {
                    "line": match_data.line,
                    "column": match_data.column
                }
            });

            Ok(ToolOutput::success(result.to_string()))
        } else if let Some(repl) = replacement {
            // Replace mode: Replace code in-place (supports batch and transforms)

            // Build match info for all matches
            let mut match_infos: Vec<MatchInfo> = Vec::new();
            let mut replacements: Vec<(usize, usize, String)> = Vec::new();

            for match_data in &matches {
                // Use the captured variables from ast-grep's native API
                let variables = match_data.variables.clone();

                // Apply transforms if provided
                let final_variables = if let Some(ref t) = transforms {
                    match Self::apply_transforms(t, &variables) {
                        Ok(v) => v,
                        Err(e) => {
                            return Ok(ToolOutput::error(format!("Error: Transform failed: {e}")));
                        }
                    }
                } else {
                    variables
                };

                // Apply replacement template
                let final_replacement = Self::apply_replacement_template(repl, &final_variables);

                match_infos.push(MatchInfo {
                    location: format!("{}:{}:{}", source_file, match_data.line, match_data.column),
                    original: match_data.text.clone(),
                    replacement: final_replacement.clone(),
                });

                replacements.push((
                    match_data.start_byte,
                    match_data.end_byte,
                    final_replacement,
                ));
            }

            // Preview mode: return what would happen without modifying files
            if preview {
                let result = json!({
                    "success": true,
                    "mode": "replace",
                    "preview": true,
                    "source_file": source_file,
                    "matches_count": match_infos.len(),
                    "matches": match_infos.iter().map(|m| {
                        json!({
                            "location": m.location,
                            "original": m.original,
                            "replacement": m.replacement
                        })
                    }).collect::<Vec<_>>()
                });
                return Ok(ToolOutput::success(result.to_string()));
            }

            // Apply replacements in reverse order to maintain byte offsets
            replacements.sort_by(|a, b| b.0.cmp(&a.0));
            let mut new_source = source_content.clone();
            for (start, end, replacement) in &replacements {
                new_source.replace_range(*start..*end, replacement);
            }

            // Write updated source file
            if let Err(e) = tokio::fs::write(source_path, &new_source).await {
                return Ok(ToolOutput::error(format!(
                    "Error: Failed to write source file: {e}"
                )));
            }

            // Return success result
            if batch || matches.len() > 1 {
                let result = json!({
                    "success": true,
                    "mode": "replace",
                    "source_file": source_file,
                    "matches_count": match_infos.len(),
                    "matches": match_infos.iter().map(|m| {
                        json!({
                            "location": m.location,
                            "original": m.original,
                            "replacement": m.replacement
                        })
                    }).collect::<Vec<_>>()
                });
                Ok(ToolOutput::success(result.to_string()))
            } else {
                let match_data = &matches[0];
                let result = json!({
                    "success": true,
                    "mode": "replace",
                    "original_code": match_data.text,
                    "replacement_code": match_infos[0].replacement,
                    "source_file": source_file,
                    "match_location": {
                        "line": match_data.line,
                        "column": match_data.column
                    }
                });
                Ok(ToolOutput::success(result.to_string()))
            }
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

    /// Get the source variable name from a transform
    fn get_transform_source(transform: &Transform) -> &str {
        match transform {
            Transform::Substring(t) => &t.source,
            Transform::Replace(t) => &t.source,
            Transform::Convert(t) => &t.source,
        }
    }

    /// Extract variable name from source (e.g., "$NAME" -> "NAME", "$STRIPPED" -> "STRIPPED")
    fn extract_var_name(source: &str) -> Option<&str> {
        source.strip_prefix('$')
    }

    /// Detect cyclic dependencies in transforms
    fn detect_cyclic_dependencies(
        transforms: &HashMap<String, Transform>,
    ) -> std::result::Result<(), String> {
        // Build dependency graph
        let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
        for (name, transform) in transforms {
            let source = Self::get_transform_source(transform);
            if let Some(source_var) = Self::extract_var_name(source) {
                // Check if source references another transform
                if transforms.contains_key(source_var) {
                    deps.entry(name.as_str()).or_default().push(source_var);
                }
            }
        }

        // DFS to detect cycles
        let mut visited: HashSet<&str> = HashSet::new();
        let mut rec_stack: HashSet<&str> = HashSet::new();

        fn has_cycle<'a>(
            node: &'a str,
            deps: &HashMap<&'a str, Vec<&'a str>>,
            visited: &mut HashSet<&'a str>,
            rec_stack: &mut HashSet<&'a str>,
        ) -> bool {
            visited.insert(node);
            rec_stack.insert(node);

            if let Some(neighbors) = deps.get(node) {
                for &neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        if has_cycle(neighbor, deps, visited, rec_stack) {
                            return true;
                        }
                    } else if rec_stack.contains(neighbor) {
                        return true;
                    }
                }
            }
            rec_stack.remove(node);
            false
        }

        for name in transforms.keys() {
            if !visited.contains(name.as_str())
                && has_cycle(name.as_str(), &deps, &mut visited, &mut rec_stack)
            {
                return Err("Cyclic dependency detected between transforms".to_string());
            }
        }
        Ok(())
    }

    /// Topological sort of transforms by dependency
    fn topological_sort(
        transforms: &HashMap<String, Transform>,
    ) -> std::result::Result<Vec<String>, String> {
        // Build dependency graph
        let mut deps: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut in_degree: HashMap<&str, usize> = HashMap::new();

        for name in transforms.keys() {
            in_degree.insert(name.as_str(), 0);
        }

        for (name, transform) in transforms {
            let source = Self::get_transform_source(transform);
            if let Some(source_var) = Self::extract_var_name(source) {
                if transforms.contains_key(source_var) {
                    deps.entry(source_var).or_default().push(name.as_str());
                    *in_degree.entry(name.as_str()).or_insert(0) += 1;
                }
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&name, _)| name)
            .collect();
        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node.to_string());
            if let Some(neighbors) = deps.get(node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        if result.len() != transforms.len() {
            return Err("Cyclic dependency detected between transforms".to_string());
        }
        Ok(result)
    }

    /// Split text into words based on separators
    fn split_into_words(text: &str, separators: Option<&[Separator]>) -> Vec<String> {
        let default_separators = vec![
            Separator::CaseChange,
            Separator::Underscore,
            Separator::Dash,
            Separator::Space,
        ];
        let seps = separators.unwrap_or(&default_separators);

        let mut words = vec![text.to_string()];

        for sep in seps {
            let mut new_words = Vec::new();
            for word in words {
                match sep {
                    Separator::Underscore => {
                        new_words.extend(word.split('_').map(std::string::ToString::to_string));
                    }
                    Separator::Dash => {
                        new_words.extend(word.split('-').map(std::string::ToString::to_string));
                    }
                    Separator::Dot => {
                        new_words.extend(word.split('.').map(std::string::ToString::to_string));
                    }
                    Separator::Slash => {
                        new_words.extend(word.split('/').map(std::string::ToString::to_string));
                    }
                    Separator::Space => {
                        new_words.extend(
                            word.split_whitespace()
                                .map(std::string::ToString::to_string),
                        );
                    }
                    Separator::CaseChange => {
                        // Split on case transitions (camelCase -> camel, Case)
                        let mut current = String::new();
                        let mut prev_lower = false;
                        for ch in word.chars() {
                            if ch.is_uppercase() && prev_lower && !current.is_empty() {
                                new_words.push(current);
                                current = String::new();
                            }
                            current.push(ch);
                            prev_lower = ch.is_lowercase();
                        }
                        if !current.is_empty() {
                            new_words.push(current);
                        }
                    }
                }
            }
            words = new_words.into_iter().filter(|w| !w.is_empty()).collect();
        }

        words
    }

    /// Convert text to specified case
    fn convert_case(text: &str, to_case: &CaseType, separated_by: Option<&[Separator]>) -> String {
        let words = Self::split_into_words(text, separated_by);
        if words.is_empty() {
            return String::new();
        }

        match to_case {
            CaseType::LowerCase => words.join("").to_lowercase(),
            CaseType::UpperCase => words.join("").to_uppercase(),
            CaseType::Capitalize => {
                let joined = words.join("");
                let mut chars = joined.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                }
            }
            CaseType::CamelCase => words
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    if i == 0 {
                        w.to_lowercase()
                    } else {
                        let mut chars = w.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(c) => {
                                c.to_uppercase().to_string() + &chars.as_str().to_lowercase()
                            }
                        }
                    }
                })
                .collect(),
            CaseType::PascalCase => words
                .iter()
                .map(|w| {
                    let mut chars = w.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                    }
                })
                .collect(),
            CaseType::SnakeCase => words
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("_"),
            CaseType::KebabCase => words
                .iter()
                .map(|w| w.to_lowercase())
                .collect::<Vec<_>>()
                .join("-"),
        }
    }

    /// Apply a single transform to get a result
    fn apply_transform(
        transform: &Transform,
        variables: &HashMap<String, String>,
    ) -> std::result::Result<String, String> {
        match transform {
            Transform::Substring(t) => {
                let source_var = Self::extract_var_name(&t.source)
                    .ok_or_else(|| format!("Invalid source variable: {}", t.source))?;
                let source_text = variables
                    .get(source_var)
                    .ok_or_else(|| format!("Variable not found: {source_var}"))?;

                let len = source_text.chars().count() as i32;
                let start = t.start_char.unwrap_or(0);
                let end = t.end_char.unwrap_or(len);

                // Handle negative indices (Python-style)
                let start_idx = if start < 0 {
                    (len + start).max(0) as usize
                } else {
                    start.min(len) as usize
                };
                let end_idx = if end < 0 {
                    (len + end).max(0) as usize
                } else {
                    end.min(len) as usize
                };

                let chars: Vec<char> = source_text.chars().collect();
                if start_idx >= end_idx || start_idx >= chars.len() {
                    return Ok(String::new());
                }
                Ok(chars[start_idx..end_idx.min(chars.len())].iter().collect())
            }
            Transform::Replace(t) => {
                let source_var = Self::extract_var_name(&t.source)
                    .ok_or_else(|| format!("Invalid source variable: {}", t.source))?;
                let source_text = variables
                    .get(source_var)
                    .ok_or_else(|| format!("Variable not found: {source_var}"))?;

                let regex = Regex::new(&t.replace)
                    .map_err(|e| format!("Invalid regex '{}': {}", t.replace, e))?;
                Ok(regex.replace_all(source_text, &t.by).to_string())
            }
            Transform::Convert(t) => {
                let source_var = Self::extract_var_name(&t.source)
                    .ok_or_else(|| format!("Invalid source variable: {}", t.source))?;
                let source_text = variables
                    .get(source_var)
                    .ok_or_else(|| format!("Variable not found: {source_var}"))?;

                Ok(Self::convert_case(
                    source_text,
                    &t.to_case,
                    t.separated_by.as_deref(),
                ))
            }
        }
    }

    /// Apply all transforms in dependency order
    fn apply_transforms(
        transforms: &HashMap<String, Transform>,
        initial_variables: &HashMap<String, String>,
    ) -> std::result::Result<HashMap<String, String>, String> {
        // Check for cyclic dependencies
        Self::detect_cyclic_dependencies(transforms)?;

        // Get execution order
        let order = Self::topological_sort(transforms)?;

        // Apply transforms in order
        let mut variables = initial_variables.clone();
        for name in order {
            if let Some(transform) = transforms.get(&name) {
                let result = Self::apply_transform(transform, &variables)?;
                variables.insert(name, result);
            }
        }

        Ok(variables)
    }

    /// Apply replacement template with variables
    fn apply_replacement_template(template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (name, value) in variables {
            // Replace $NAME with value
            let var_pattern = format!("${name}");
            result = result.replace(&var_pattern, value);
        }
        result
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
    /// Pattern must match exactly ONE node in the source file (unless batch mode).
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
    /// Can reference captured meta-variables (e.g., $NAME, $$$ARGS) and transformed variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    /// Transform definitions for replace mode. Maps variable names to transforms.
    /// Transforms can chain - output of one transform can be input to another.
    /// NOT supported in extract mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transforms: Option<HashMap<String, Transform>>,
    /// Enable batch mode to replace ALL matches instead of requiring exactly one.
    /// Only applies to replace mode, not extract mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch: Option<bool>,
    /// Enable preview/dry-run mode to see what would change without modifying files.
    /// Returns match locations, original code, and proposed replacements.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<bool>,
}

/// Match information for batch/preview modes
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct MatchInfo {
    /// Location in format "file:line:column"
    pub location: String,
    /// Original matched code
    pub original: String,
    /// Proposed replacement (for preview) or applied replacement (for batch)
    pub replacement: String,
}

/// Result of a refactor operation
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepRefactorResult {
    /// Whether the operation succeeded
    pub success: bool,
    /// The mode used: "extract" or "replace"
    pub mode: String,
    /// The code that was moved (extract mode) or replaced (replace mode, single match)
    pub moved_code: Option<String>,
    /// The original code (replace mode, single match only)
    pub original_code: Option<String>,
    /// The replacement code (replace mode, single match only)
    pub replacement_code: Option<String>,
    /// Source file path
    pub source_file: String,
    /// Target file path (extract mode only)
    pub target_file: Option<String>,
    /// Number of matches (batch mode)
    pub matches_count: Option<usize>,
    /// All matches with details (batch and preview modes)
    pub matches: Option<Vec<MatchInfo>>,
    /// Whether this was a preview (dry-run) - file not modified
    pub preview: Option<bool>,
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
                BATCH MODE (replace mode only):\n\
                - Set batch: true to replace ALL matches instead of requiring exactly one\n\
                - Result includes matches_count with number of replacements made\n\n\
                PREVIEW MODE:\n\
                - Set preview: true to see what would change WITHOUT modifying files\n\
                - Returns match locations, original code, and proposed replacements\n\n\
                PATTERN SYNTAX (matches PARTIAL code structure, not exact strings):\n\
                - $NAME: Single-node wildcard (captures one AST node)\n\
                - $$NAME: Unnamed single-node wildcard\n\
                - $_: Dropped wildcard (matches but doesn't capture)\n\
                - $$$ARGS: Multi-node wildcard (captures multiple nodes)\n\
                - Meta-variable names: UPPERCASE letters, underscores, digits (after first char)\n\n\
                TRANSFORMS (replace mode only, NOT supported in extract mode):\n\
                Transforms modify captured variables before substitution in replacement template.\n\n\
                1. Substring: Extract portion of text (Python-style indexing)\n\
                   {\"SHORT\": {\"substring\": {\"source\": \"$NAME\", \"startChar\": 0, \"endChar\": -5}}}\n\n\
                2. Replace: Regex find/replace on captured text\n\
                   {\"CLEAN\": {\"replace\": {\"source\": \"$NAME\", \"replace\": \"_id$\", \"by\": \"\"}}}\n\n\
                3. Convert: Case conversion with optional word splitting\n\
                   {\"NEW\": {\"convert\": {\"source\": \"$NAME\", \"toCase\": \"camelCase\"}}}\n\n\
                CASE TYPES: lowerCase, upperCase, capitalize, camelCase, snakeCase, kebabCase, pascalCase\n\n\
                SEPARATORS (for word splitting in convert): caseChange, underscore, dash, dot, slash, space\n\
                Default: [caseChange, underscore, dash, space]. Use separatedBy to override.\n\n\
                TRANSFORM CHAINING:\n\
                Transforms can depend on other transforms. They execute in dependency order.\n\
                Example: {\"STRIPPED\": {\"replace\": ...}, \"FINAL\": {\"convert\": {\"source\": \"$STRIPPED\", ...}}}\n\n\
                ERROR HANDLING:\n\
                - Invalid regex fails the entire operation with clear error message\n\
                - Cyclic dependencies between transforms fail with error\n\
                - Transform errors fail whole operation - no silent skipping\n\n\
                EXAMPLES:\n\
                - Extract function: pattern='fn $NAME($$$ARGS) { $$$BODY }' target_file='extracted.rs'\n\
                - Replace function: pattern='fn old() { $$$BODY }' replacement='fn new() { }'\n\
                - Rename to camelCase: pattern='fn $NAME()' transforms={NEW: convert $NAME to camelCase} replacement='fn $NEW()'\n\
                - Batch replace calls: pattern='oldFunc($$$ARGS)' replacement='newFunc($$$ARGS)' batch=true\n\
                - Preview changes: pattern='fn old()' replacement='fn new()' preview=true"
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
        if let Some(transforms) = args.transforms {
            value_map.insert(
                "transforms".to_string(),
                serde_json::to_value(transforms).unwrap_or(serde_json::Value::Null),
            );
        }
        if let Some(batch) = args.batch {
            value_map.insert("batch".to_string(), serde_json::Value::Bool(batch));
        }
        if let Some(preview) = args.preview {
            value_map.insert("preview".to_string(), serde_json::Value::Bool(preview));
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
        assert!(
            !result.contains("\n\n\n"),
            "Should not have 3+ consecutive newlines"
        );
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
