//! TypeScript/JavaScript AST Extractor
//!
//! Extracts Function nodes, File nodes, and relationship edges
//! (Contains, Imports) from TypeScript/JavaScript source files
//! using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};
use serde_json::{Map, Value};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for TypeScript function declarations.
///
/// Two patterns cover all cases:
/// - Functions with a return type annotation: `function name(args): RetType { body }`
/// - Functions without a return type: `function name(args) { body }`
///
/// Both patterns match async functions (async is a modifier on the AST node).
/// Export status is determined by checking the parent node text.
const TS_FUNCTION_PATTERNS: &[&str] = &[
    "function $NAME($$$ARGS): $RET { $$$BODY }",
    "function $NAME($$$ARGS) { $$$BODY }",
];

/// ast-grep pattern for TypeScript import statements.
const TS_IMPORT_PATTERN: &str = "import $$$IMPORTS from $SOURCE";

/// Extract entities from TypeScript/JavaScript source code.
pub fn extract_typescript(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::TypeScript;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    // Create File node
    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test")
        || rel_path.contains("spec")
        || rel_path.contains("__tests__");
    let language = if rel_path.ends_with(".tsx") || rel_path.ends_with(".jsx") {
        "tsx"
    } else {
        "typescript"
    };

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, language, line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations
    extract_functions(&root, &file_slug, &mut entities);

    // Extract import statements
    extract_imports(&root, &file_slug, rel_path, &mut entities);

    Ok(entities)
}

/// Extract function declarations from TypeScript source using multiple patterns.
///
/// Uses two patterns to catch both typed and untyped function declarations.
/// Deduplicates by function name since a function can only appear once per file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in TS_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = extract_function_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.contains("async ");
            let is_public = node
                .parent()
                .is_some_and(|p| p.text().starts_with("export "));
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                is_async,
                is_public,
                param_count,
                start_pos.line() as i32 + 1,
                end_pos.line() as i32 + 1,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &fn_slug,
                "Contains",
            ));
        }
    }
}

/// Extract import statements from TypeScript source.
fn extract_imports(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    rel_path: &str,
    entities: &mut Vec<GraphEntity>,
) {
    for node in root.root().find_all(TS_IMPORT_PATTERN) {
        let matched_text = node.text();
        let import_path = extract_import_path(&matched_text);
        if import_path.is_empty() {
            continue;
        }

        // Resolve relative import to a file path
        let resolved = resolve_import_path(rel_path, &import_path);
        let target_slug = helpers::slugify_path(&resolved);

        // Create target File node (may be merged if already exists in graph)
        let mut target_props = Map::new();
        target_props.insert("slug".to_string(), Value::String(target_slug.clone()));
        target_props.insert("path".to_string(), Value::String(resolved.clone()));
        entities.push(GraphEntity::Node {
            node_type: "File".to_string(),
            slug: target_slug.clone(),
            properties: target_props,
        });

        // Imports edge
        let is_type_only = matched_text.contains("import type ");
        let mut edge_props = Map::new();
        edge_props.insert(
            "importPath".to_string(),
            Value::String(import_path.to_string()),
        );
        edge_props.insert("isTypeOnly".to_string(), Value::Bool(is_type_only));
        entities.push(GraphEntity::Edge {
            edge_type: "Imports".to_string(),
            from_slug: file_slug.to_string(),
            to_slug: target_slug,
            properties: edge_props,
        });
    }
}

/// Extract function name from matched text.
///
/// Handles both `function login(...)` and `async function login(...)`.
fn extract_function_name(text: &str) -> String {
    super::helpers::extract_name_after_keyword(text, "function ")
}

/// Extract the import path string from an import statement.
fn extract_import_path(text: &str) -> String {
    if let Some(from_pos) = text.find("from ") {
        let after = &text[from_pos + 5..];
        let quote_char = after.chars().next().unwrap_or(' ');
        if quote_char == '\'' || quote_char == '"' {
            let inner = &after[1..];
            if let Some(end) = inner.find(quote_char) {
                return inner[..end].to_string();
            }
        }
    }
    String::new()
}

/// Resolve a relative import path to a file path.
fn resolve_import_path(source_file: &str, import_path: &str) -> String {
    if !import_path.starts_with('.') {
        return import_path.to_string();
    }

    let source_dir = if let Some(pos) = source_file.rfind('/') {
        &source_file[..pos]
    } else {
        ""
    };

    let mut parts: Vec<&str> = if source_dir.is_empty() {
        vec![]
    } else {
        source_dir.split('/').collect()
    };

    for segment in import_path.split('/') {
        match segment {
            "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(segment),
        }
    }

    let mut resolved = parts.join("/");
    if !resolved.ends_with(".ts")
        && !resolved.ends_with(".tsx")
        && !resolved.ends_with(".js")
        && !resolved.ends_with(".jsx")
    {
        resolved.push_str(".ts");
    }
    resolved
}
