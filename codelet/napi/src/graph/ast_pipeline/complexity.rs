//! Cyclomatic Complexity Calculator
//!
//! Language-agnostic cyclomatic complexity calculation from function source text.
//! Single entry point `calculate()` handles all 14 supported languages via
//! data-driven keyword configuration.
//!
//! CGC equivalent: `_calculate_complexity()` methods in each language parser.
//!
//! Formula: `1 + count(decision_points)`
//! Decision points: `if`, `else if`, `for`, `while`, `case`, `catch`, `&&`, `||`, etc.

/// Calculate cyclomatic complexity from function source text.
///
/// Returns `1 + number_of_decision_points`. A function with no branches
/// has complexity 1.
///
/// # Arguments
/// * `source` - The full source text of the function (including signature)
/// * `language` - Language identifier (e.g., "typescript", "python", "rust")
pub fn calculate(source: &str, language: &str) -> i32 {
    let config = config_for_language(language);
    let stripped = strip_for_complexity(source, config.comment_style);
    1 + count_decision_points(&stripped, config)
}

/// Comment style determines how to strip non-code text.
#[derive(Clone, Copy)]
enum CommentStyle {
    /// C-family: `//` line comments, `/* */` block comments
    CStyle,
    /// Python: `#` line comments, triple-quoted strings
    Hash,
    /// Ruby: `#` line comments, `=begin`/`=end` blocks
    HashBlock,
}

/// Per-language configuration for decision point detection.
struct ComplexityConfig {
    /// Keywords matched with word boundaries (e.g., `\bif\b`)
    keywords: &'static [&'static str],
    /// Operator literals matched exactly (e.g., `&&`, `||`)
    operators: &'static [&'static str],
    /// Comment style for stripping
    comment_style: CommentStyle,
    /// Whether to count `=>` as match arms (Rust-specific)
    count_match_arms: bool,
}

// ── Language Configurations ───────────────────────────────────

const TS_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "do", "case", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const RUST_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "loop"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: true,
};

const PYTHON_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "elif", "for", "while", "except"],
    operators: &[" and ", " or "],
    comment_style: CommentStyle::Hash,
    count_match_arms: false,
};

const GO_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "case", "select"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const JAVA_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "do", "case", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const C_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "do", "case", "goto"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const CPP_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "do", "case", "catch", "goto"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const CSHARP_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "do", "case", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const RUBY_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "unless", "case", "when", "while", "until", "for", "rescue"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::HashBlock,
    count_match_arms: false,
};

const KOTLIN_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "when", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const SWIFT_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "case", "catch", "guard"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const SCALA_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "case", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const PHP_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "elseif", "for", "while", "do", "case", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

const DART_CONFIG: ComplexityConfig = ComplexityConfig {
    keywords: &["if", "for", "while", "do", "case", "catch"],
    operators: &["&&", "||"],
    comment_style: CommentStyle::CStyle,
    count_match_arms: false,
};

/// Get the complexity config for a language identifier.
fn config_for_language(language: &str) -> &'static ComplexityConfig {
    match language {
        "typescript" | "tsx" | "javascript" | "jsx" => &TS_CONFIG,
        "rust" => &RUST_CONFIG,
        "python" => &PYTHON_CONFIG,
        "go" => &GO_CONFIG,
        "java" => &JAVA_CONFIG,
        "c" => &C_CONFIG,
        "cpp" | "c++" => &CPP_CONFIG,
        "csharp" | "c#" => &CSHARP_CONFIG,
        "ruby" => &RUBY_CONFIG,
        "kotlin" => &KOTLIN_CONFIG,
        "swift" => &SWIFT_CONFIG,
        "scala" => &SCALA_CONFIG,
        "php" => &PHP_CONFIG,
        "dart" => &DART_CONFIG,
        // Default: C-family keywords as a reasonable fallback
        _ => &TS_CONFIG,
    }
}

// ── Source Text Stripping ─────────────────────────────────────

/// Strip comments and string literals from source to avoid false keyword matches.
fn strip_for_complexity(source: &str, style: CommentStyle) -> String {
    match style {
        CommentStyle::CStyle => strip_c_style(source),
        CommentStyle::Hash => strip_hash_style(source),
        CommentStyle::HashBlock => strip_hash_block_style(source),
    }
}

/// Strip C-style comments (`//`, `/* */`) and string literals.
fn strip_c_style(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Line comment — skip to end of line
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
        } else if bytes[i] == b'"' || bytes[i] == b'\'' || bytes[i] == b'`' {
            let quote = bytes[i];
            result.push(' ');
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Strip hash-style comments (`#`) and triple-quoted strings.
fn strip_hash_style(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Triple-quoted string
        if i + 2 < len
            && ((bytes[i] == b'"' && bytes[i + 1] == b'"' && bytes[i + 2] == b'"')
                || (bytes[i] == b'\'' && bytes[i + 1] == b'\'' && bytes[i + 2] == b'\''))
        {
            let quote = bytes[i];
            result.push(' ');
            i += 3;
            while i + 2 < len
                && !(bytes[i] == quote && bytes[i + 1] == quote && bytes[i + 2] == quote)
            {
                i += 1;
            }
            if i + 2 < len {
                i += 3;
            }
        } else if bytes[i] == b'#' {
            // Line comment
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            result.push(' ');
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Strip Ruby-style comments (`#` and `=begin`/`=end` blocks).
fn strip_hash_block_style(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let lines: Vec<&str> = source.lines().collect();
    let mut in_block = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed == "=begin" {
            in_block = true;
            continue;
        }
        if trimmed == "=end" {
            in_block = false;
            continue;
        }
        if in_block {
            continue;
        }
        // Strip inline # comments (but not inside strings — best effort)
        if let Some(hash_pos) = line.find('#') {
            result.push_str(&line[..hash_pos]);
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}

// ── Decision Point Counting ───────────────────────────────────

/// Count decision points in stripped source text.
fn count_decision_points(stripped: &str, config: &ComplexityConfig) -> i32 {
    let mut count = 0i32;

    // Count keyword occurrences with word-boundary matching
    for keyword in config.keywords {
        count += count_keyword_occurrences(stripped, keyword);
    }

    // Count operator occurrences (literal match)
    for operator in config.operators {
        count += count_operator_occurrences(stripped, operator);
    }

    // Rust-specific: count match arms (=>)
    if config.count_match_arms {
        count += count_match_arms(stripped);
    }

    count
}

/// Count word-boundary-aware keyword occurrences.
///
/// A keyword matches when preceded and followed by a non-alphanumeric/underscore char.
fn count_keyword_occurrences(text: &str, keyword: &str) -> i32 {
    let mut count = 0i32;
    let kw_bytes = keyword.as_bytes();
    let kw_len = kw_bytes.len();
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();

    if text_len < kw_len {
        return 0;
    }

    let mut i = 0;
    while i + kw_len <= text_len {
        if &text_bytes[i..i + kw_len] == kw_bytes {
            // Check word boundary before
            let before_ok = i == 0 || !is_word_char(text_bytes[i - 1]);
            // Check word boundary after
            let after_ok =
                i + kw_len >= text_len || !is_word_char(text_bytes[i + kw_len]);
            if before_ok && after_ok {
                count += 1;
                i += kw_len;
                continue;
            }
        }
        i += 1;
    }

    count
}

/// Count literal operator occurrences in text.
fn count_operator_occurrences(text: &str, operator: &str) -> i32 {
    text.matches(operator).count() as i32
}

/// Count Rust match arms by counting `=>` that aren't `>=` or part of `->`.
fn count_match_arms(text: &str) -> i32 {
    let mut count = 0i32;
    let bytes = text.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i + 1 < len {
        if bytes[i] == b'=' && bytes[i + 1] == b'>' {
            // Make sure it's not preceded by `>`, `<`, `!`, or `-`
            let ok = i == 0 || !matches!(bytes[i - 1], b'>' | b'<' | b'!' | b'-');
            if ok {
                count += 1;
            }
            i += 2;
        } else {
            i += 1;
        }
    }

    count
}

/// Check if a byte is a word character (alphanumeric or underscore).
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_complexity() {
        assert_eq!(calculate("fn noop() {}", "rust"), 1);
        assert_eq!(calculate("function empty() {}", "typescript"), 1);
        assert_eq!(calculate("def pass_fn():\n    pass", "python"), 1);
    }

    #[test]
    fn test_keyword_in_string_not_counted() {
        let source = r#"function test() { return "if while for"; }"#;
        assert_eq!(calculate(source, "typescript"), 1);
    }

    #[test]
    fn test_keyword_in_comment_not_counted() {
        let source = "function test() {\n  // if while for\n  return 1;\n}";
        assert_eq!(calculate(source, "typescript"), 1);
    }

    #[test]
    fn test_python_hash_comment_stripped() {
        let source = "def test():\n    # if for while\n    return 1";
        assert_eq!(calculate(source, "python"), 1);
    }

    #[test]
    fn test_else_if_not_double_counted() {
        // "else if" should count as 1 decision point (the `if`), not 2
        let source = "function f(x) { if (x > 0) {} else if (x < 0) {} }";
        assert_eq!(calculate(source, "typescript"), 3); // 1 + if + if (from else if)
    }

    #[test]
    fn test_no_partial_keyword_match() {
        // "elif" should not trigger "if" match, "iffy" should not trigger "if"
        let source = "def f():\n    iffy = True\n    return iffy";
        assert_eq!(calculate(source, "python"), 1);
    }
}
