//! Ruby AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls) from Ruby source files using
//! ast-grep pattern matching.
//!
//! Ruby import resolution: `require_relative 'path'` resolves to `path.rb`
//! relative to the source file's directory. `require 'gem'` imports are
//! filtered out — only project-local `require_relative` produce edges.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
use super::helpers;
use crate::graph_entities::GraphEntity;

/// ast-grep patterns for Ruby method declarations.
const RUBY_METHOD_PATTERNS: &[(&str, bool)] = &[
    ("def $NAME($$$ARGS) $$$BODY end", false),
    ("def $NAME $$$BODY end", false),
    ("def self.$NAME($$$ARGS) $$$BODY end", true),
    ("def self.$NAME $$$BODY end", true),
];

/// ast-grep patterns for Ruby class/module declarations.
const RUBY_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME $$$BODY end", "class"),
    ("module $NAME $$$BODY end", "interface"),
];

/// Extract entities from Ruby source code.
///
/// Extracts File, Function, and Type nodes, plus Imports and Calls edges.
/// The `known_files` set is used for import resolution — only project-local
/// `require_relative` statements produce Imports edges.
pub fn extract_ruby(source: &str, rel_path: &str, known_files: &HashSet<String>) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Ruby;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("_spec.rb")
        || rel_path.contains("_test.rb")
        || rel_path.contains("test/")
        || rel_path.contains("spec/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "ruby", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract method declarations → collect names for call resolution
    let function_names = extract_methods(&root, &file_slug, &mut entities);

    // Extract type (class/module) declarations
    extract_types(&root, &file_slug, &mut entities);

    // Extract import statements → Imports edges + import map
    let import_map = extract_imports(source, rel_path, &file_slug, known_files, &mut entities);

    // Extract Calls edges from method bodies
    extract_calls(source, &file_slug, &function_names, &import_map, &mut entities);

    // Extract module-level variables
    variables::extract_variables(source, &file_slug, rel_path, "ruby", &mut entities);
    Ok(entities)
}

/// Extract method declarations from Ruby source.
///
/// Returns the set of method names found in this file (for call resolution).
fn extract_methods(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (pattern, is_class_method) in RUBY_METHOD_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = if *is_class_method {
                // "def self.foo" → extract after "self."
                if let Some(dot_pos) = matched_text.find("self.") {
                    let after = &matched_text[dot_pos + 5..];
                    after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '?' || *c == '!')
                        .collect::<String>()
                } else {
                    continue;
                }
            } else {
                helpers::extract_name_after_keyword(&matched_text, "def ")
            };

            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            // In Ruby, methods starting with _ are considered private
            let is_public = !name.starts_with('_');
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            let cc = complexity::calculate(&matched_text, "ruby");
            let meta = metadata::extract_function_meta(&matched_text, "ruby");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false,
                is_public,
                param_count,
                start_pos.line() as i32 + 1,
                end_pos.line() as i32 + 1,
            cc,
                &meta.parameters,
                &meta.source,
                &meta.docstring,
                &meta.decorators,
                "ruby",
                meta.truncated,
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

/// Extract class/module declarations from Ruby source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in RUBY_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "interface" => "module ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let type_slug = format!("{file_slug}::{name}");
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "ruby");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, true,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "ruby", type_meta.truncated,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}

/// Extract Ruby `require_relative` statements and produce Imports edges.
///
/// Resolves `require_relative 'path'` to `dir/path.rb` where `dir` is the
/// directory of the source file. Only produces edges when the resolved path
/// exists in `known_files`.
///
/// `require 'gem'` statements are skipped (external dependencies).
///
/// Returns a map of `local_name → (target_file_slug, is_local, original_name)`.
fn extract_imports(
    source: &str,
    rel_path: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let import_map = HashMap::new();

    // Determine the directory of the source file for relative resolution
    let source_dir = if let Some(slash_pos) = rel_path.rfind('/') {
        &rel_path[..slash_pos]
    } else {
        ""
    };

    for line in source.lines() {
        let trimmed = line.trim();

        // Match `require_relative 'path'` or `require_relative "path"`
        if !trimmed.starts_with("require_relative ") {
            continue;
        }

        let after = trimmed.strip_prefix("require_relative ").unwrap_or("").trim();

        // Extract the path from quotes
        let require_path = if (after.starts_with('\'') && after.ends_with('\''))
            || (after.starts_with('"') && after.ends_with('"'))
        {
            &after[1..after.len() - 1]
        } else {
            continue;
        };

        if require_path.is_empty() {
            continue;
        }

        // Resolve relative to source file directory
        let resolved_path = if source_dir.is_empty() {
            format!("{require_path}.rb")
        } else {
            format!("{source_dir}/{require_path}.rb")
        };

        let is_local = known_files.contains(&resolved_path);

        if is_local {
            edge_helpers::build_import_edge(
                file_slug,
                require_path,
                &resolved_path,
                false,
                entities,
            );
        }
    }
    import_map
}

/// Extract Calls edges from Ruby method bodies.
///
/// Scans each method body for bare function calls and resolves them
/// against known local methods and the import map.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Ruby;
    let root = lang.ast_grep(source);

    for (pattern, is_class_method) in RUBY_METHOD_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let fn_name = if *is_class_method {
                if let Some(dot_pos) = matched_text.find("self.") {
                    let after = &matched_text[dot_pos + 5..];
                    after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '?' || *c == '!')
                        .collect::<String>()
                } else {
                    continue;
                }
            } else {
                helpers::extract_name_after_keyword(&matched_text, "def ")
            };

            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            // Ruby method body: everything after the first line (def ...) until `end`
            // Find the body after the def line
            let body = if let Some(newline_pos) = matched_text.find('\n') {
                &matched_text[newline_pos..]
            } else {
                continue;
            };

            let mut callee_names = HashSet::new();
            edge_helpers::extract_call_names_from_body(body, &mut callee_names);

            edge_helpers::resolve_calls(
                &caller_slug,
                file_slug,
                &callee_names,
                &fn_name,
                local_functions,
                &HashSet::new(),
                import_map,
                entities,
            );
        }
    }
}
