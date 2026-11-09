//! Import and package rules of C, C++, Ruby, PHP, Swift and Dart
//! (KGRAPH-075). See `imports/mod.rs`.
//!
//! - C / C++: `#include "x.h"` names the file relative to the including
//!   file's directory, then relative to the root (the only rule the
//!   previous extractor had); `<x.h>`, and a quoted include found in
//!   neither place (the preprocessor then searches the system paths), are
//!   external. An include binds every top-level name of the included file.
//! - Ruby: `require_relative 'x'` names `x.rb` relative to the requiring
//!   file. `require 'x'` names `<lib>/x.rb` when the requiring file is
//!   inside a `lib` directory (`lib` is a gem's load path); otherwise it is
//!   external. Both bind every top-level name of the file.
//! - PHP: `use A\B\C [as D];` binds `C` (or `D`) to the definition `C`
//!   declared in namespace `A\B`; `include` / `require` of a string literal
//!   names the file relative to the including file's directory, and
//!   `__DIR__ . '/x.php'` names it exactly. Files declaring the same
//!   (non-global) namespace see each other's names.
//! - Swift: `import Module` is module-level: external. The files of one
//!   module (an SPM target directory `Sources/<T>/` or `Tests/<T>/`, else
//!   one directory) see each other's names.
//! - Dart: `import 'x.dart'` is relative to the importing file;
//!   `package:<p>/x.dart` names `lib/x.dart` of the package whose
//!   pubspec.yaml declares `name: <p>` (when that manifest is passed in);
//!   `dart:` and other packages are external. `as p` binds `p` to the file.
//!   An import with `show` / `hide`, and an `export`, gives the Imports edge
//!   but binds no name. `part 'x.dart'` binds the part's names; a part sees
//!   its library and the library's other parts.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{Reason, TreeIndex};
use super::{ImportBinding, ImportTarget};
use crate::ast_pipeline::facts::{ContainerFactKind, FileFacts, Syntax};

pub(crate) fn bindings(
    index: &TreeIndex<'_>,
    file: usize,
    manifests: &BTreeMap<String, String>,
) -> Vec<ImportBinding> {
    match index.file(file).language {
        "c" | "cpp" => includes(index, file),
        "ruby" => ruby(index, file),
        "php" => php(index, file),
        "swift" => swift(index, file),
        "dart" => dart(index, file, manifests),
        _ => Vec::new(),
    }
}

pub(crate) fn package_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    match index.file(file).language {
        "php" => php_peers(index, file),
        "swift" => swift_peers(index, file),
        "dart" => dart_peers(index, file),
        _ => Vec::new(),
    }
}

// ---------------------------------------------------------------- paths

/// The directory of a relative path ("" for the root).
fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// `rel` joined to `dir`, with `.` and `..` segments applied; `None` when
/// it leaves the root or is absolute.
fn join(dir: &str, rel: &str) -> Option<String> {
    if rel.starts_with('/') || rel.is_empty() {
        return None;
    }
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for segment in rel.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

fn file_at(index: &TreeIndex<'_>, path: Option<String>) -> Option<usize> {
    path.and_then(|p| index.file_by_path.get(p.as_str()).copied())
}

fn binding(
    local: Option<String>,
    target: Result<ImportTarget, Reason>,
    import_path: &str,
) -> ImportBinding {
    ImportBinding {
        local,
        target,
        import_path: import_path.to_string(),
    }
}

/// A binding that gives the Imports edge and no name: the local name is
/// the import as written, which no identifier can spell.
fn edge_only(target: Result<ImportTarget, Reason>, import_path: &str) -> ImportBinding {
    binding(Some(import_path.to_string()), target, import_path)
}

/// The leaf text of a string node whose only child is one `content` leaf.
fn literal<'s>(string: &'s Syntax, kinds: &[&str], content: &[&str]) -> Option<&'s str> {
    if !kinds.contains(&string.kind.as_str()) {
        return None;
    }
    match string.children.as_slice() {
        [leaf] if content.contains(&leaf.kind.as_str()) && leaf.children.is_empty() => {
            Some(&leaf.text)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------- C / C++

fn includes(index: &TreeIndex<'_>, file: usize) -> Vec<ImportBinding> {
    let facts = index.file(file);
    let dir = dir_of(&facts.path);
    let mut out = Vec::new();
    for include in facts.imports.iter().filter(|i| i.kind == "preproc_include") {
        let Some(path) = include.children.first() else {
            continue;
        };
        match path.kind.as_str() {
            "string_literal" => {
                let Some(written) = literal(path, &["string_literal"], &["string_content"]) else {
                    out.push(binding(None, Err(Reason::NotFound), &path.text));
                    continue;
                };
                let target = file_at(index, join(dir, written))
                    .or_else(|| file_at(index, join("", written)))
                    .filter(|t| *t != file)
                    .map(ImportTarget::File)
                    .ok_or(Reason::External);
                out.push(binding(None, target, written));
            }
            "system_lib_string" => out.push(binding(None, Err(Reason::External), &path.text)),
            // `#include MACRO`: the path is not written.
            _ => out.push(binding(None, Err(Reason::NotFound), &path.text)),
        }
    }
    out
}

// ---------------------------------------------------------------- Ruby

fn ruby(index: &TreeIndex<'_>, file: usize) -> Vec<ImportBinding> {
    let facts = index.file(file);
    let dir = dir_of(&facts.path);
    let mut out = Vec::new();
    for call in facts.declarations.iter().filter(|d| d.kind == "call") {
        let [method, arguments] = call.children.as_slice() else {
            continue;
        };
        if method.kind != "identifier" || arguments.kind != "argument_list" {
            continue;
        }
        let [argument] = arguments.children.as_slice() else {
            continue;
        };
        let Some(written) = literal(argument, &["string"], &["string_content"]) else {
            continue;
        };
        let with_ext = if written.ends_with(".rb") {
            written.to_string()
        } else {
            format!("{written}.rb")
        };
        let target = match method.text.as_str() {
            "require_relative" => file_at(index, join(dir, &with_ext))
                .map(ImportTarget::File)
                .ok_or(Reason::NotFound),
            "require" => {
                // The innermost `lib` directory around the requiring file.
                let parts: Vec<&str> = facts.path.split('/').collect();
                parts[..parts.len().saturating_sub(1)]
                    .iter()
                    .rposition(|p| *p == "lib")
                    .and_then(|i| file_at(index, join(&parts[..=i].join("/"), &with_ext)))
                    .map(ImportTarget::File)
                    .ok_or(Reason::External)
            }
            _ => continue,
        };
        out.push(binding(
            None,
            target.and_then(|t| not_self(t, file)),
            written,
        ));
    }
    out
}

fn not_self(target: ImportTarget, file: usize) -> Result<ImportTarget, Reason> {
    match target {
        ImportTarget::File(f) if f == file => Err(Reason::NotFound),
        other => Ok(other),
    }
}

// ---------------------------------------------------------------- PHP

/// The leaf texts of a `namespace_name` (`App\Models`).
fn namespace_text(name: &Syntax) -> Option<String> {
    if name.kind != "namespace_name" {
        return None;
    }
    let parts: Vec<&str> = name
        .children
        .iter()
        .filter(|c| c.kind == "name")
        .map(|c| c.text.as_str())
        .collect();
    (!parts.is_empty()).then(|| parts.join("\\"))
}

/// The namespace declarations of a PHP file: (start byte, name, braced
/// container if the declaration has a body).
fn php_namespaces(facts: &FileFacts) -> Vec<(usize, String, Option<usize>)> {
    facts
        .declarations
        .iter()
        .filter(|d| d.kind == "namespace_definition")
        .map(|d| {
            let name = d
                .children
                .first()
                .and_then(namespace_text)
                .unwrap_or_default();
            let braced = facts
                .containers
                .iter()
                .position(|c| c.kind == ContainerFactKind::Module && c.start_byte == d.start_byte);
            (d.start_byte, name, braced)
        })
        .collect()
}

/// The namespace-level definitions of a PHP file, with their namespace.
fn php_namespace_defs(facts: &FileFacts) -> Vec<(String, usize)> {
    let namespaces = php_namespaces(facts);
    facts
        .definitions
        .iter()
        .enumerate()
        .filter_map(|(i, d)| match d.container {
            None => {
                let ns = namespaces
                    .iter()
                    .rfind(|(start, _, braced)| braced.is_none() && *start < d.start_byte)
                    .map(|(_, name, _)| name.clone())
                    .unwrap_or_default();
                Some((ns, i))
            }
            Some(c) => namespaces
                .iter()
                .find(|(_, _, braced)| *braced == Some(c))
                .map(|(_, name, _)| (name.clone(), i)),
        })
        .collect()
}

fn php(index: &TreeIndex<'_>, file: usize) -> Vec<ImportBinding> {
    let facts = index.file(file);
    let dir = dir_of(&facts.path);
    let mut out = Vec::new();
    // `use` clauses: (local name, namespace, name, as written).
    let mut uses: Vec<(String, String, String, String)> = Vec::new();
    for import in &facts.imports {
        match import.kind.as_str() {
            "namespace_use_declaration" => {
                let clauses: Vec<&Syntax> = import
                    .children
                    .iter()
                    .filter(|c| c.kind == "namespace_use_clause")
                    .collect();
                if clauses.is_empty() {
                    // `use A\{B, C};`: the group is the declaration's body,
                    // which the snapshot does not hold.
                    let written = import
                        .children
                        .first()
                        .and_then(namespace_text)
                        .unwrap_or_default();
                    out.push(binding(None, Err(Reason::NotFound), &written));
                }
                for clause in clauses {
                    let (namespace, name) = match clause.children.first() {
                        Some(q) if q.kind == "qualified_name" => {
                            let namespace = q
                                .children
                                .iter()
                                .find_map(namespace_text)
                                .unwrap_or_default();
                            match q.children.iter().rfind(|c| c.kind == "name") {
                                Some(n) => (namespace, n.text.clone()),
                                None => continue,
                            }
                        }
                        Some(n) if n.kind == "name" => (String::new(), n.text.clone()),
                        _ => continue,
                    };
                    let alias = clause
                        .children
                        .get(1)
                        .filter(|c| c.kind == "name")
                        .map(|c| c.text.clone());
                    let written = if namespace.is_empty() {
                        name.clone()
                    } else {
                        format!("{namespace}\\{name}")
                    };
                    uses.push((
                        alias.unwrap_or_else(|| name.clone()),
                        namespace,
                        name,
                        written,
                    ));
                }
            }
            "include_expression"
            | "include_once_expression"
            | "require_expression"
            | "require_once_expression" => {
                let Some((path, written)) = php_include_path(dir, import) else {
                    out.push(binding(None, Err(Reason::NotFound), &import.kind));
                    continue;
                };
                let target = file_at(index, path)
                    .map(ImportTarget::File)
                    .ok_or(Reason::NotFound)
                    .and_then(|t| not_self(t, file));
                out.push(binding(None, target, &written));
            }
            _ => {}
        }
    }
    if uses.is_empty() {
        return out;
    }
    let wanted: BTreeSet<String> = uses.iter().map(|(_, ns, _, _)| ns.clone()).collect();
    // namespace -> name -> definitions, over every PHP file.
    let mut declared: BTreeMap<String, BTreeMap<String, BTreeSet<(usize, usize)>>> =
        BTreeMap::new();
    let mut namespaces: BTreeSet<String> = BTreeSet::new();
    for (f, other) in index.tree.files.iter().enumerate() {
        if other.language != "php" {
            continue;
        }
        for (_, ns, _) in php_namespaces(other) {
            namespaces.insert(ns);
        }
        for (ns, d) in php_namespace_defs(other) {
            if wanted.contains(&ns) {
                declared
                    .entry(ns)
                    .or_default()
                    .entry(other.definitions[d].name.clone())
                    .or_default()
                    .insert((f, d));
            }
        }
    }
    for (local, namespace, name, written) in uses {
        let found = declared
            .get(namespace.as_str())
            .and_then(|names| names.get(&name))
            .cloned()
            .unwrap_or_default();
        let target = match super::super::single(found) {
            Ok(def) => Ok(ImportTarget::Definition(def)),
            Err(Reason::NotFound) => {
                // A namespace of the tree (or the namespace itself, `use
                // App\Models;`): a name the tree does not define.
                let local_namespace = namespaces.iter().any(|ns| {
                    *ns == namespace || *ns == written || ns.starts_with(&format!("{written}\\"))
                });
                Err(if local_namespace {
                    Reason::NotFound
                } else {
                    Reason::External
                })
            }
            Err(reason) => Err(reason),
        };
        out.push(binding(Some(local), target, &written));
    }
    out
}

/// The path an include names: a string literal relative to the including
/// file's directory, or `__DIR__ . '/x'`.
fn php_include_path(dir: &str, include: &Syntax) -> Option<(Option<String>, String)> {
    const STRINGS: &[&str] = &["string", "encapsed_string"];
    match include.children.as_slice() {
        [string] if string.kind != "binary_expression" => {
            let written = literal(string, STRINGS, &["string_content"])?;
            Some((join(dir, written), written.to_string()))
        }
        [concat] => match concat.children.as_slice() {
            [magic, string] if magic.kind == "name" && magic.text == "__DIR__" => {
                let written = literal(string, STRINGS, &["string_content"])?;
                let rest = written.strip_prefix('/')?;
                Some((join(dir, rest), format!("__DIR__ . '{written}'")))
            }
            _ => None,
        },
        _ => None,
    }
}

/// The one unbraced namespace a PHP file declares, if it declares exactly
/// one and it is not the global namespace.
fn php_file_namespace(facts: &FileFacts) -> Option<String> {
    match php_namespaces(facts).as_slice() {
        [(_, name, None)] if !name.is_empty() => Some(name.clone()),
        _ => None,
    }
}

fn php_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    let Some(namespace) = php_file_namespace(index.file(file)) else {
        return Vec::new();
    };
    index
        .tree
        .files
        .iter()
        .enumerate()
        .filter(|(f, other)| {
            *f != file
                && other.language == "php"
                && php_file_namespace(other).as_deref() == Some(namespace.as_str())
        })
        .map(|(f, _)| f)
        .collect()
}

// ---------------------------------------------------------------- Swift

fn swift(index: &TreeIndex<'_>, file: usize) -> Vec<ImportBinding> {
    index
        .file(file)
        .imports
        .iter()
        .filter(|i| i.kind == "import_declaration")
        .map(|import| {
            let module = import
                .children
                .iter()
                .find(|c| c.kind == "identifier")
                .map(|id| {
                    id.children
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                })
                .unwrap_or_default();
            binding(None, Err(Reason::External), &module)
        })
        .collect()
}

/// The directory whose files form the module of a Swift file.
fn swift_module_dir(path: &str) -> (String, bool) {
    let parts: Vec<&str> = path.split('/').collect();
    // `Sources/<Target>/...`: the target is every file below it.
    if let Some(i) = parts[..parts.len().saturating_sub(2)]
        .iter()
        .position(|p| *p == "Sources" || *p == "Tests")
    {
        return (parts[..i + 2].join("/"), true);
    }
    (dir_of(path).to_string(), false)
}

fn swift_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    let path = &index.file(file).path;
    let (dir, recursive) = swift_module_dir(path);
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    index
        .file_by_path
        .range::<str, _>((
            std::ops::Bound::Included(prefix.as_str()),
            std::ops::Bound::Unbounded,
        ))
        .take_while(|(p, _)| p.starts_with(&prefix))
        .filter(|(p, f)| {
            **f != file
                && index.file(**f).language == "swift"
                && (recursive || !p[prefix.len()..].contains('/'))
        })
        .map(|(_, f)| *f)
        .collect()
}

// ---------------------------------------------------------------- Dart

/// The URI of a Dart directive: the `uri` child's single-part string.
fn dart_uri(uri: &Syntax) -> Option<&str> {
    if uri.kind != "uri" {
        return None;
    }
    let [string] = uri.children.as_slice() else {
        return None;
    };
    if string.kind != "string_literal" {
        return None;
    }
    let [quoted] = string.children.as_slice() else {
        return None;
    };
    literal(
        quoted,
        &[
            "string_literal_single_quotes",
            "string_literal_double_quotes",
        ],
        &[
            "template_chars_single_single",
            "template_chars_double_single",
        ],
    )
}

/// The `uri` of a `configurable_uri` (its default, not a conditional one).
fn configurable(uri: &Syntax) -> Option<&str> {
    match uri.kind.as_str() {
        "configurable_uri" => uri
            .children
            .iter()
            .find(|c| c.kind == "uri")
            .and_then(dart_uri),
        "uri" => dart_uri(uri),
        _ => None,
    }
}

/// Package name -> directory of its pubspec.yaml, for names declared once.
fn dart_packages(manifests: &BTreeMap<String, String>) -> BTreeMap<String, Option<String>> {
    let mut packages: BTreeMap<String, Option<String>> = BTreeMap::new();
    for (path, text) in manifests {
        let file = path.rsplit('/').next().unwrap_or(path);
        if file != "pubspec.yaml" {
            continue;
        }
        // The top-level `name:` key.
        let name = text.lines().find_map(|line| {
            let value = line.strip_prefix("name:")?;
            let value = value.split(" #").next().unwrap_or(value).trim();
            let value = value.trim_matches(|c| c == '"' || c == '\'');
            (!value.is_empty()).then(|| value.to_string())
        });
        if let Some(name) = name {
            let dir = dir_of(path).to_string();
            packages
                .entry(name)
                .and_modify(|d| *d = None)
                .or_insert(Some(dir));
        }
    }
    packages
}

fn dart_target(
    index: &TreeIndex<'_>,
    file: usize,
    uri: &str,
    packages: &BTreeMap<String, Option<String>>,
) -> Result<ImportTarget, Reason> {
    let facts = index.file(file);
    let path = if let Some(rest) = uri.strip_prefix("package:") {
        let (package, path) = rest.split_once('/').ok_or(Reason::External)?;
        match packages.get(package) {
            Some(Some(dir)) => join(&join(dir, "lib").ok_or(Reason::NotFound)?, path),
            Some(None) => return Err(Reason::Ambiguous),
            None => return Err(Reason::External),
        }
    } else if uri.starts_with("dart:") || uri.contains(':') {
        return Err(Reason::External);
    } else {
        join(dir_of(&facts.path), uri)
    };
    file_at(index, path)
        .map(ImportTarget::File)
        .ok_or(Reason::NotFound)
        .and_then(|t| not_self(t, file))
}

fn dart(
    index: &TreeIndex<'_>,
    file: usize,
    manifests: &BTreeMap<String, String>,
) -> Vec<ImportBinding> {
    let facts = index.file(file);
    let packages = dart_packages(manifests);
    let mut out = Vec::new();
    for directive in &facts.imports {
        match directive.kind.as_str() {
            "import_or_export" => {
                for library in &directive.children {
                    match library.kind.as_str() {
                        "library_import" => {
                            let Some(spec) = library
                                .children
                                .iter()
                                .find(|c| c.kind == "import_specification")
                            else {
                                continue;
                            };
                            let Some(uri) = spec.children.first().and_then(configurable) else {
                                continue;
                            };
                            let target = dart_target(index, file, uri, &packages);
                            let prefix = spec.children.iter().find(|c| c.kind == "identifier");
                            let combinators = spec.children.iter().any(|c| c.kind == "combinator");
                            out.push(match (prefix, combinators) {
                                (Some(prefix), _) => {
                                    binding(Some(prefix.text.clone()), target, uri)
                                }
                                // `show` and `hide` are anonymous tokens the
                                // snapshot does not hold: which names the
                                // combinator lists is unknown.
                                (None, true) => edge_only(target, uri),
                                (None, false) => binding(None, target, uri),
                            });
                        }
                        // An export makes names visible to the importers of
                        // this file, not in it.
                        "library_export" => {
                            let Some(uri) = library.children.iter().find_map(configurable) else {
                                continue;
                            };
                            out.push(edge_only(dart_target(index, file, uri, &packages), uri));
                        }
                        _ => {}
                    }
                }
            }
            "part_directive" => {
                let Some(uri) = directive.children.iter().find_map(dart_uri) else {
                    continue;
                };
                out.push(binding(None, dart_target(index, file, uri, &packages), uri));
            }
            _ => {}
        }
    }
    out
}

/// The parts a Dart library file declares.
fn dart_parts(index: &TreeIndex<'_>, library: usize) -> Vec<usize> {
    let facts = index.file(library);
    facts
        .imports
        .iter()
        .filter(|d| d.kind == "part_directive")
        .filter_map(|d| d.children.iter().find_map(dart_uri))
        .filter_map(|uri| file_at(index, join(dir_of(&facts.path), uri)))
        .filter(|p| *p != library)
        .collect()
}

fn dart_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    let mut peers = BTreeSet::new();
    for (library, facts) in index.tree.files.iter().enumerate() {
        if facts.language != "dart" || library == file {
            continue;
        }
        let parts = dart_parts(index, library);
        if parts.contains(&file) {
            peers.insert(library);
            peers.extend(parts.into_iter().filter(|p| *p != file));
        }
    }
    peers.into_iter().collect()
}
