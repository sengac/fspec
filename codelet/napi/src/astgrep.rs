//! AST-grep NAPI bindings
//!
//! Exposes ast-grep search and refactor functionality to TypeScript via NAPI-RS.
//! Reuses the existing AstGrepTool implementation from codelet-tools.

use ast_grep_language::{LanguageExt, SupportLang};
use napi_derive::napi;
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

/// Result of an AST-grep refactor operation
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
