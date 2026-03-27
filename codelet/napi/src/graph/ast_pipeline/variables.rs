//! Variable Extraction Module
//!
//! Extracts module-level and class-level variable/constant declarations
//! from source code using ast-grep pattern matching. Function-local
//! variables are excluded to avoid noise.
//!
//! Provides a single `extract_variables()` function called by each
//! language extractor, similar to how `complexity.rs` and `metadata.rs`
//! provide shared extraction logic.
//!
//! CGC equivalent: Variable node with name/value/context/line_number +
//! find_by_variable_name() + who_modifies_variable().

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// Per-language variable extraction configuration.
struct VariableConfig {
    /// ast-grep patterns for variable declarations: (pattern, is_constant).
    patterns: &'static [(&'static str, bool)],
    /// ast-grep patterns for function declarations (for scope filtering).
    function_patterns: &'static [&'static str],
    /// ast-grep patterns for class/struct declarations (for class scope).
    class_patterns: &'static [&'static str],
    /// The ast-grep SupportLang for parsing.
    lang: SupportLang,
    /// Whether ALL_CAPS names imply constant (Python, Go, Ruby).
    all_caps_is_constant: bool,
}

// ── Variable Patterns ───────────────────────────────────────────

const TS_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const $NAME = $VALUE", true),
    ("const $NAME: $TYPE = $VALUE", true),
    ("let $NAME = $VALUE", false),
    ("let $NAME: $TYPE = $VALUE", false),
    ("var $NAME = $VALUE", false),
];

const PY_VAR_PATTERNS: &[(&str, bool)] = &[
    ("$NAME = $VALUE", false),
    ("$NAME: $TYPE = $VALUE", false),
];

const RUST_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const $NAME: $TYPE = $VALUE;", true),
    ("pub const $NAME: $TYPE = $VALUE;", true),
    ("static $NAME: $TYPE = $VALUE;", false),
    ("pub static $NAME: $TYPE = $VALUE;", false),
];

const GO_VAR_PATTERNS: &[(&str, bool)] = &[
    ("var $NAME = $VALUE", false),
    ("const $NAME = $VALUE", true),
];

const JAVA_VAR_PATTERNS: &[(&str, bool)] = &[
    ("static final $TYPE $NAME = $VALUE;", true),
    ("public static final $TYPE $NAME = $VALUE;", true),
    ("private static final $TYPE $NAME = $VALUE;", true),
    ("static $TYPE $NAME = $VALUE;", false),
    ("public static $TYPE $NAME = $VALUE;", false),
    ("final $TYPE $NAME = $VALUE;", true),
];

const CSHARP_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const $TYPE $NAME = $VALUE;", true),
    ("public const $TYPE $NAME = $VALUE;", true),
    ("static readonly $TYPE $NAME = $VALUE;", true),
    ("public static readonly $TYPE $NAME = $VALUE;", true),
    ("static $TYPE $NAME = $VALUE;", false),
];

const KOTLIN_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const val $NAME = $VALUE", true),
    ("val $NAME = $VALUE", true),
    ("var $NAME = $VALUE", false),
];

const SWIFT_VAR_PATTERNS: &[(&str, bool)] = &[
    ("let $NAME = $VALUE", true),
    ("let $NAME: $TYPE = $VALUE", true),
    ("var $NAME = $VALUE", false),
    ("static let $NAME = $VALUE", true),
];

const PHP_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const $NAME = $VALUE;", true),
    ("public const $NAME = $VALUE;", true),
];

const SCALA_VAR_PATTERNS: &[(&str, bool)] = &[
    ("val $NAME = $VALUE", true),
    ("var $NAME = $VALUE", false),
];

const C_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const $TYPE $NAME = $VALUE;", true),
    ("static const $TYPE $NAME = $VALUE;", true),
];

const CPP_VAR_PATTERNS: &[(&str, bool)] = &[
    ("const $TYPE $NAME = $VALUE;", true),
    ("constexpr $TYPE $NAME = $VALUE;", true),
    ("static const $TYPE $NAME = $VALUE;", true),
];

const RUBY_VAR_PATTERNS: &[(&str, bool)] = &[
    ("$NAME = $VALUE", false),
];

// ── Function Patterns (for scope filtering) ─────────────────────

const TS_FN_PATTERNS: &[&str] = &[
    "function $NAME($$$ARGS) { $$$BODY }",
    "function $NAME($$$ARGS): $RET { $$$BODY }",
];

const PY_FN_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS): $$$BODY",
];

const RUST_FN_PATTERNS: &[&str] = &[
    "fn $NAME($$$ARGS) { $$$BODY }",
    "fn $NAME($$$ARGS) -> $RET { $$$BODY }",
];

const GO_FN_PATTERNS: &[&str] = &[
    "func $NAME($$$ARGS) { $$$BODY }",
    "func $NAME($$$ARGS) $RET { $$$BODY }",
];

const JAVA_FN_PATTERNS: &[&str] = &[
    "$RET $NAME($$$ARGS) { $$$BODY }",
];

const CSHARP_FN_PATTERNS: &[&str] = &[
    "$RET $NAME($$$ARGS) { $$$BODY }",
];

const KOTLIN_FN_PATTERNS: &[&str] = &[
    "fun $NAME($$$ARGS) { $$$BODY }",
    "fun $NAME($$$ARGS): $RET { $$$BODY }",
];

const SWIFT_FN_PATTERNS: &[&str] = &[
    "func $NAME($$$ARGS) { $$$BODY }",
    "func $NAME($$$ARGS) -> $RET { $$$BODY }",
];

const PHP_FN_PATTERNS: &[&str] = &[
    "function $NAME($$$ARGS) { $$$BODY }",
];

const SCALA_FN_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS) = $BODY",
    "def $NAME($$$ARGS): $RET = $BODY",
];

const C_FN_PATTERNS: &[&str] = &[
    "$RET $NAME($$$ARGS) { $$$BODY }",
];

const RUBY_FN_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS) $$$BODY end",
];

// ── Class Patterns (for class scope detection) ──────────────────

const TS_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const PY_CLASS_PATTERNS: &[&str] = &["class $NAME: $$$BODY"];
const JAVA_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const CSHARP_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const KOTLIN_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const SWIFT_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const SCALA_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const PHP_CLASS_PATTERNS: &[&str] = &["class $NAME { $$$BODY }"];
const RUBY_CLASS_PATTERNS: &[&str] = &["class $NAME $$$BODY end"];
const EMPTY_PATTERNS: &[&str] = &[];

// ── Config Factory ──────────────────────────────────────────────

fn config_for_language(language: &str) -> Option<VariableConfig> {
    match language {
        "typescript" | "tsx" | "javascript" => Some(VariableConfig {
            patterns: TS_VAR_PATTERNS,
            function_patterns: TS_FN_PATTERNS,
            class_patterns: TS_CLASS_PATTERNS,
            lang: SupportLang::TypeScript,
            all_caps_is_constant: false,
        }),
        "python" => Some(VariableConfig {
            patterns: PY_VAR_PATTERNS,
            function_patterns: PY_FN_PATTERNS,
            class_patterns: PY_CLASS_PATTERNS,
            lang: SupportLang::Python,
            all_caps_is_constant: true,
        }),
        "rust" => Some(VariableConfig {
            patterns: RUST_VAR_PATTERNS,
            function_patterns: RUST_FN_PATTERNS,
            class_patterns: EMPTY_PATTERNS,
            lang: SupportLang::Rust,
            all_caps_is_constant: false,
        }),
        "go" => Some(VariableConfig {
            patterns: GO_VAR_PATTERNS,
            function_patterns: GO_FN_PATTERNS,
            class_patterns: EMPTY_PATTERNS,
            lang: SupportLang::Go,
            all_caps_is_constant: true,
        }),
        "java" => Some(VariableConfig {
            patterns: JAVA_VAR_PATTERNS,
            function_patterns: JAVA_FN_PATTERNS,
            class_patterns: JAVA_CLASS_PATTERNS,
            lang: SupportLang::Java,
            all_caps_is_constant: false,
        }),
        "csharp" => Some(VariableConfig {
            patterns: CSHARP_VAR_PATTERNS,
            function_patterns: CSHARP_FN_PATTERNS,
            class_patterns: CSHARP_CLASS_PATTERNS,
            lang: SupportLang::CSharp,
            all_caps_is_constant: false,
        }),
        "kotlin" => Some(VariableConfig {
            patterns: KOTLIN_VAR_PATTERNS,
            function_patterns: KOTLIN_FN_PATTERNS,
            class_patterns: KOTLIN_CLASS_PATTERNS,
            lang: SupportLang::Kotlin,
            all_caps_is_constant: false,
        }),
        "swift" => Some(VariableConfig {
            patterns: SWIFT_VAR_PATTERNS,
            function_patterns: SWIFT_FN_PATTERNS,
            class_patterns: SWIFT_CLASS_PATTERNS,
            lang: SupportLang::Swift,
            all_caps_is_constant: false,
        }),
        "php" => Some(VariableConfig {
            patterns: PHP_VAR_PATTERNS,
            function_patterns: PHP_FN_PATTERNS,
            class_patterns: PHP_CLASS_PATTERNS,
            lang: SupportLang::Php,
            all_caps_is_constant: false,
        }),
        "scala" => Some(VariableConfig {
            patterns: SCALA_VAR_PATTERNS,
            function_patterns: SCALA_FN_PATTERNS,
            class_patterns: SCALA_CLASS_PATTERNS,
            lang: SupportLang::Scala,
            all_caps_is_constant: false,
        }),
        "c" => Some(VariableConfig {
            patterns: C_VAR_PATTERNS,
            function_patterns: C_FN_PATTERNS,
            class_patterns: EMPTY_PATTERNS,
            lang: SupportLang::C,
            all_caps_is_constant: false,
        }),
        "cpp" => Some(VariableConfig {
            patterns: CPP_VAR_PATTERNS,
            function_patterns: C_FN_PATTERNS,
            class_patterns: EMPTY_PATTERNS,
            lang: SupportLang::Cpp,
            all_caps_is_constant: false,
        }),
        "ruby" => Some(VariableConfig {
            patterns: RUBY_VAR_PATTERNS,
            function_patterns: RUBY_FN_PATTERNS,
            class_patterns: RUBY_CLASS_PATTERNS,
            lang: SupportLang::Ruby,
            all_caps_is_constant: true,
        }),
        _ => None,
    }
}

/// Line range for a named scope (function or class).
struct ScopeRange {
    name: String,
    start: i32,
    end: i32,
}

/// Check if a name is ALL_CAPS (Python/Ruby/Go convention for constants).
fn is_all_caps(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
        && name.chars().any(|c| c.is_ascii_uppercase())
}

/// Extract value from matched text by finding `= value`.
fn extract_value(text: &str) -> String {
    if let Some(eq_pos) = text.find(" = ") {
        let val = text[eq_pos + 3..].trim();
        val.trim_end_matches(';').trim().to_string()
    } else if let Some(eq_pos) = text.find('=') {
        let val = text[eq_pos + 1..].trim();
        val.trim_end_matches(';').trim().to_string()
    } else {
        String::new()
    }
}

/// Find line ranges of all functions in the source (for filtering locals).
fn find_function_ranges(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    config: &VariableConfig,
) -> Vec<ScopeRange> {
    let mut ranges = Vec::new();
    for pattern in config.function_patterns {
        for node in root.root().find_all(*pattern) {
            let start_line = node.start_pos().line() as i32 + 1;
            let end_line = node.end_pos().line() as i32 + 1;
            let name = node
                .get_env()
                .get_match("NAME")
                .map(|n| n.text().to_string())
                .unwrap_or_default();
            ranges.push(ScopeRange { name, start: start_line, end: end_line });
        }
    }
    ranges
}

/// Find line ranges of all classes in the source (for class scope).
fn find_class_ranges(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    config: &VariableConfig,
) -> Vec<ScopeRange> {
    let mut ranges = Vec::new();
    for pattern in config.class_patterns {
        for node in root.root().find_all(*pattern) {
            let start_line = node.start_pos().line() as i32 + 1;
            let end_line = node.end_pos().line() as i32 + 1;
            let name = node
                .get_env()
                .get_match("NAME")
                .map(|n| n.text().to_string())
                .unwrap_or_default();
            ranges.push(ScopeRange { name, start: start_line, end: end_line });
        }
    }
    ranges
}

/// Extract variables from source code for a given language.
///
/// Self-contained: finds function/class ranges internally, then extracts
/// variable declarations that are NOT inside function bodies.
///
/// # Arguments
/// * `source` — full source code
/// * `file_slug` — slugified file path (for node slugs)
/// * `rel_path` — relative file path (for the path property)
/// * `language` — language identifier (e.g. "typescript", "python")
/// * `entities` — output vector to push extracted entities into
pub fn extract_variables(
    source: &str,
    file_slug: &str,
    rel_path: &str,
    language: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let config = match config_for_language(language) {
        Some(c) => c,
        None => return,
    };

    let root = config.lang.ast_grep(source);
    let fn_ranges = find_function_ranges(&root, &config);
    let class_ranges = find_class_ranges(&root, &config);
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for &(pattern, pattern_is_const) in config.patterns {
        for node in root.root().find_all(pattern) {
            let name = match node.get_env().get_match("NAME") {
                Some(n) => n.text().to_string(),
                None => continue,
            };

            // Skip non-variable identifiers
            if name.is_empty()
                || name == "self"
                || name == "cls"
                || name == "this"
                || name == "_"
                || (name.starts_with("__") && name.ends_with("__"))
            {
                continue;
            }

            let line = node.start_pos().line() as i32 + 1;

            // Skip function-local variables
            if fn_ranges.iter().any(|r| line > r.start && line < r.end) {
                continue;
            }

            // De-duplicate
            let dedup_key = format!("{name}:{line}");
            if !seen.insert(dedup_key) {
                continue;
            }

            // Determine scope
            let (scope, scope_name) =
                if let Some(cr) = class_ranges.iter().find(|r| line >= r.start && line <= r.end) {
                    ("class", cr.name.as_str())
                } else {
                    ("module", "")
                };

            let matched_text = node.text().to_string();
            let value = extract_value(&matched_text);
            let is_constant =
                pattern_is_const || (config.all_caps_is_constant && is_all_caps(&name));

            let var_node = helpers::build_variable_node(
                file_slug, &name, rel_path, line, &value, scope, scope_name, is_constant, language,
            );
            let var_slug = match &var_node {
                GraphEntity::Node { slug, .. } => slug.clone(),
                _ => continue,
            };
            entities.push(var_node);
            entities.push(helpers::build_contains_variable_edge(file_slug, &var_slug));
        }
    }
}
