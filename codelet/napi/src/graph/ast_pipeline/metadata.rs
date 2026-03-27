//! Shared Metadata Extraction Helpers
//!
//! Language-agnostic extraction of source code, docstrings, decorators, and
//! parameter names from AST-matched text. Single entry points used by all
//! 14 language extractors via data-driven per-language configuration.
//!
//! CGC equivalent: source, docstring, args, decorators properties extracted
//! per function/class by each language parser.
//!
//! Follows the same DRY pattern as `complexity.rs`.

/// Bundled metadata for a function entity.
pub struct FunctionMeta {
    /// Comma-separated parameter names (no types).
    pub parameters: String,
    /// Source code, capped at MAX_SOURCE_LINES / MAX_SOURCE_BYTES.
    pub source: String,
    /// Extracted doc comment.
    pub docstring: String,
    /// Comma-separated decorator/annotation strings.
    pub decorators: String,
    /// Whether source was truncated.
    pub truncated: bool,
}

/// Bundled metadata for a type entity.
pub struct TypeMeta {
    /// Source code, capped at MAX_SOURCE_LINES / MAX_SOURCE_BYTES.
    pub source: String,
    /// Extracted doc comment.
    pub docstring: String,
    /// Comma-separated decorator/annotation strings.
    pub decorators: String,
    /// Whether source was truncated.
    pub truncated: bool,
}

/// Extract all metadata for a function from its matched AST text.
///
/// Single call replaces 4 individual extractions. Used by all 14 extractors.
pub fn extract_function_meta(matched_text: &str, language: &str) -> FunctionMeta {
    let parameters = extract_parameters(matched_text, language);
    let (source, truncated) = extract_source(matched_text);
    let docstring = extract_docstring(matched_text, language);
    let decorators = extract_decorators(matched_text, language);
    FunctionMeta { parameters, source, docstring, decorators, truncated }
}

/// Extract all metadata for a type from its matched AST text.
///
/// Single call replaces 3 individual extractions. Used by all 14 extractors.
pub fn extract_type_meta(matched_text: &str, language: &str) -> TypeMeta {
    let (source, truncated) = extract_source(matched_text);
    let docstring = extract_docstring(matched_text, language);
    let decorators = extract_decorators(matched_text, language);
    TypeMeta { source, docstring, decorators, truncated }
}

/// Maximum number of lines to store in the `source` property.
const MAX_SOURCE_LINES: usize = 100;

/// Maximum bytes to store in the `source` property.
const MAX_SOURCE_BYTES: usize = 4096;

// ═══════════════════════════════════════════════════════════════
// Source Extraction
// ═══════════════════════════════════════════════════════════════

/// Extract source code with a cap on size.
///
/// Returns `(capped_source, was_truncated)`.
/// Caps at `MAX_SOURCE_LINES` lines or `MAX_SOURCE_BYTES` bytes,
/// whichever limit is hit first.
pub fn extract_source(full_text: &str) -> (String, bool) {
    let lines: Vec<&str> = full_text.lines().collect();

    if lines.len() <= MAX_SOURCE_LINES && full_text.len() <= MAX_SOURCE_BYTES {
        return (full_text.to_string(), false);
    }

    // Take up to MAX_SOURCE_LINES
    let capped_lines = &lines[..lines.len().min(MAX_SOURCE_LINES)];
    let mut result = capped_lines.join("\n");

    // If still over byte limit, truncate further
    if result.len() > MAX_SOURCE_BYTES {
        result.truncate(MAX_SOURCE_BYTES);
        // Trim to last complete line
        if let Some(last_newline) = result.rfind('\n') {
            result.truncate(last_newline);
        }
    }

    (result, true)
}

// ═══════════════════════════════════════════════════════════════
// Docstring Extraction
// ═══════════════════════════════════════════════════════════════

/// Docstring comment style for each language family.
#[derive(Clone, Copy)]
enum DocStyle {
    /// `/** ... */` (JSDoc, Javadoc, PHPDoc)
    SlashStarStar,
    /// `///` or `//!` line comments (Rust, Dart, Swift, C#)
    TripleSlash,
    /// Python triple-quoted docstrings inside function body
    PythonDocstring,
    /// `#` line comments before declaration (Ruby, YARD)
    HashLines,
    /// `//` line comments before declaration (Go)
    DoubleSlashLines,
    /// C/C++: either `/** ... */` or `///`
    CStyleMixed,
}

/// Per-language docstring configuration.
struct DocConfig {
    style: DocStyle,
}

const TS_DOC: DocConfig = DocConfig { style: DocStyle::SlashStarStar };
const RUST_DOC: DocConfig = DocConfig { style: DocStyle::TripleSlash };
const PYTHON_DOC: DocConfig = DocConfig { style: DocStyle::PythonDocstring };
const GO_DOC: DocConfig = DocConfig { style: DocStyle::DoubleSlashLines };
const JAVA_DOC: DocConfig = DocConfig { style: DocStyle::SlashStarStar };
const C_DOC: DocConfig = DocConfig { style: DocStyle::CStyleMixed };
const CPP_DOC: DocConfig = DocConfig { style: DocStyle::CStyleMixed };
const CSHARP_DOC: DocConfig = DocConfig { style: DocStyle::TripleSlash };
const RUBY_DOC: DocConfig = DocConfig { style: DocStyle::HashLines };
const KOTLIN_DOC: DocConfig = DocConfig { style: DocStyle::SlashStarStar };
const SWIFT_DOC: DocConfig = DocConfig { style: DocStyle::TripleSlash };
const SCALA_DOC: DocConfig = DocConfig { style: DocStyle::SlashStarStar };
const PHP_DOC: DocConfig = DocConfig { style: DocStyle::SlashStarStar };
const DART_DOC: DocConfig = DocConfig { style: DocStyle::TripleSlash };

/// Get docstring config for a language.
fn doc_config_for_language(language: &str) -> &'static DocConfig {
    match language {
        "typescript" | "tsx" | "javascript" | "jsx" => &TS_DOC,
        "rust" => &RUST_DOC,
        "python" => &PYTHON_DOC,
        "go" => &GO_DOC,
        "java" => &JAVA_DOC,
        "c" => &C_DOC,
        "cpp" | "c++" => &CPP_DOC,
        "csharp" | "c#" => &CSHARP_DOC,
        "ruby" => &RUBY_DOC,
        "kotlin" => &KOTLIN_DOC,
        "swift" => &SWIFT_DOC,
        "scala" => &SCALA_DOC,
        "php" => &PHP_DOC,
        "dart" => &DART_DOC,
        _ => &TS_DOC,
    }
}

/// Extract the docstring/doc comment from the full matched text of an entity.
///
/// The `text` should be the complete AST-matched text that includes any
/// leading doc comments (for languages where the AST node captures them)
/// or the function/type body (for Python docstrings).
pub fn extract_docstring(text: &str, language: &str) -> String {
    let config = doc_config_for_language(language);
    match config.style {
        DocStyle::SlashStarStar => extract_slash_star_star(text),
        DocStyle::TripleSlash => extract_triple_slash(text),
        DocStyle::PythonDocstring => extract_python_docstring(text),
        DocStyle::HashLines => extract_hash_doc_lines(text),
        DocStyle::DoubleSlashLines => extract_double_slash_lines(text),
        DocStyle::CStyleMixed => {
            // Try /** first, then ///
            let result = extract_slash_star_star(text);
            if result.is_empty() {
                extract_triple_slash(text)
            } else {
                result
            }
        }
    }
}

/// Extract `/** ... */` block doc comments.
fn extract_slash_star_star(text: &str) -> String {
    if let Some(start) = text.find("/**") {
        if let Some(end) = text[start..].find("*/") {
            return text[start..start + end + 2].trim().to_string();
        }
    }
    String::new()
}

/// Extract `///` or `//!` line doc comments.
fn extract_triple_slash(text: &str) -> String {
    let mut doc_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            doc_lines.push(trimmed);
        } else if !doc_lines.is_empty() {
            // Stop at first non-doc line after we've started collecting
            break;
        }
    }
    doc_lines.join("\n")
}

/// Extract Python triple-quoted docstring from inside function/class body.
fn extract_python_docstring(text: &str) -> String {
    // Look for triple-quoted string after the first `:` (def/class line)
    let after_colon = if let Some(colon_pos) = text.find(':') {
        &text[colon_pos + 1..]
    } else {
        text
    };

    let trimmed = after_colon.trim();

    // Try triple double-quotes first, then triple single-quotes
    for quote in &["\"\"\"", "'''"] {
        if trimmed.starts_with(quote) {
            let after_open = &trimmed[quote.len()..];
            if let Some(close_pos) = after_open.find(quote) {
                return after_open[..close_pos].trim().to_string();
            }
        }
    }

    String::new()
}

/// Extract `#` line comments before a declaration (Ruby/YARD).
fn extract_hash_doc_lines(text: &str) -> String {
    let mut doc_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            doc_lines.push(trimmed.trim_start_matches('#').trim());
        } else if !doc_lines.is_empty() {
            break;
        }
    }
    doc_lines.join("\n")
}

/// Extract `//` line comments before a declaration (Go).
fn extract_double_slash_lines(text: &str) -> String {
    let mut doc_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") && !trimmed.starts_with("///") {
            doc_lines.push(trimmed.trim_start_matches("//").trim());
        } else if !doc_lines.is_empty() {
            break;
        }
    }
    doc_lines.join("\n")
}

// ═══════════════════════════════════════════════════════════════
// Decorator / Annotation Extraction
// ═══════════════════════════════════════════════════════════════

/// Decorator syntax style per language.
#[derive(Clone, Copy)]
enum DecoratorStyle {
    /// `@name` or `@name(args)` — Python, TypeScript, Dart, Java, Kotlin
    AtSign,
    /// `#[name]` or `#[name(args)]` — Rust
    HashBracket,
    /// `[Name]` or `[Name(args)]` — C#
    SquareBracket,
    /// No standard decorator syntax
    None,
}

/// Get decorator style for a language.
fn decorator_style_for_language(language: &str) -> DecoratorStyle {
    match language {
        "python" | "typescript" | "tsx" | "javascript" | "jsx" | "dart" => DecoratorStyle::AtSign,
        "java" | "kotlin" | "scala" => DecoratorStyle::AtSign,
        "rust" => DecoratorStyle::HashBracket,
        "php" => DecoratorStyle::HashBracket,
        "csharp" | "c#" => DecoratorStyle::SquareBracket,
        "swift" => DecoratorStyle::AtSign,
        _ => DecoratorStyle::None,
    }
}

/// Extract decorators/annotations from the text before an entity declaration.
///
/// Returns comma-separated decorator strings, e.g. `"@staticmethod, @override"`.
pub fn extract_decorators(text: &str, language: &str) -> String {
    let style = decorator_style_for_language(language);
    let decorators = match style {
        DecoratorStyle::AtSign => extract_at_decorators(text),
        DecoratorStyle::HashBracket => extract_hash_bracket_attrs(text),
        DecoratorStyle::SquareBracket => extract_square_bracket_attrs(text),
        DecoratorStyle::None => Vec::new(),
    };
    decorators.join(", ")
}

/// Extract `@name` or `@name(...)` decorators.
fn extract_at_decorators(text: &str) -> Vec<String> {
    let mut decorators = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            // Take the whole decorator line (e.g., `@SuppressWarnings("unchecked")`)
            let dec = trimmed.split_whitespace().next().unwrap_or(trimmed);
            // But only if it's before the actual declaration (not an annotation in body)
            decorators.push(dec.to_string());
        } else if !decorators.is_empty()
            && !trimmed.is_empty()
            && !trimmed.starts_with("//")
            && !trimmed.starts_with('#')
        {
            // Stop at first non-decorator, non-comment, non-empty line
            break;
        }
    }
    decorators
}

/// Extract `#[name]` or `#[name(...)]` attributes (Rust).
fn extract_hash_bracket_attrs(text: &str) -> Vec<String> {
    let mut attrs = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#[") {
            attrs.push(trimmed.to_string());
        } else if !attrs.is_empty()
            && !trimmed.is_empty()
            && !trimmed.starts_with("//")
        {
            break;
        }
    }
    attrs
}

/// Extract `[Name]` or `[Name(...)]` attributes (C#).
fn extract_square_bracket_attrs(text: &str) -> Vec<String> {
    let mut attrs = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.contains(']') {
            attrs.push(trimmed.to_string());
        } else if !attrs.is_empty()
            && !trimmed.is_empty()
            && !trimmed.starts_with("//")
        {
            break;
        }
    }
    attrs
}

// ═══════════════════════════════════════════════════════════════
// Parameter Extraction
// ═══════════════════════════════════════════════════════════════

/// Extract parameter names from a function signature, without types.
///
/// Returns comma-separated parameter names. Filters language-specific
/// "self" parameters (Python `self`/`cls`, Rust `&self`/`self`, Go receiver).
pub fn extract_parameters(signature: &str, language: &str) -> String {
    let params_text = extract_params_text(signature, language);
    if params_text.is_empty() {
        return String::new();
    }

    let names: Vec<String> = params_text
        .split(',')
        .map(|p| extract_param_name(p.trim(), language))
        .filter(|n| !n.is_empty())
        .filter(|n| !is_self_param(n, language))
        .collect();

    names.join(", ")
}

/// Extract the text between the parameter parentheses.
///
/// For Go methods `func (recv) Name(params)`, returns the second paren group.
fn extract_params_text(text: &str, language: &str) -> String {
    if language == "go" {
        // Go methods: func (r *Recv) Name(params)
        // We need the parameter list after the function name, not the receiver
        let has_receiver = text.contains("func (") || text.contains("func(");
        if has_receiver {
            if let Some(first_close) = text.find(')') {
                let rest = &text[first_close + 1..];
                return extract_paren_content(rest);
            }
        }
    }

    extract_paren_content(text)
}

/// Extract content between first `(` and matching `)`.
fn extract_paren_content(text: &str) -> String {
    if let Some(open) = text.find('(') {
        // Find matching close, respecting nesting
        let mut depth = 0;
        for (i, ch) in text[open..].char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return text[open + 1..open + i].trim().to_string();
                    }
                }
                _ => {}
            }
        }
    }
    String::new()
}

/// Extract the parameter name from a typed parameter fragment.
///
/// Handles patterns like:
/// - `name: string` → `name` (TypeScript)
/// - `name String` → `name` (Go)
/// - `name: &str` → `name` (Rust)
/// - `name: str = "default"` → `name` (Python)
/// - `String name` → `name` (Java/C#)
fn extract_param_name(param: &str, language: &str) -> String {
    if param.is_empty() {
        return String::new();
    }

    match language {
        // Colon-separated: name comes before the colon
        "typescript" | "tsx" | "javascript" | "jsx" | "rust" | "python" | "dart" | "kotlin"
        | "swift" | "scala" => {
            let before_default = param.split('=').next().unwrap_or(param).trim();
            let name = before_default.split(':').next().unwrap_or(before_default).trim();
            // Strip leading modifiers (mut, ref, etc.)
            name.split_whitespace().last().unwrap_or(name).to_string()
        }
        // Type-first: type comes before name (Java, C#, C, C++, Go, PHP)
        "java" | "csharp" | "c#" | "c" | "cpp" | "c++" => {
            // Last whitespace-separated token is the name
            let name = param.split_whitespace().last().unwrap_or(param);
            // Strip pointer/reference decorators
            name.trim_start_matches('*')
                .trim_start_matches('&')
                .to_string()
        }
        "go" => {
            // Go: `name Type` — first token is the name
            param
                .split_whitespace()
                .next()
                .unwrap_or(param)
                .to_string()
        }
        "php" => {
            // PHP: `Type $name` or `$name`
            param
                .split_whitespace()
                .find(|t| t.starts_with('$'))
                .map(|s| s.trim_start_matches('$').to_string())
                .unwrap_or_else(|| {
                    param
                        .split_whitespace()
                        .last()
                        .unwrap_or(param)
                        .trim_start_matches('$')
                        .to_string()
                })
        }
        "ruby" => param.to_string(),
        _ => param.split(':').next().unwrap_or(param).trim().to_string(),
    }
}

/// Check if a parameter name is a language-specific "self" parameter.
fn is_self_param(name: &str, language: &str) -> bool {
    match language {
        "python" => name == "self" || name == "cls",
        "rust" => name == "self" || name == "&self" || name == "&mut self",
        _ => false,
    }
}

// ═══════════════════════════════════════════════════════════════
// Unit Tests
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Source extraction ───────────────────────────────────

    #[test]
    fn test_short_source_not_truncated() {
        let src = "fn short() { 42 }";
        let (result, trunc) = extract_source(src);
        assert_eq!(result, src);
        assert!(!trunc);
    }

    #[test]
    fn test_long_source_truncated_by_lines() {
        let lines: String = (0..200).map(|i| format!("line {i}\n")).collect();
        let (result, trunc) = extract_source(&lines);
        assert!(result.lines().count() <= MAX_SOURCE_LINES);
        assert!(trunc);
    }

    // ── Docstring extraction ───────────────────────────────

    #[test]
    fn test_jsdoc() {
        let text = "/** Hello world. */\nfunction f() {}";
        let doc = extract_docstring(text, "typescript");
        assert!(doc.contains("Hello world"));
    }

    #[test]
    fn test_rustdoc() {
        let text = "/// Finds nodes.\n/// Uses BFS.\nfn find() {}";
        let doc = extract_docstring(text, "rust");
        assert!(doc.contains("Finds nodes"));
    }

    #[test]
    fn test_python_docstring() {
        let text = "def f():\n    \"\"\"My doc.\"\"\"\n    pass";
        let doc = extract_docstring(text, "python");
        assert_eq!(doc, "My doc.");
    }

    // ── Decorator extraction ───────────────────────────────

    #[test]
    fn test_python_decorators() {
        let text = "@staticmethod\n@override\ndef f(): pass";
        let decs = extract_decorators(text, "python");
        assert!(decs.contains("@staticmethod"));
        assert!(decs.contains("@override"));
    }

    #[test]
    fn test_rust_attrs() {
        let text = "#[derive(Debug)]\npub fn f() {}";
        let decs = extract_decorators(text, "rust");
        assert!(decs.contains("#[derive(Debug)]"));
    }

    // ── Parameter extraction ───────────────────────────────

    #[test]
    fn test_ts_params() {
        let sig = "function f(a: string, b: number) {}";
        assert_eq!(extract_parameters(sig, "typescript"), "a, b");
    }

    #[test]
    fn test_rust_params_no_self() {
        let sig = "fn process(&self, name: String, age: i32) {}";
        assert_eq!(extract_parameters(sig, "rust"), "name, age");
    }

    #[test]
    fn test_python_params_no_self() {
        let sig = "def method(self, name: str, age: int):";
        assert_eq!(extract_parameters(sig, "python"), "name, age");
    }

    #[test]
    fn test_go_params_no_receiver() {
        let sig = "func (s *Server) Handle(ctx Context, req Request) {}";
        assert_eq!(extract_parameters(sig, "go"), "ctx, req");
    }
}
