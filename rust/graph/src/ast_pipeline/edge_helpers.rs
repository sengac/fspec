#![allow(clippy::too_many_arguments, clippy::type_complexity)]
//! Shared edge extraction helpers for cross-language Imports, Calls, and TypeRef.
//!
//! Provides reusable functions for extracting edge relationships that are common
//! across multiple language extractors. Each language extractor calls these helpers
//! with language-specific AST node kinds and resolution logic.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use crate::graph_entities::GraphEntity;

/// Build an Imports edge from a source file to a target file.
///
/// Also creates a stub File node for the target (will be merged/deduplicated
/// if a full File node exists from walking that file).
pub fn build_import_edge(
    file_slug: &str,
    import_path: &str,
    resolved_path: &str,
    is_type_only: bool,
    entities: &mut Vec<GraphEntity>,
) {
    let target_slug = super::helpers::slugify_path(resolved_path);

    // Stub File node for import target
    let mut target_props = Map::new();
    target_props.insert("slug".to_string(), Value::String(target_slug.clone()));
    target_props.insert("path".to_string(), Value::String(resolved_path.to_string()));
    entities.push(GraphEntity::Node {
        node_type: "File".to_string(),
        slug: target_slug.clone(),
        properties: target_props,
    });

    // Imports edge
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

/// Build a Calls edge from one function to another.
pub fn build_calls_edge(from_fn_slug: &str, to_fn_slug: &str, entities: &mut Vec<GraphEntity>) {
    entities.push(GraphEntity::Edge {
        edge_type: "Calls".to_string(),
        from_slug: from_fn_slug.to_string(),
        to_slug: to_fn_slug.to_string(),
        properties: Map::new(),
    });
}

/// Build a TypeRef edge from a function to a type.
pub fn build_typeref_edge(from_fn_slug: &str, to_type_slug: &str, entities: &mut Vec<GraphEntity>) {
    entities.push(GraphEntity::Edge {
        edge_type: "TypeRef".to_string(),
        from_slug: from_fn_slug.to_string(),
        to_slug: to_type_slug.to_string(),
        properties: Map::new(),
    });
}

/// Extract bare function call names from a function body string.
///
/// Finds patterns like `identifier(` but NOT `something.identifier(` (method calls).
/// Also skips `new Identifier(`, keywords like `if`, `for`, etc.
///
/// Shared across languages that use C-like call syntax (PHP, Java, C#, etc.).
pub fn extract_call_names_from_body(body: &str, out: &mut HashSet<String>) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip strings (single-quoted, double-quoted, backtick)
        if bytes[i] == b'\'' || bytes[i] == b'"' || bytes[i] == b'`' {
            let quote = bytes[i];
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }

        // Skip line comments
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Skip block comments
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // Skip # comments (PHP, Python, Ruby)
        if bytes[i] == b'#' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Look for identifier followed by (
        if is_ident_start(bytes[i]) {
            let start = i;
            while i < len && is_ident_char(bytes[i]) {
                i += 1;
            }
            let name = &body[start..i];

            // Skip whitespace between identifier and (
            let mut j = i;
            while j < len && bytes[j] == b' ' {
                j += 1;
            }

            if j < len && bytes[j] == b'(' {
                // Not preceded by `.` or `->` (method call) or `::` (static call)
                let not_method = start == 0
                    || (bytes[start - 1] != b'.'
                        && bytes[start - 1] != b'>'
                        && bytes[start - 1] != b':');
                // Not `new Something(`
                let not_constructor = if start >= 4 {
                    body.get(start - 4..start) != Some("new ")
                } else {
                    true
                };
                let not_keyword = !matches!(
                    name,
                    "if" | "for"
                        | "while"
                        | "switch"
                        | "catch"
                        | "return"
                        | "typeof"
                        | "instanceof"
                        | "await"
                        | "function"
                        | "foreach"
                        | "match"
                        | "echo"
                        | "print"
                        | "array"
                        | "list"
                        | "empty"
                        | "isset"
                        | "unset"
                        | "die"
                        | "exit"
                );

                if not_method && not_constructor && not_keyword {
                    out.insert(name.to_string());
                }
            }
            continue;
        }
        i += 1;
    }
}

/// Extract constructor target names from `new ClassName(` patterns.
///
/// In Java/C#/C++, `new Foo(...)` is a type reference, not a function call.
/// Returns the class names that follow `new`.
pub fn extract_constructor_names_from_body(body: &str, out: &mut HashSet<String>) {
    let search = "new ";
    let mut pos = 0;
    let bytes = body.as_bytes();
    let len = bytes.len();

    while let Some(found) = body[pos..].find(search) {
        let abs = pos + found + search.len();
        // Check the identifier starts here
        if abs < len && is_ident_start(bytes[abs]) {
            let start = abs;
            let mut end = abs;
            while end < len && is_ident_char(bytes[end]) {
                end += 1;
            }
            let name = &body[start..end];
            // Skip whitespace
            let mut j = end;
            while j < len && bytes[j] == b' ' {
                j += 1;
            }
            // Must be followed by (
            if j < len && bytes[j] == b'(' {
                // Must start with uppercase (class convention)
                let first = name.chars().next().unwrap_or('a');
                if first.is_uppercase() {
                    out.insert(name.to_string());
                }
            }
            pos = end;
        } else {
            pos = abs;
        }
    }
}

/// Extract `$this->method()` and `self::method()` calls from a PHP/C#/Java method body.
///
/// Returns the method names (without the `$this->` / `self::` prefix).
pub fn extract_member_call_names(body: &str, out: &mut HashSet<String>) {
    // Match $this->identifier( patterns
    for pattern in &["$this->", "self::", "static::"] {
        let mut search_start = 0;
        while let Some(pos) = body[search_start..].find(pattern) {
            let abs_pos = search_start + pos + pattern.len();
            let bytes = body.as_bytes();
            if abs_pos < bytes.len() && is_ident_start(bytes[abs_pos]) {
                let name_start = abs_pos;
                let mut end = abs_pos;
                while end < bytes.len() && is_ident_char(bytes[end]) {
                    end += 1;
                }
                // Skip whitespace
                let mut j = end;
                while j < bytes.len() && bytes[j] == b' ' {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'(' {
                    out.insert(body[name_start..end].to_string());
                }
            }
            search_start = abs_pos;
        }
    }
}

/// Resolve calls against known local functions and import map.
///
/// For each callee name, checks:
/// 1. If it's a known function in the same file → local Calls edge
/// 2. If it's an imported function from a relative import → cross-file Calls edge
/// 3. If it's a known TYPE (local or imported) → TypeRef edge instead of Calls
///
/// The `local_types` set prevents emitting dangling Calls edges when a class
/// is called as a constructor (e.g. `OptionParser(ctx)` in Python).
///
/// Returns nothing — emits edges directly into the entities vector.
pub fn resolve_calls(
    caller_slug: &str,
    file_slug: &str,
    callee_names: &HashSet<String>,
    caller_fn_name: &str,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    for callee_name in callee_names {
        if callee_name == caller_fn_name {
            continue; // skip self-recursion
        }

        // Check local entities first
        if local_functions.contains(callee_name.as_str()) {
            let target_slug = format!("{file_slug}::{callee_name}");
            build_calls_edge(caller_slug, &target_slug, entities);
        } else if local_types.contains(callee_name.as_str()) {
            // Constructor call to a local class → TypeRef, not Calls
            let target_slug = format!("{file_slug}::{callee_name}");
            build_typeref_edge(caller_slug, &target_slug, entities);
        } else if let Some((target_file_slug, is_relative, original_name)) =
            import_map.get(callee_name.as_str())
        {
            if *is_relative {
                let target_slug = format!("{target_file_slug}::{original_name}");
                // Check if the original name starts with uppercase → likely a class/type
                // In Python and Java, classes follow PascalCase convention.
                // Strip leading underscores first: _OptionParser → O is uppercase → Type
                let stripped = original_name.trim_start_matches('_');
                let first_char = stripped.chars().next().unwrap_or('a');
                if first_char.is_uppercase() {
                    build_typeref_edge(caller_slug, &target_slug, entities);
                } else {
                    build_calls_edge(caller_slug, &target_slug, entities);
                }
            }
        }
    }
}

/// Resolve type refs against known local types and import map.
///
/// Similar to resolve_calls but for TypeRef edges.
pub fn resolve_type_refs(
    fn_slug: &str,
    file_slug: &str,
    type_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    for type_name in type_names {
        if local_types.contains(type_name.as_str()) {
            let target_slug = format!("{file_slug}::{type_name}");
            build_typeref_edge(fn_slug, &target_slug, entities);
        } else if let Some((target_file_slug, is_relative, original_name)) =
            import_map.get(type_name.as_str())
        {
            if *is_relative {
                let target_slug = format!("{target_file_slug}::{original_name}");
                build_typeref_edge(fn_slug, &target_slug, entities);
            }
        }
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Extract local `#include "file.h"` directives from C/C++ source.
///
/// Only processes quoted includes (not angle-bracket system includes).
/// Resolves against known_files and emits Imports edges.
///
/// Returns a map of `header_basename → (target_file_slug, is_local, original_name)`.
pub fn extract_c_includes(
    source: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let mut import_map = HashMap::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("#include \"") {
            continue;
        }

        // Extract the path between quotes
        let after_include = &trimmed[10..]; // skip `#include "`
        if let Some(end_quote) = after_include.find('"') {
            let header_path = &after_include[..end_quote];
            if header_path.is_empty() {
                continue;
            }

            let is_local = known_files.contains(header_path);
            if is_local {
                let local_name = header_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(header_path)
                    .to_string();

                let target_slug = super::helpers::slugify_path(header_path);
                import_map.insert(local_name.clone(), (target_slug.clone(), true, local_name));

                build_import_edge(file_slug, header_path, header_path, false, entities);
            }
        }
    }
    import_map
}
