//! Go AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls) from Go source files using
//! ast-grep pattern matching.
//!
//! Go import resolution: string paths in import declarations.
//! Local imports (starting with `.` or matching known project files) produce
//! Imports edges. External packages (github.com/*, stdlib) are filtered.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Go function declarations.
/// Each tuple: (pattern, has_receiver).
const GO_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("func $NAME($$$ARGS) $RET { $$$BODY }", false),
    ("func $NAME($$$ARGS) { $$$BODY }", false),
    ("func ($RECV) $NAME($$$ARGS) $RET { $$$BODY }", true),
    ("func ($RECV) $NAME($$$ARGS) { $$$BODY }", true),
];

/// ast-grep patterns for Go type declarations.
const GO_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("type $NAME struct { $$$FIELDS }", "struct_kind"),
    ("type $NAME interface { $$$METHODS }", "interface"),
];

/// Extract entities from Go source code.
///
/// Extracts File, Function, and Type nodes, plus Imports and Calls edges.
/// The `known_files` set is used for import resolution.
pub fn extract_go(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Go;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.ends_with("_test.go");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "go", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations → collect names
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations
    extract_types(&root, &file_slug, &mut entities);

    // Extract imports → Imports edges
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges
    extract_calls(source, &file_slug, &function_names, &import_map, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Go source.
///
/// Returns the set of function names found in this file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (pattern, _has_receiver) in GO_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "func ");
            // For methods: "func (r *Recv) Name(...)" — skip past the receiver
            let name = if name.starts_with('(') {
                if let Some(close) = matched_text.find(") ") {
                    let after = &matched_text[close + 2..];
                    helpers::extract_name_after_keyword(after, "")
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect()
                } else {
                    continue;
                }
            } else {
                name
            };

            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            let param_count = helpers::count_params_go(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false,
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
    seen_names
}

/// Extract type declarations from Go source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in GO_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "type ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = name.starts_with(|c: char| c.is_uppercase());

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}

/// Extract Go import statements and produce Imports edges.
///
/// Handles single imports (`import "path"`) and grouped imports (`import (...)`).
/// Local imports (starting with `.` or matching known project directories) produce
/// Imports edges. External packages are skipped.
fn extract_imports(
    source: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let import_map = HashMap::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // Match import "path" or just "path" inside import block
        let import_path = if trimmed.starts_with("import \"") {
            // Single import: import "path"
            extract_go_import_path(trimmed)
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            // Inside import(...) block
            trimmed.trim_matches('"').to_string()
        } else {
            continue;
        };

        if import_path.is_empty() {
            continue;
        }

        // Determine if this is a local import
        let is_local = import_path.starts_with('.')
            || is_go_local_import(&import_path, known_files);

        if is_local {
            // Resolve local import path
            let resolved = if import_path.starts_with('.') {
                // Relative: ./internal/util → internal/util (as directory)
                let clean = import_path.trim_start_matches("./");
                // Find any .go file in this directory from known_files
                let dir_prefix = format!("{clean}/");
                if let Some(first_file) = known_files.iter().find(|f| f.starts_with(&dir_prefix)) {
                    first_file.clone()
                } else {
                    format!("{clean}.go")
                }
            } else {
                format!("{import_path}.go")
            };

            edge_helpers::build_import_edge(
                file_slug,
                &import_path,
                &resolved,
                false,
                entities,
            );
        }
    }
    import_map
}

/// Check if a Go import path is local (not external package or stdlib).
///
/// A local import doesn't contain a domain name (no dots in first segment).
fn is_go_local_import(import_path: &str, _known_files: &HashSet<String>) -> bool {
    // External packages typically have domain: github.com/..., golang.org/...
    // Stdlib packages are single words: fmt, os, strings, etc.
    // Local packages start with ./ or ../ or are relative without dots
    !import_path.contains('.')
        && !is_go_stdlib_package(import_path)
}

/// Check if an import path is a Go standard library package.
fn is_go_stdlib_package(path: &str) -> bool {
    const GO_STDLIB: &[&str] = &[
        "fmt", "os", "io", "net", "http", "strings", "bytes", "bufio",
        "encoding", "crypto", "sync", "context", "errors", "flag",
        "log", "math", "path", "reflect", "regexp", "runtime", "sort",
        "strconv", "testing", "time", "unicode",
    ];
    let first_segment = path.split('/').next().unwrap_or(path);
    GO_STDLIB.contains(&first_segment)
}

/// Extract import path from a single Go import statement.
fn extract_go_import_path(line: &str) -> String {
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}

/// Extract Calls edges from Go function bodies.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Go;
    let root = lang.ast_grep(source);

    for (pattern, _) in GO_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let name = helpers::extract_name_after_keyword(&fn_text, "func ");
            let fn_name = if name.starts_with('(') {
                if let Some(close) = fn_text.find(") ") {
                    let after = &fn_text[close + 2..];
                    helpers::extract_name_after_keyword(after, "")
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect()
                } else {
                    continue;
                }
            } else {
                name
            };

            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            if let Some(body_start) = fn_text.find('{') {
                let body = &fn_text[body_start..];
                let mut callee_names = HashSet::new();
                edge_helpers::extract_call_names_from_body(body, &mut callee_names);

                edge_helpers::resolve_calls(
                    &caller_slug,
                    file_slug,
                    &callee_names,
                    &fn_name,
                    local_functions,
                    import_map,
                    entities,
                );
            }
        }
    }
}
