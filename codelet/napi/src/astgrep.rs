//! AST-grep NAPI bindings
//!
//! Exposes ast-grep search and refactor functionality to TypeScript via NAPI-RS.
//! Reuses the existing AstGrepTool implementation from codelet-tools.

use ast_grep_core::meta_var::MetaVariable;
use ast_grep_language::{LanguageExt, SupportLang};
use napi_derive::napi;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Result of an AST-grep search match
#[napi(object)]
pub struct AstGrepMatchResult {
    /// File path where match was found
    pub file: String,
    /// Line number (1-based)
    pub line: u32,
    /// Column number (1-based)
    pub column: u32,
    /// Matched text
    pub text: String,
}

/// Result of an AST-grep refactor operation (extract mode)
#[napi(object)]
pub struct AstGrepRefactorResult {
    /// Whether the refactor was successful
    pub success: bool,
    /// The code that was moved
    pub moved_code: String,
    /// Source file path
    pub source_file: String,
    /// Target file path
    pub target_file: String,
}

/// Result of an AST-grep replace operation (replace mode)
#[napi(object)]
pub struct AstGrepReplaceResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Mode: "replace" or "extract"
    pub mode: String,
    /// Source file path
    pub source_file: String,
    /// Number of matches replaced (batch mode)
    pub matches_count: u32,
    /// Whether this was a preview (dry-run)
    pub preview: bool,
    /// Match details (location, original, replacement)
    pub matches: Vec<AstGrepMatchInfo>,
}

/// Match information for replace operations
#[napi(object)]
pub struct AstGrepMatchInfo {
    /// Location in format "file:line:column"
    pub location: String,
    /// Original matched code
    pub original: String,
    /// Replacement code
    pub replacement: String,
}

/// Case conversion types for transforms
#[napi(string_enum)]
pub enum AstGrepCaseType {
    LowerCase,
    UpperCase,
    Capitalize,
    CamelCase,
    SnakeCase,
    KebabCase,
    PascalCase,
}

/// Separator options for word splitting
#[napi(string_enum)]
pub enum AstGrepSeparator {
    CaseChange,
    Underscore,
    Dash,
    Dot,
    Slash,
    Space,
}

/// Substring transform configuration
#[napi(object)]
pub struct AstGrepSubstringTransform {
    /// Source variable (e.g., "$NAME")
    pub source: String,
    /// Start character index (0-based, negative counts from end)
    pub start_char: Option<i32>,
    /// End character index (negative counts from end)
    pub end_char: Option<i32>,
}

/// Replace transform configuration
#[napi(object)]
pub struct AstGrepReplaceTransform {
    /// Source variable (e.g., "$NAME")
    pub source: String,
    /// Regex pattern to find
    pub replace: String,
    /// Replacement string
    pub by: String,
}

/// Convert transform configuration
#[napi(object)]
pub struct AstGrepConvertTransform {
    /// Source variable (e.g., "$NAME")
    pub source: String,
    /// Target case type
    pub to_case: AstGrepCaseType,
    /// Optional separators for word splitting
    pub separated_by: Option<Vec<AstGrepSeparator>>,
}

/// Transform definition - one of substring, replace, or convert
#[napi(object)]
pub struct AstGrepTransform {
    /// Transform name (the variable it creates, e.g., "NEW")
    pub name: String,
    /// Substring transform (mutually exclusive with replace/convert)
    pub substring: Option<AstGrepSubstringTransform>,
    /// Replace transform (mutually exclusive with substring/convert)
    pub replace_transform: Option<AstGrepReplaceTransform>,
    /// Convert transform (mutually exclusive with substring/replace)
    pub convert: Option<AstGrepConvertTransform>,
}

/// Parse language string to SupportLang enum
fn parse_language(lang: &str) -> Option<SupportLang> {
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

/// Search for AST pattern matches in files
///
/// # Arguments
/// * `pattern` - AST pattern to search for (e.g., "function $NAME($$$ARGS)")
/// * `language` - Programming language (e.g., "typescript", "rust")
/// * `paths` - List of file or directory paths to search
///
/// # Returns
/// Array of match results with file, line, column, and matched text
#[napi]
pub async fn ast_grep_search(
    pattern: String,
    language: String,
    paths: Vec<String>,
) -> napi::Result<Vec<AstGrepMatchResult>> {
    // Parse language
    let lang = parse_language(&language).ok_or_else(|| {
        napi::Error::from_reason(format!(
            "Unsupported language '{}'. Supported: typescript, javascript, rust, python, go, java, c, cpp, ruby, kotlin, swift, etc.",
            language
        ))
    })?;

    let extensions = get_extensions(lang);
    let mut all_matches = Vec::new();

    for path_str in &paths {
        let path = Path::new(path_str);

        if !path.exists() {
            continue;
        }

        if path.is_file() {
            // Search single file
            let matches = search_file(path, &pattern, lang).await;
            all_matches.extend(matches);
        } else {
            // Walk directory
            let walker = ignore::WalkBuilder::new(path)
                .hidden(false)
                .git_ignore(true)
                .build();

            for entry in walker.flatten() {
                if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                    continue;
                }

                let file_path = entry.path();
                let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

                if !extensions.contains(&ext) {
                    continue;
                }

                let matches = search_file(file_path, &pattern, lang).await;
                all_matches.extend(matches);
            }
        }
    }

    Ok(all_matches)
}

/// Search a single file for pattern matches
async fn search_file(path: &Path, pattern: &str, lang: SupportLang) -> Vec<AstGrepMatchResult> {
    let source = match tokio::fs::read_to_string(path).await {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    let path_str = path.to_string_lossy().to_string();
    let pattern_owned = pattern.to_string();

    // Wrap in catch_unwind to prevent panics from crashing
    let search_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut results = Vec::new();
        let ast_grep = lang.ast_grep(&source);
        let root = ast_grep.root();
        let matches = root.find_all(pattern_owned.as_str());

        for node_match in matches {
            let start_pos = node_match.start_pos();
            let line = (start_pos.line() + 1) as u32;
            let column = (start_pos.column(&node_match) + 1) as u32;
            let text = node_match.text().to_string();

            results.push(AstGrepMatchResult {
                file: path_str.clone(),
                line,
                column,
                text,
            });
        }
        results
    }));

    match search_result {
        Ok(results) => results,
        Err(_) => Vec::new(),
    }
}

/// Refactor by moving matched code from source file to target file
///
/// # Arguments
/// * `pattern` - AST pattern to match (must match exactly 1 node)
/// * `language` - Programming language
/// * `source_file` - Path to source file
/// * `target_file` - Path to target file (will be created)
///
/// # Returns
/// Result containing the moved code, or error if pattern doesn't match exactly 1 node
#[napi]
pub async fn ast_grep_refactor(
    pattern: String,
    language: String,
    source_file: String,
    target_file: String,
) -> napi::Result<AstGrepRefactorResult> {
    // Parse language
    let lang = parse_language(&language).ok_or_else(|| {
        napi::Error::from_reason(format!("Unsupported language '{}'", language))
    })?;

    // Read source file
    let source_path = Path::new(&source_file);
    let source_content = tokio::fs::read_to_string(source_path)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Failed to read source file: {}", e)))?;

    // Find matches
    let pattern_owned = pattern.clone();
    let source_clone = source_content.clone();
    
    let matches_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ast_grep = lang.ast_grep(&source_clone);
        let root = ast_grep.root();
        let matches: Vec<_> = root.find_all(pattern_owned.as_str()).collect();
        
        matches.iter().map(|m| {
            let start_pos = m.start_pos();
            (
                m.text().to_string(),
                start_pos.line() + 1,
                m.range().start,
                m.range().end,
            )
        }).collect::<Vec<_>>()
    }));

    let matches = matches_result.map_err(|_| {
        napi::Error::from_reason("Pattern matching failed - invalid pattern syntax")
    })?;

    // Check match count
    if matches.is_empty() {
        return Err(napi::Error::from_reason(
            "Pattern matched 0 nodes. No code to refactor.",
        ));
    }

    if matches.len() > 1 {
        let mut error_msg = format!(
            "Pattern matched {} nodes. Refactor requires exactly 1 match.\n\nMatches found:",
            matches.len()
        );
        for (text, line, _, _) in &matches {
            let first_line = text.lines().next().unwrap_or("");
            error_msg.push_str(&format!("\n  {}:{}: {}", source_file, line, first_line));
        }
        error_msg.push_str("\n\nMake your pattern more specific.");
        return Err(napi::Error::from_reason(error_msg));
    }

    // Extract the matched code
    let (matched_text, _, start_byte, end_byte) = &matches[0];

    // Remove matched code from source
    let mut new_source = source_content.clone();
    new_source.replace_range(*start_byte..*end_byte, "");
    
    // Clean up extra blank lines that might result
    let new_source = new_source
        .lines()
        .collect::<Vec<_>>()
        .join("\n");

    // Write updated source file
    tokio::fs::write(source_path, &new_source)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Failed to write source file: {}", e)))?;

    // Write target file with matched code
    let target_path = Path::new(&target_file);
    tokio::fs::write(target_path, matched_text)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Failed to write target file: {}", e)))?;

    Ok(AstGrepRefactorResult {
        success: true,
        moved_code: matched_text.clone(),
        source_file,
        target_file,
    })
}

/// Replace matched code in-place with optional transforms
///
/// # Arguments
/// * `pattern` - AST pattern to match
/// * `language` - Programming language
/// * `source_file` - Path to source file
/// * `replacement` - Replacement template (can reference $NAME, $$$ARGS, and transform outputs)
/// * `transforms` - Optional array of transforms to apply to captured variables
/// * `batch` - If true, replace ALL matches; if false, require exactly one match
/// * `preview` - If true, return what would change without modifying files
///
/// # Returns
/// Result containing match details and replacement info
#[napi]
pub async fn ast_grep_replace(
    pattern: String,
    language: String,
    source_file: String,
    replacement: String,
    transforms: Option<Vec<AstGrepTransform>>,
    batch: Option<bool>,
    preview: Option<bool>,
) -> napi::Result<AstGrepReplaceResult> {
    let batch = batch.unwrap_or(false);
    let preview = preview.unwrap_or(false);

    // Parse language
    let lang = parse_language(&language).ok_or_else(|| {
        napi::Error::from_reason(format!("Unsupported language '{}'", language))
    })?;

    // Validate transforms (check for cyclic dependencies and invalid regex)
    if let Some(ref t) = transforms {
        validate_transforms(t)?;
    }

    // Read source file
    let source_path = Path::new(&source_file);
    let source_content = tokio::fs::read_to_string(source_path)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Failed to read source file: {}", e)))?;

    // Find matches - keep NodeMatch objects to access captured meta-variables via get_env()
    let pattern_owned = pattern.clone();
    let source_clone = source_content.clone();

    // Struct to hold match data including captured variables
    struct MatchData {
        text: String,
        line: u32,
        column: u32,
        start_byte: usize,
        end_byte: usize,
        variables: HashMap<String, String>,
    }

    let matches_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ast_grep = lang.ast_grep(&source_clone);
        let root = ast_grep.root();
        let matches: Vec<_> = root.find_all(pattern_owned.as_str()).collect();

        matches
            .iter()
            .map(|m| {
                let start_pos = m.start_pos();

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
                                let text: String = nodes.iter()
                                    .map(|n| n.text())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                variables.insert(name, text);
                            }
                        }
                        // Dropped ($_) and Multiple ($$$) are unnamed wildcards - skip them
                        MetaVariable::Dropped(_) | MetaVariable::Multiple => {}
                    }
                }

                MatchData {
                    text: m.text().to_string(),
                    line: (start_pos.line() + 1) as u32,
                    column: (start_pos.column(m) + 1) as u32,
                    start_byte: m.range().start,
                    end_byte: m.range().end,
                    variables,
                }
            })
            .collect::<Vec<_>>()
    }));

    let matches = matches_result.map_err(|_| {
        napi::Error::from_reason("Pattern matching failed - invalid pattern syntax")
    })?;

    // Check match count
    if matches.is_empty() {
        return Err(napi::Error::from_reason(format!(
            "No matches found for pattern '{}' in {}",
            pattern, source_file
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
        return Err(napi::Error::from_reason(format!(
            "Multiple matches found ({}). Use batch: true or make pattern more specific:\n{}",
            matches.len(),
            locations.join("\n")
        )));
    }

    // Build match info for all matches
    let mut match_infos: Vec<AstGrepMatchInfo> = Vec::new();
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();

    for match_data in &matches {
        // Use the captured variables from ast-grep's native API
        let variables = match_data.variables.clone();

        // Apply transforms if provided
        let final_variables = if let Some(ref t) = transforms {
            apply_transforms(t, &variables)?
        } else {
            variables
        };

        // Apply replacement template
        let final_replacement = apply_replacement_template(&replacement, &final_variables);

        match_infos.push(AstGrepMatchInfo {
            location: format!("{}:{}:{}", source_file, match_data.line, match_data.column),
            original: match_data.text.clone(),
            replacement: final_replacement.clone(),
        });

        replacements.push((match_data.start_byte, match_data.end_byte, final_replacement));
    }

    // Preview mode: return what would happen without modifying files
    if preview {
        return Ok(AstGrepReplaceResult {
            success: true,
            mode: "replace".to_string(),
            source_file,
            matches_count: match_infos.len() as u32,
            preview: true,
            matches: match_infos,
        });
    }

    // Apply replacements in reverse order to maintain byte offsets
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    let mut new_source = source_content.clone();
    for (start, end, repl) in &replacements {
        new_source.replace_range(*start..*end, repl);
    }

    // Write updated source file
    tokio::fs::write(source_path, &new_source)
        .await
        .map_err(|e| napi::Error::from_reason(format!("Failed to write source file: {}", e)))?;

    Ok(AstGrepReplaceResult {
        success: true,
        mode: "replace".to_string(),
        source_file,
        matches_count: match_infos.len() as u32,
        preview: false,
        matches: match_infos,
    })
}

// ============================================================================
// Transform helper functions
// ============================================================================

/// Validate transforms for cyclic dependencies and invalid regex
fn validate_transforms(transforms: &[AstGrepTransform]) -> napi::Result<()> {
    // Check for cyclic dependencies
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    for t in transforms {
        let source = get_transform_source(t);
        if let Some(source_var) = extract_var_name(&source) {
            if transforms.iter().any(|other| other.name == source_var) {
                deps.entry(t.name.clone()).or_default().push(source_var);
            }
        }
    }

    // DFS to detect cycles
    let mut visited: HashSet<String> = HashSet::new();
    let mut rec_stack: HashSet<String> = HashSet::new();

    for t in transforms {
        if !visited.contains(&t.name) {
            if has_cycle(&t.name, &deps, &mut visited, &mut rec_stack) {
                return Err(napi::Error::from_reason(
                    "Cyclic dependency detected between transforms",
                ));
            }
        }
    }

    // Validate regex patterns
    for t in transforms {
        if let Some(ref r) = t.replace_transform {
            if Regex::new(&r.replace).is_err() {
                return Err(napi::Error::from_reason(format!(
                    "Invalid regex in transform '{}': {}",
                    t.name, r.replace
                )));
            }
        }
    }

    Ok(())
}

fn has_cycle(
    node: &str,
    deps: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
) -> bool {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());

    if let Some(neighbors) = deps.get(node) {
        for neighbor in neighbors {
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

fn get_transform_source(transform: &AstGrepTransform) -> String {
    if let Some(ref s) = transform.substring {
        s.source.clone()
    } else if let Some(ref r) = transform.replace_transform {
        r.source.clone()
    } else if let Some(ref c) = transform.convert {
        c.source.clone()
    } else {
        String::new()
    }
}

fn extract_var_name(source: &str) -> Option<String> {
    if source.starts_with('$') {
        Some(source[1..].to_string())
    } else {
        None
    }
}

/// Apply all transforms in dependency order
fn apply_transforms(
    transforms: &[AstGrepTransform],
    initial_variables: &HashMap<String, String>,
) -> napi::Result<HashMap<String, String>> {
    // Get execution order via topological sort
    let order = topological_sort(transforms)?;

    let mut variables = initial_variables.clone();
    for name in order {
        if let Some(transform) = transforms.iter().find(|t| t.name == name) {
            let result = apply_transform(transform, &variables)?;
            variables.insert(name, result);
        }
    }

    Ok(variables)
}

fn topological_sort(transforms: &[AstGrepTransform]) -> napi::Result<Vec<String>> {
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    let mut in_degree: HashMap<String, usize> = HashMap::new();

    for t in transforms {
        in_degree.insert(t.name.clone(), 0);
    }

    for t in transforms {
        let source = get_transform_source(t);
        if let Some(source_var) = extract_var_name(&source) {
            if transforms.iter().any(|other| other.name == source_var) {
                deps.entry(source_var).or_default().push(t.name.clone());
                *in_degree.entry(t.name.clone()).or_insert(0) += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();
    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.clone());
        if let Some(neighbors) = deps.get(&node) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(neighbor.clone());
                    }
                }
            }
        }
    }

    if result.len() != transforms.len() {
        return Err(napi::Error::from_reason(
            "Cyclic dependency detected between transforms",
        ));
    }
    Ok(result)
}

fn apply_transform(
    transform: &AstGrepTransform,
    variables: &HashMap<String, String>,
) -> napi::Result<String> {
    if let Some(ref s) = transform.substring {
        let source_var = extract_var_name(&s.source)
            .ok_or_else(|| napi::Error::from_reason(format!("Invalid source variable: {}", s.source)))?;
        let source_text = variables
            .get(&source_var)
            .ok_or_else(|| napi::Error::from_reason(format!("Variable not found: {}", source_var)))?;

        let len = source_text.chars().count() as i32;
        let start = s.start_char.unwrap_or(0);
        let end = s.end_char.unwrap_or(len);

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
    } else if let Some(ref r) = transform.replace_transform {
        let source_var = extract_var_name(&r.source)
            .ok_or_else(|| napi::Error::from_reason(format!("Invalid source variable: {}", r.source)))?;
        let source_text = variables
            .get(&source_var)
            .ok_or_else(|| napi::Error::from_reason(format!("Variable not found: {}", source_var)))?;

        let regex = Regex::new(&r.replace)
            .map_err(|e| napi::Error::from_reason(format!("Invalid regex '{}': {}", r.replace, e)))?;
        Ok(regex.replace_all(source_text, &r.by).to_string())
    } else if let Some(ref c) = transform.convert {
        let source_var = extract_var_name(&c.source)
            .ok_or_else(|| napi::Error::from_reason(format!("Invalid source variable: {}", c.source)))?;
        let source_text = variables
            .get(&source_var)
            .ok_or_else(|| napi::Error::from_reason(format!("Variable not found: {}", source_var)))?;

        Ok(convert_case(source_text, &c.to_case, c.separated_by.as_deref()))
    } else {
        Err(napi::Error::from_reason(format!(
            "Transform '{}' has no operation defined",
            transform.name
        )))
    }
}

fn convert_case(text: &str, to_case: &AstGrepCaseType, separated_by: Option<&[AstGrepSeparator]>) -> String {
    let words = split_into_words(text, separated_by);
    if words.is_empty() {
        return String::new();
    }

    match to_case {
        AstGrepCaseType::LowerCase => words.join("").to_lowercase(),
        AstGrepCaseType::UpperCase => words.join("").to_uppercase(),
        AstGrepCaseType::Capitalize => {
            let joined = words.join("");
            let mut chars = joined.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        }
        AstGrepCaseType::CamelCase => words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if i == 0 {
                    w.to_lowercase()
                } else {
                    let mut chars = w.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                    }
                }
            })
            .collect(),
        AstGrepCaseType::PascalCase => words
            .iter()
            .map(|w| {
                let mut chars = w.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
                }
            })
            .collect(),
        AstGrepCaseType::SnakeCase => words
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("_"),
        AstGrepCaseType::KebabCase => words
            .iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("-"),
    }
}

fn split_into_words(text: &str, separators: Option<&[AstGrepSeparator]>) -> Vec<String> {
    let default_separators = vec![
        AstGrepSeparator::CaseChange,
        AstGrepSeparator::Underscore,
        AstGrepSeparator::Dash,
        AstGrepSeparator::Space,
    ];
    let seps = separators.unwrap_or(&default_separators);

    let mut words = vec![text.to_string()];

    for sep in seps {
        let mut new_words = Vec::new();
        for word in words {
            match sep {
                AstGrepSeparator::Underscore => {
                    new_words.extend(word.split('_').map(|s| s.to_string()));
                }
                AstGrepSeparator::Dash => {
                    new_words.extend(word.split('-').map(|s| s.to_string()));
                }
                AstGrepSeparator::Dot => {
                    new_words.extend(word.split('.').map(|s| s.to_string()));
                }
                AstGrepSeparator::Slash => {
                    new_words.extend(word.split('/').map(|s| s.to_string()));
                }
                AstGrepSeparator::Space => {
                    new_words.extend(word.split_whitespace().map(|s| s.to_string()));
                }
                AstGrepSeparator::CaseChange => {
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

fn apply_replacement_template(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (name, value) in variables {
        let var_pattern = format!("${}", name);
        result = result.replace(&var_pattern, value);
    }
    result
}

