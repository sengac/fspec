#![allow(clippy::too_many_arguments, clippy::type_complexity)]
//! Shared helpers for AST extractors.
//!
//! Common utility functions used by all language extractors
//! (TypeScript, Rust, Python, Go, Java, C, C++, C#, Ruby, Kotlin,
//! Swift, Scala, PHP) to build graph entities consistently.

use serde_json::{Map, Value};

use crate::graph_entities::GraphEntity;

/// Slugify a relative file path for use as a graph node key.
///
/// Replaces path separators and dots with hyphens.
pub fn slugify_path(path: &str) -> String {
    path.replace(['/', '\\'], "-")
        .replace('.', "-")
        .trim_matches('-')
        .to_string()
}

/// Build a File node with standard properties.
pub fn build_file_node(
    rel_path: &str,
    file_slug: &str,
    language: &str,
    line_count: i32,
    is_test: bool,
) -> GraphEntity {
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(file_slug.to_string()));
    props.insert("path".to_string(), Value::String(rel_path.to_string()));
    props.insert("language".to_string(), Value::String(language.to_string()));
    props.insert("lineCount".to_string(), Value::Number(line_count.into()));
    props.insert("isTest".to_string(), Value::Bool(is_test));
    GraphEntity::Node {
        node_type: "File".to_string(),
        slug: file_slug.to_string(),
        properties: props,
    }
}

/// Build a Function node with standard properties.
pub fn build_function_node(
    file_slug: &str,
    name: &str,
    is_async: bool,
    is_public: bool,
    param_count: i32,
    line_start: i32,
    line_end: i32,
    cyclomatic_complexity: i32,
    parameters: &str,
    source: &str,
    docstring: &str,
    decorators: &str,
    language: &str,
    truncated: bool,
) -> GraphEntity {
    let fn_slug = format!("{file_slug}::{name}");
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(fn_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("qualifiedName".to_string(), Value::String(fn_slug.clone()));
    props.insert("isAsync".to_string(), Value::Bool(is_async));
    props.insert("isPublic".to_string(), Value::Bool(is_public));
    props.insert("paramCount".to_string(), Value::Number(param_count.into()));
    props.insert("lineStart".to_string(), Value::Number(line_start.into()));
    props.insert("lineEnd".to_string(), Value::Number(line_end.into()));
    props.insert(
        "cyclomaticComplexity".to_string(),
        Value::Number(cyclomatic_complexity.into()),
    );
    props.insert(
        "parameters".to_string(),
        Value::String(parameters.to_string()),
    );
    props.insert("source".to_string(), Value::String(source.to_string()));
    props.insert(
        "docstring".to_string(),
        Value::String(docstring.to_string()),
    );
    props.insert(
        "decorators".to_string(),
        Value::String(decorators.to_string()),
    );
    props.insert("language".to_string(), Value::String(language.to_string()));
    props.insert("truncated".to_string(), Value::Bool(truncated));
    GraphEntity::Node {
        node_type: "Function".to_string(),
        slug: fn_slug,
        properties: props,
    }
}

/// Build a Contains edge from a File to a child entity.
pub fn build_contains_edge(file_slug: &str, child_slug: &str, edge_type: &str) -> GraphEntity {
    GraphEntity::Edge {
        edge_type: edge_type.to_string(),
        from_slug: file_slug.to_string(),
        to_slug: child_slug.to_string(),
        properties: Map::new(),
    }
}

/// Build a Type node with standard properties.
pub fn build_type_node(
    file_slug: &str,
    name: &str,
    type_kind: &str,
    is_public: bool,
    line_start: i32,
    line_end: i32,
    source: &str,
    docstring: &str,
    decorators: &str,
    language: &str,
    truncated: bool,
) -> GraphEntity {
    let type_slug = format!("{file_slug}::{name}");
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(type_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("typeKind".to_string(), Value::String(type_kind.to_string()));
    props.insert("isPublic".to_string(), Value::Bool(is_public));
    props.insert("lineStart".to_string(), Value::Number(line_start.into()));
    props.insert("lineEnd".to_string(), Value::Number(line_end.into()));
    props.insert("source".to_string(), Value::String(source.to_string()));
    props.insert(
        "docstring".to_string(),
        Value::String(docstring.to_string()),
    );
    props.insert(
        "decorators".to_string(),
        Value::String(decorators.to_string()),
    );
    props.insert("language".to_string(), Value::String(language.to_string()));
    props.insert("truncated".to_string(), Value::Bool(truncated));
    GraphEntity::Node {
        node_type: "Type".to_string(),
        slug: type_slug,
        properties: props,
    }
}

/// Build a Variable node with standard properties.
///
/// For module-level variables: `scope` is "module", `scope_name` is "".
/// For class-level variables: `scope` is "class", `scope_name` is the class name.
/// The slug format is `{file_slug}::{name}` for module-level,
/// or `{file_slug}::{ClassName}.{name}` for class-level.
pub fn build_variable_node(
    file_slug: &str,
    name: &str,
    rel_path: &str,
    line_start: i32,
    value: &str,
    scope: &str,
    scope_name: &str,
    is_constant: bool,
    language: &str,
) -> GraphEntity {
    let var_slug = if scope == "class" && !scope_name.is_empty() {
        format!("{file_slug}::{scope_name}.{name}")
    } else {
        format!("{file_slug}::{name}")
    };
    // Cap value at 200 chars (count by chars, not bytes, to stay on UTF-8 boundaries)
    let capped_value = if value.chars().count() > 200 {
        let truncated: String = value.chars().take(199).collect();
        format!("{truncated}…")
    } else {
        value.to_string()
    };
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(var_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("path".to_string(), Value::String(rel_path.to_string()));
    props.insert("lineStart".to_string(), Value::Number(line_start.into()));
    props.insert("value".to_string(), Value::String(capped_value));
    props.insert("scope".to_string(), Value::String(scope.to_string()));
    props.insert(
        "scopeName".to_string(),
        Value::String(scope_name.to_string()),
    );
    props.insert("isConstant".to_string(), Value::Bool(is_constant));
    props.insert("language".to_string(), Value::String(language.to_string()));
    GraphEntity::Node {
        node_type: "Variable".to_string(),
        slug: var_slug,
        properties: props,
    }
}

/// Build a ContainsVariable edge from a File to a Variable.
pub fn build_contains_variable_edge(file_slug: &str, variable_slug: &str) -> GraphEntity {
    GraphEntity::Edge {
        edge_type: "ContainsVariable".to_string(),
        from_slug: file_slug.to_string(),
        to_slug: variable_slug.to_string(),
        properties: Map::new(),
    }
}

/// Count parameters by splitting on commas between the first `(` and `)`.
///
/// Handles both TypeScript and Rust parameter lists.
pub fn count_params(text: &str) -> i32 {
    if let Some(open) = text.find('(') {
        if let Some(close) = text.find(')') {
            let params = text[open + 1..close].trim();
            if params.is_empty() {
                return 0;
            }
            return params.matches(',').count() as i32 + 1;
        }
    }
    0
}

/// Count Rust parameters, filtering out `self`, `&self`, `&mut self`.
pub fn count_params_rust(text: &str) -> i32 {
    if let Some(open) = text.find('(') {
        if let Some(close) = text.find(')') {
            let params = text[open + 1..close].trim();
            if params.is_empty() {
                return 0;
            }
            let params_str = params
                .replace("&self", "")
                .replace("&mut self", "")
                .replace("self", "");
            let trimmed = params_str.trim().trim_matches(',').trim();
            if trimmed.is_empty() {
                return 0;
            }
            return trimmed.matches(',').count() as i32 + 1;
        }
    }
    0
}

/// Build a Dependency node with standard properties.
///
/// Slug format: `dep::<package-name>` for upsert semantics.
pub fn build_dependency_node(name: &str, version: &str, is_dev: bool, source: &str) -> GraphEntity {
    let dep_slug = format!("dep::{name}");
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(dep_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("version".to_string(), Value::String(version.to_string()));
    props.insert("isDev".to_string(), Value::Bool(is_dev));
    props.insert("source".to_string(), Value::String(source.to_string()));
    GraphEntity::Node {
        node_type: "Dependency".to_string(),
        slug: dep_slug,
        properties: props,
    }
}

/// Build a DependsOn edge from a File to a Dependency.
pub fn build_depends_on_edge(file_slug: &str, dep_name: &str) -> GraphEntity {
    let dep_slug = format!("dep::{dep_name}");
    GraphEntity::Edge {
        edge_type: "DependsOn".to_string(),
        from_slug: file_slug.to_string(),
        to_slug: dep_slug,
        properties: Map::new(),
    }
}

/// Count Python parameters, filtering out `self` and `cls`.
pub fn count_params_python(text: &str) -> i32 {
    if let Some(open) = text.find('(') {
        if let Some(close) = text.find(')') {
            let params = text[open + 1..close].trim();
            if params.is_empty() {
                return 0;
            }
            let count = params
                .split(',')
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .filter(|p| {
                    let name = p
                        .split(':')
                        .next()
                        .unwrap_or(p)
                        .split('=')
                        .next()
                        .unwrap_or(p)
                        .trim();
                    name != "self" && name != "cls"
                })
                .count();
            return count as i32;
        }
    }
    0
}

/// Count Go parameters, filtering out the receiver parameter.
///
/// Go method signatures look like: `func (r *Receiver) Name(a int, b string)`
/// The receiver `(r *Receiver)` is the first paren group, actual params are the second.
pub fn count_params_go(text: &str) -> i32 {
    // For methods: func (recv) Name(params) — use the second paren group
    // For functions: func Name(params) — use the first paren group
    let has_receiver = text.starts_with("func (") || text.starts_with("func(");

    if has_receiver {
        // Skip past the receiver paren group
        if let Some(first_close) = text.find(')') {
            let rest = &text[first_close + 1..];
            return count_params(rest);
        }
    }

    count_params(text)
}

/// Extract an identifier name after a keyword (e.g., "function " → name, "fn " → name).
///
/// Shared by both TypeScript and Rust extractors to avoid duplicated name-parsing logic.
/// Returns an empty string if the keyword is not found.
pub fn extract_name_after_keyword(text: &str, keyword: &str) -> String {
    // Strip comments before searching, so keywords in comments don't match.
    // Process line by line: remove // line comments and /* */ block comments.
    let stripped = strip_comments(text);
    if let Some(pos) = stripped.find(keyword) {
        let after = &stripped[pos + keyword.len()..];
        return after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
    }
    String::new()
}

/// Strip C-style comments from source text.
///
/// Removes `// ...` line comments and `/* ... */` block comments.
/// Also strips string literals (double-quoted) to avoid matching keywords
/// inside annotation values like `@SuppressWarnings("MemberName")`.
fn strip_comments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Line comment — skip to end of line
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Block comment — skip to closing */
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2; // skip */
            }
        } else if bytes[i] == b'"' {
            // String literal — skip contents to avoid keyword matches
            result.push(' ');
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2; // skip escape sequence
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1; // skip closing "
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Find the line number of the closing brace for a block starting at `start`.
///
/// Tracks `{` / `}` depth and returns the line index where depth returns
/// to zero. Used by C++, C#, and other line-scanning extractors.
pub fn find_closing_brace(lines: &[&str], start: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate().skip(start) {
        for c in line.chars() {
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: `build_variable_node` previously byte-sliced the value at
    /// `&value[..199]`, panicking when byte 199 landed inside a multi-byte
    /// UTF-8 character (e.g. '•', '☁'). Truncation must be char-boundary safe.
    #[test]
    fn truncates_value_on_char_boundary_with_multibyte_chars() {
        // '•' (U+2022) is 3 bytes; 201 of them = 201 chars / 603 bytes.
        // Byte index 199 falls *inside* the 67th '•', which used to panic.
        let value = "•".repeat(201);

        let entity = build_variable_node(
            "src/icons",
            "maskedKey",
            "src/icons.ts",
            1,
            &value,
            "module",
            "",
            true,
            "typescript",
        );

        let stored = match entity {
            GraphEntity::Node { properties, .. } => properties
                .get("value")
                .and_then(Value::as_str)
                .map(str::to_string)
                .expect("value property present"),
            _ => panic!("expected a Node"),
        };

        // 199 retained chars + the ellipsis marker.
        assert!(stored.ends_with('…'));
        assert_eq!(stored.chars().count(), 200);
        // Must be valid UTF-8 with no replacement/garbage from a bad slice.
        assert!(stored.chars().take(199).all(|c| c == '•'));
    }

    /// Short values (<= 200 chars) are stored verbatim, even with unicode.
    #[test]
    fn short_unicode_value_stored_verbatim() {
        let value = "key = '☁ ✏ 🖥 ⚠'";
        let entity = build_variable_node(
            "src/icons",
            "k",
            "src/icons.ts",
            1,
            value,
            "module",
            "",
            false,
            "typescript",
        );
        let stored = match entity {
            GraphEntity::Node { properties, .. } => properties
                .get("value")
                .and_then(Value::as_str)
                .unwrap()
                .to_string(),
            _ => panic!("expected a Node"),
        };
        assert_eq!(stored, value);
    }
}
