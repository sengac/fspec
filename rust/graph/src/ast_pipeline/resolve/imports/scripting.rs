//! Import and package rules of Python, TypeScript / JavaScript and Go
//! (KGRAPH-075). Every binding comes from the import as written (its syntax
//! snapshot), the files of the tree and the package clauses; a module is a
//! file only when the language's module rules name that file.
//!
//! A binding whose `local` is the empty string binds no name: it only
//! carries the Imports edge (and, when unresolved, its reason). It is used
//! for the module an import statement loads, and for imports that bind a
//! name only inside a function or class body.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{Reason, TreeIndex};
use super::{ImportBinding, ImportTarget};
use crate::ast_pipeline::facts::{FileFacts, Syntax};

pub(crate) fn bindings(
    index: &TreeIndex<'_>,
    file: usize,
    manifests: &BTreeMap<String, String>,
) -> Vec<ImportBinding> {
    match index.file(file).language {
        "python" => python::bindings(index, file),
        "typescript" | "javascript" => ecma::bindings(index, file),
        "go" => go::bindings(index, file, manifests),
        _ => Vec::new(),
    }
}

/// Go: the other files of the package directory with the same package
/// clause. Python and ES modules have no implicit package scope.
pub(crate) fn package_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    match index.file(file).language {
        "go" => go::package_peers(index, file),
        _ => Vec::new(),
    }
}

/// The name of a binding that binds no name.
const EDGE_ONLY: &str = "";

fn binding(
    local: Option<&str>,
    target: Result<ImportTarget, Reason>,
    import_path: &str,
) -> ImportBinding {
    ImportBinding {
        local: local.map(str::to_string),
        target,
        import_path: import_path.to_string(),
    }
}

/// Named children of a snapshot of one of `kinds`.
fn kids<'s>(syntax: &'s Syntax, kinds: &'s [&'s str]) -> impl Iterator<Item = &'s Syntax> + 's {
    syntax
        .children
        .iter()
        .filter(move |c| kinds.contains(&c.kind.as_str()))
}

fn kid<'s>(syntax: &'s Syntax, kinds: &'s [&'s str]) -> Option<&'s Syntax> {
    kids(syntax, kinds).next()
}

/// The directory of a relative path ("" for the root).
fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn join(dir: &str, rel: &str) -> String {
    if dir.is_empty() {
        rel.to_string()
    } else if rel.is_empty() {
        dir.to_string()
    } else {
        format!("{dir}/{rel}")
    }
}

/// `rel` (with `.` and `..` segments) from `dir`, or `None` when it leaves
/// the root.
fn normalize(dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for segment in rel.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    Some(parts.join("/"))
}

/// The files directly inside `dir`, sorted by path.
fn files_in_dir(index: &TreeIndex<'_>, dir: &str) -> Vec<usize> {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    index
        .file_by_path
        .range(prefix.as_str()..)
        .take_while(|(path, _)| path.starts_with(prefix.as_str()))
        .filter(|(path, _)| !path[prefix.len()..].contains('/'))
        .map(|(_, file)| *file)
        .collect()
}

/// Whether a file of `language` is under `dir`.
fn has_files_under(index: &TreeIndex<'_>, dir: &str, language: &str) -> bool {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    index
        .file_by_path
        .range(prefix.as_str()..)
        .take_while(|(path, _)| path.starts_with(prefix.as_str()))
        .any(|(_, file)| index.file(*file).language == language)
}

/// The file-level definitions of `facts` named `name`.
fn top_level<'f>(facts: &'f FileFacts, name: &'f str) -> impl Iterator<Item = usize> + 'f {
    facts
        .definitions
        .iter()
        .enumerate()
        .filter(move |(_, d)| d.container.is_none() && d.name == name)
        .map(|(i, _)| i)
}

/// One definition among `found`, or why not.
fn one(file: usize, found: Vec<usize>) -> Option<Result<ImportTarget, Reason>> {
    match found.as_slice() {
        [] => None,
        [def] => Some(Ok(ImportTarget::Definition((file, *def)))),
        _ => Some(Err(Reason::Ambiguous)),
    }
}

type Visited = BTreeSet<(usize, String)>;

/// Python: `import a.b`, `import a.b as c`, `from a.b import c as d`,
/// `from ..x import y`, `from x import *`. Absolute modules are relative to
/// the root; relative modules to the importing file's package (its
/// directory). A package (`a/__init__.py`) is found before a module
/// (`a.py`), as CPython's path finder does; a directory of Python files
/// without `__init__` is a namespace package.
mod python {
    use super::*;

    /// A resolved module: its file, and its directory when it is a package.
    #[derive(Clone)]
    pub(super) struct Module {
        file: Option<usize>,
        package: Option<String>,
    }

    /// What one imported name refers to.
    enum What {
        Module(Result<Module, Reason>),
        Name(Result<Module, Reason>, String),
        Wildcard(Result<Module, Reason>),
    }

    struct Item {
        /// `None` for a wildcard; `EDGE_ONLY` when no name is bound.
        local: Option<String>,
        what: What,
        import_path: String,
    }

    fn module_at(index: &TreeIndex<'_>, path: &str) -> Option<Module> {
        for init in ["__init__.py", "__init__.pyi"] {
            if let Some(file) = index.file_by_path.get(join(path, init).as_str()) {
                return Some(Module {
                    file: Some(*file),
                    package: Some(path.to_string()),
                });
            }
        }
        if !path.is_empty() {
            for ext in [".py", ".pyi"] {
                if let Some(file) = index.file_by_path.get(format!("{path}{ext}").as_str()) {
                    return Some(Module {
                        file: Some(*file),
                        package: None,
                    });
                }
            }
        }
        has_files_under(index, path, "python").then(|| Module {
            file: None,
            package: Some(path.to_string()),
        })
    }

    /// An absolute module. A missing submodule of a regular package or
    /// module of the tree is not found; any other missing module is outside
    /// the root.
    fn absolute(index: &TreeIndex<'_>, segments: &[&str]) -> Result<Module, Reason> {
        if let Some(module) = module_at(index, &segments.join("/")) {
            return Ok(module);
        }
        let local_top = segments
            .first()
            .and_then(|top| module_at(index, top))
            .is_some_and(|m| m.file.is_some());
        Err(if local_top {
            Reason::NotFound
        } else {
            Reason::External
        })
    }

    fn dotted(syntax: &Syntax) -> Vec<&str> {
        match syntax.kind.as_str() {
            "dotted_name" => kids(syntax, &["identifier"])
                .map(|i| i.text.as_str())
                .collect(),
            "identifier" => vec![syntax.text.as_str()],
            _ => Vec::new(),
        }
    }

    /// The items of one import statement of `file`.
    fn items(index: &TreeIndex<'_>, file: usize, import: &Syntax) -> Vec<Item> {
        // An import in a function or class body binds its names there only.
        let scoped = import.container.is_some();
        let local = |name: &str| {
            Some(if scoped {
                EDGE_ONLY.to_string()
            } else {
                name.to_string()
            })
        };
        let mut out = Vec::new();
        match import.kind.as_str() {
            "import_statement" => {
                for item in &import.children {
                    match item.kind.as_str() {
                        "dotted_name" => {
                            let segments = dotted(item);
                            let Some(top) = segments.first() else {
                                continue;
                            };
                            let path = segments.join(".");
                            if segments.len() > 1 {
                                // `import a.b` loads a.b and binds `a`.
                                out.push(Item {
                                    local: Some(EDGE_ONLY.to_string()),
                                    what: What::Module(absolute(index, &segments)),
                                    import_path: path,
                                });
                                if let Some(module) = module_at(index, top) {
                                    out.push(Item {
                                        local: local(top),
                                        what: What::Module(Ok(module)),
                                        import_path: (*top).to_string(),
                                    });
                                }
                            } else {
                                out.push(Item {
                                    local: local(top),
                                    what: What::Module(absolute(index, &segments)),
                                    import_path: path,
                                });
                            }
                        }
                        "aliased_import" => {
                            let (Some(name), Some(alias)) =
                                (kid(item, &["dotted_name"]), kid(item, &["identifier"]))
                            else {
                                continue;
                            };
                            let segments = dotted(name);
                            out.push(Item {
                                local: local(&alias.text),
                                what: What::Module(absolute(index, &segments)),
                                import_path: segments.join("."),
                            });
                        }
                        _ => {}
                    }
                }
            }
            "import_from_statement" => {
                let parts: Vec<&Syntax> = kids(
                    import,
                    &[
                        "dotted_name",
                        "relative_import",
                        "aliased_import",
                        "wildcard_import",
                    ],
                )
                .collect();
                let Some((first, names)) = parts.split_first() else {
                    return out;
                };
                let (module, path) = match first.kind.as_str() {
                    "relative_import" => relative(index, file, first),
                    _ => {
                        let segments = dotted(first);
                        (absolute(index, &segments), segments.join("."))
                    }
                };
                if let Ok(Module { file: Some(_), .. }) = &module {
                    out.push(Item {
                        local: Some(EDGE_ONLY.to_string()),
                        what: What::Module(module.clone()),
                        import_path: path.clone(),
                    });
                }
                for name in names {
                    let (imported, bound) = match name.kind.as_str() {
                        "wildcard_import" => {
                            out.push(Item {
                                local: if scoped {
                                    Some(EDGE_ONLY.to_string())
                                } else {
                                    None
                                },
                                what: What::Wildcard(module.clone()),
                                import_path: path.clone(),
                            });
                            continue;
                        }
                        "aliased_import" => {
                            match (kid(name, &["dotted_name"]), kid(name, &["identifier"])) {
                                (Some(n), Some(alias)) => (dotted(n).join("."), alias.text.clone()),
                                _ => continue,
                            }
                        }
                        _ => {
                            let n = dotted(name).join(".");
                            (n.clone(), n)
                        }
                    };
                    out.push(Item {
                        local: local(&bound),
                        what: What::Name(module.clone(), imported),
                        import_path: path.clone(),
                    });
                }
            }
            _ => {}
        }
        out
    }

    /// `from ..x import`: one dot is the importing file's package, each
    /// further dot its parent.
    fn relative(
        index: &TreeIndex<'_>,
        file: usize,
        syntax: &Syntax,
    ) -> (Result<Module, Reason>, String) {
        let prefix = kid(syntax, &["import_prefix"]).map_or("", |p| p.text.as_str());
        let segments = kid(syntax, &["dotted_name"])
            .map(dotted)
            .unwrap_or_default();
        let written = format!("{prefix}{}", segments.join("."));
        let dots = prefix.chars().filter(|c| *c == '.').count();
        let mut dir = Some(dir_of(&index.file(file).path).to_string());
        for _ in 1..dots {
            dir = dir.and_then(|d| normalize(&d, ".."));
        }
        let module = dir
            .and_then(|d| module_at(index, &join(&d, &segments.join("/"))))
            .ok_or(Reason::NotFound);
        (module, written)
    }

    fn resolve(
        index: &TreeIndex<'_>,
        what: &What,
        visited: &mut Visited,
    ) -> Option<Result<ImportTarget, Reason>> {
        match what {
            What::Module(Ok(module)) | What::Wildcard(Ok(module)) => {
                module.file.map(|f| Ok(ImportTarget::File(f)))
            }
            What::Module(Err(r)) | What::Wildcard(Err(r)) | What::Name(Err(r), _) => Some(Err(*r)),
            What::Name(Ok(module), name) => Some(lookup(index, module, name, visited)),
        }
    }

    /// `name` in `module`: a definition or a binding of the module's file,
    /// else its submodule.
    fn lookup(
        index: &TreeIndex<'_>,
        module: &Module,
        name: &str,
        visited: &mut Visited,
    ) -> Result<ImportTarget, Reason> {
        if let Some(found) = module
            .file
            .and_then(|f| defined_in(index, f, name, visited))
        {
            return found;
        }
        if let Some(package) = &module.package {
            if let Some(file) = module_at(index, &join(package, name)).and_then(|m| m.file) {
                return Ok(ImportTarget::File(file));
            }
        }
        Err(Reason::NotFound)
    }

    /// What `name` is at the top level of `file`: a definition, or the
    /// target of one of its module-level imports (a re-export).
    fn defined_in(
        index: &TreeIndex<'_>,
        file: usize,
        name: &str,
        visited: &mut Visited,
    ) -> Option<Result<ImportTarget, Reason>> {
        if !visited.insert((file, name.to_string())) {
            return Some(Err(Reason::NotFound));
        }
        let facts = index.file(file);
        if let Some(found) = one(file, top_level(facts, name).collect()) {
            return Some(found);
        }
        // Every import that binds the name (`try: from a import x` /
        // `except: from b import x` binds it twice).
        let mut targets = BTreeSet::new();
        let mut first_error = None;
        for import in facts.imports.iter().filter(|i| i.container.is_none()) {
            for item in items(index, file, import) {
                let found = match &item.local {
                    Some(local) if local == name => resolve(index, &item.what, visited),
                    // `import *` binds the public names of the module.
                    None if !name.starts_with('_') => match &item.what {
                        What::Wildcard(Ok(Module { file: Some(f), .. })) => {
                            defined_in(index, *f, name, visited).filter(Result::is_ok)
                        }
                        _ => None,
                    },
                    _ => None,
                };
                match found {
                    Some(Ok(target)) => {
                        targets.insert(target);
                    }
                    Some(Err(reason)) => {
                        first_error.get_or_insert(reason);
                    }
                    None => {}
                }
            }
        }
        let mut targets = targets.into_iter();
        match (targets.next(), targets.next()) {
            (Some(target), None) => return Some(Ok(target)),
            (Some(_), Some(_)) => return Some(Err(Reason::Ambiguous)),
            (None, _) => {
                if let Some(reason) = first_error {
                    return Some(Err(reason));
                }
            }
        }
        // A module-level variable is not a definition.
        facts
            .variables
            .iter()
            .any(|v| v.container.is_none() && v.name == name)
            .then_some(Err(Reason::NotFound))
    }

    /// The names `from m import *` binds: the module's public names (no
    /// leading underscore), defined or imported at its top level.
    fn public_names(
        index: &TreeIndex<'_>,
        file: usize,
        seen: &mut BTreeSet<usize>,
    ) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        if !seen.insert(file) {
            return names;
        }
        let facts = index.file(file);
        names.extend(
            facts
                .definitions
                .iter()
                .filter(|d| d.container.is_none())
                .map(|d| d.name.clone()),
        );
        for import in facts.imports.iter().filter(|i| i.container.is_none()) {
            for item in items(index, file, import) {
                match (item.local, &item.what) {
                    (Some(local), _) => {
                        names.insert(local);
                    }
                    (None, What::Wildcard(Ok(Module { file: Some(f), .. }))) => {
                        names.extend(public_names(index, *f, seen));
                    }
                    _ => {}
                }
            }
        }
        names.retain(|n| !n.is_empty() && !n.starts_with('_'));
        names
    }

    pub(super) fn bindings(index: &TreeIndex<'_>, file: usize) -> Vec<ImportBinding> {
        let mut out = Vec::new();
        for import in &index.file(file).imports {
            for item in items(index, file, import) {
                let mut visited = Visited::new();
                if let (
                    None,
                    What::Wildcard(Ok(Module {
                        file: Some(module), ..
                    })),
                ) = (&item.local, &item.what)
                {
                    // Each public name, rather than every top-level name.
                    out.push(binding(
                        Some(EDGE_ONLY),
                        Ok(ImportTarget::File(*module)),
                        &item.import_path,
                    ));
                    for name in public_names(index, *module, &mut BTreeSet::new()) {
                        if let Some(target) = defined_in(index, *module, &name, &mut Visited::new())
                        {
                            out.push(binding(Some(&name), target, &item.import_path));
                        }
                    }
                    continue;
                }
                if let Some(target) = resolve(index, &item.what, &mut visited) {
                    out.push(ImportBinding {
                        local: item.local,
                        target,
                        import_path: item.import_path,
                    });
                }
            }
        }
        out
    }
}

/// TypeScript and JavaScript ES modules: relative specifiers resolve
/// against the importing file's directory with the TypeScript / Node
/// extension candidates and `index` files; bare specifiers are packages.
/// Named, default and namespace imports bind names; exports, re-exports
/// (`export { a } from`, `export * from`, `export * as ns from`) and local
/// export lists are followed through `export_statement` snapshots.
mod ecma {
    use super::*;

    /// Candidates appended to a specifier, in resolution order.
    const EXTENSIONS: &[&str] = &[
        ".ts", ".tsx", ".d.ts", ".js", ".jsx", ".mjs", ".mts", ".cjs", ".cts",
    ];
    /// TypeScript maps a JavaScript output extension to its sources.
    const SOURCE_FOR_OUTPUT: &[(&str, &[&str])] = &[
        (".js", &[".ts", ".tsx", ".d.ts"]),
        (".jsx", &[".tsx", ".d.ts"]),
        (".mjs", &[".mts", ".d.mts"]),
        (".cjs", &[".cts", ".d.cts"]),
    ];
    const INDEX_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".d.ts", ".js", ".jsx"];

    enum What {
        /// The module itself (namespace import, side-effect import).
        File(Result<usize, Reason>),
        /// An export of the module (`default` for a default import).
        Export(Result<usize, Reason>, String),
    }

    struct Item {
        local: String,
        what: What,
        import_path: String,
    }

    /// The text of a string literal snapshot.
    fn string_text(syntax: &Syntax) -> String {
        if syntax.kind == "string" {
            kids(syntax, &["string_fragment"])
                .map(|f| f.text.as_str())
                .collect()
        } else {
            syntax.text.clone()
        }
    }

    pub(super) fn resolve_specifier(
        index: &TreeIndex<'_>,
        importer: usize,
        specifier: &str,
    ) -> Result<usize, Reason> {
        let relative = specifier == "."
            || specifier == ".."
            || specifier.starts_with("./")
            || specifier.starts_with("../");
        if !relative {
            return Err(Reason::External);
        }
        let base =
            normalize(dir_of(&index.file(importer).path), specifier).ok_or(Reason::NotFound)?;
        let mut candidates = Vec::new();
        if !base.is_empty() {
            candidates.push(base.clone());
            candidates.extend(EXTENSIONS.iter().map(|ext| format!("{base}{ext}")));
            for (output, sources) in SOURCE_FOR_OUTPUT {
                if let Some(stem) = base.strip_suffix(output) {
                    candidates.extend(sources.iter().map(|ext| format!("{stem}{ext}")));
                }
            }
        }
        candidates.extend(
            INDEX_EXTENSIONS
                .iter()
                .map(|ext| join(&base, &format!("index{ext}"))),
        );
        candidates
            .iter()
            .find_map(|c| index.file_by_path.get(c.as_str()).copied())
            .ok_or(Reason::NotFound)
    }

    /// The two names of an import or export specifier: (name in the
    /// source module, name bound or exported). The JavaScript grammar keeps
    /// `default` as an anonymous token, so the snapshot of
    /// `{ default as x }` has the one child `x`, which does not start where
    /// the specifier starts, and `export { default } from` has none.
    fn specifier_names(
        index: &TreeIndex<'_>,
        file: usize,
        specifier: &Syntax,
    ) -> Option<(String, String)> {
        let javascript = index.file(file).language == "javascript";
        let names: Vec<String> = specifier.children.iter().map(string_text).collect();
        match names.as_slice() {
            [name, alias] => Some((name.clone(), alias.clone())),
            [name] if javascript && specifier.children[0].start_byte != specifier.start_byte => {
                Some(("default".to_string(), name.clone()))
            }
            [name] => Some((name.clone(), name.clone())),
            [] if javascript => Some(("default".to_string(), "default".to_string())),
            _ => None,
        }
    }

    fn items(index: &TreeIndex<'_>, file: usize, import: &Syntax) -> Vec<Item> {
        // An import inside an ambient module binds its names there only.
        let scoped = import.container.is_some();
        let local = |name: &str| {
            if scoped {
                EDGE_ONLY.to_string()
            } else {
                name.to_string()
            }
        };
        let mut out = Vec::new();
        let (source, require) = match kid(import, &["import_require_clause"]) {
            Some(clause) => (kid(clause, &["string"]), true),
            None => (kid(import, &["string"]), false),
        };
        let Some(source) = source else {
            return out;
        };
        let specifier = string_text(source);
        let target = resolve_specifier(index, file, &specifier);
        let item = |local: String, what: What| Item {
            local,
            what,
            import_path: specifier.clone(),
        };
        // `import x = require()` binds the module's `export =` value, which
        // the snapshot does not show: only the module is loaded.
        let clauses = if require {
            Vec::new()
        } else {
            kids(import, &["import_clause"]).collect()
        };
        for clause in clauses {
            for part in &clause.children {
                match part.kind.as_str() {
                    "identifier" => out.push(item(
                        local(&part.text),
                        What::Export(target, "default".into()),
                    )),
                    "namespace_import" => {
                        if let Some(name) = kid(part, &["identifier"]) {
                            out.push(item(local(&name.text), What::File(target)));
                        }
                    }
                    "named_imports" => {
                        for specifier in kids(part, &["import_specifier"]) {
                            let Some((imported, bound)) = specifier_names(index, file, specifier)
                            else {
                                continue;
                            };
                            out.push(item(local(&bound), What::Export(target, imported)));
                        }
                    }
                    _ => {}
                }
            }
        }
        // The module the statement loads; an unresolved module is counted
        // once, on the names when it binds any.
        if target.is_ok() || out.is_empty() {
            out.insert(0, item(EDGE_ONLY.to_string(), What::File(target)));
        }
        out
    }

    fn resolve(
        index: &TreeIndex<'_>,
        what: &What,
        visited: &mut Visited,
    ) -> Result<ImportTarget, Reason> {
        match what {
            What::File(target) => target.map(ImportTarget::File),
            What::Export(Err(r), _) => Err(*r),
            What::Export(Ok(file), name) => export(index, *file, name, visited),
        }
    }

    /// The export statements of `file`'s top level, without decorators
    /// and comments.
    fn exports(facts: &FileFacts) -> impl Iterator<Item = Vec<&Syntax>> + '_ {
        facts
            .declarations
            .iter()
            .filter(|d| d.kind == "export_statement" && d.container.is_none())
            .map(|d| {
                d.children
                    .iter()
                    .filter(|c| c.kind != "decorator" && c.kind != "comment")
                    .collect()
            })
    }

    /// The export `name` of `file`.
    fn export(
        index: &TreeIndex<'_>,
        file: usize,
        name: &str,
        visited: &mut Visited,
    ) -> Result<ImportTarget, Reason> {
        if !visited.insert((file, name.to_string())) {
            return Err(Reason::NotFound);
        }
        let facts = index.file(file);
        let default = name == "default";
        if !default {
            // `export function name`, `export class name`, ...
            let exported: Vec<usize> = top_level(facts, name)
                .filter(|d| facts.definitions[*d].is_public)
                .collect();
            if let Some(found) = one(file, exported) {
                return found;
            }
        }
        let mut stars = Vec::new();
        for parts in exports(facts) {
            let source = parts
                .iter()
                .find(|p| p.kind == "string")
                .map(|s| string_text(s));
            let clause = parts.iter().find(|p| p.kind == "export_clause");
            let namespace = parts.iter().find(|p| p.kind == "namespace_export");
            match (clause, namespace, &source) {
                (Some(clause), _, _) => {
                    for specifier in kids(clause, &["export_specifier"]) {
                        let Some((local, exported)) = specifier_names(index, file, specifier)
                        else {
                            continue;
                        };
                        if exported != name {
                            continue;
                        }
                        return match &source {
                            Some(s) => resolve_specifier(index, file, s)
                                .and_then(|g| export(index, g, &local, visited)),
                            None => local_name(index, file, &local, visited),
                        };
                    }
                }
                (None, Some(ns), Some(s))
                    if ns.children.first().map(string_text).as_deref() == Some(name) =>
                {
                    return resolve_specifier(index, file, s).map(ImportTarget::File);
                }
                // `export default x;`: the only part is the expression.
                (None, None, None) if default => {
                    if let [value] = parts.as_slice() {
                        if value.kind == "identifier" {
                            return local_name(index, file, &value.text, visited);
                        }
                    }
                }
                // `export * from 's'`.
                (None, None, Some(s)) if parts.len() == 1 && !default => stars.push(s.clone()),
                _ => {}
            }
        }
        let mut found = BTreeSet::new();
        for star in stars {
            if let Ok(target) =
                resolve_specifier(index, file, &star).and_then(|g| export(index, g, name, visited))
            {
                found.insert(target);
            }
        }
        let mut found = found.into_iter();
        match (found.next(), found.next()) {
            (Some(one), None) => Ok(one),
            (Some(_), Some(_)) => Err(Reason::Ambiguous),
            (None, _) => Err(Reason::NotFound),
        }
    }

    /// A name of `file`'s module scope: a definition or an import binding.
    fn local_name(
        index: &TreeIndex<'_>,
        file: usize,
        name: &str,
        visited: &mut Visited,
    ) -> Result<ImportTarget, Reason> {
        let facts = index.file(file);
        if let Some(found) = one(file, top_level(facts, name).collect()) {
            return found;
        }
        for import in facts.imports.iter().filter(|i| i.container.is_none()) {
            for item in items(index, file, import) {
                if item.local == name {
                    return resolve(index, &item.what, visited);
                }
            }
        }
        Err(Reason::NotFound)
    }

    pub(super) fn bindings(index: &TreeIndex<'_>, file: usize) -> Vec<ImportBinding> {
        let mut out = Vec::new();
        for import in &index.file(file).imports {
            for item in items(index, file, import) {
                let mut visited = Visited::new();
                out.push(binding(
                    Some(&item.local),
                    resolve(index, &item.what, &mut visited),
                    &item.import_path,
                ));
            }
        }
        out
    }
}

/// Go: an import path names a package directory: a path relative to the
/// importing file's directory (`./x`), or a path under a module whose
/// `go.mod` is among the manifests. The package is the non-test `.go` files
/// of that directory; it binds its package clause name, or the alias. A
/// dot import binds the package's exported top-level names; `_` binds
/// nothing.
mod go {
    use super::*;

    /// (module path, module directory) of every `go.mod`, longest path
    /// first.
    fn modules(manifests: &BTreeMap<String, String>) -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = manifests
            .iter()
            .filter(|(path, _)| path.rsplit('/').next() == Some("go.mod"))
            .filter_map(|(path, text)| {
                module_directive(text).map(|module| (module, dir_of(path).to_string()))
            })
            .collect();
        found.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then_with(|| a.cmp(b)));
        found
    }

    /// The `module` directive of a go.mod file.
    fn module_directive(text: &str) -> Option<String> {
        text.lines().find_map(|line| {
            let line = line.split("//").next().unwrap_or("").trim();
            let rest = line.strip_prefix("module")?;
            if !rest.starts_with(char::is_whitespace) && !rest.starts_with('"') {
                return None;
            }
            let path = rest.trim().trim_matches('"');
            (!path.is_empty()).then(|| path.to_string())
        })
    }

    pub(super) fn package_name(facts: &FileFacts) -> Option<&str> {
        facts
            .declarations
            .iter()
            .filter(|d| d.kind == "package_clause")
            .find_map(|d| kid(d, &["package_identifier"]))
            .map(|p| p.text.as_str())
    }

    /// The non-test files of the package in `dir`, and its name.
    fn package(index: &TreeIndex<'_>, dir: &str) -> Result<(Vec<usize>, String), Reason> {
        let files: Vec<usize> = files_in_dir(index, dir)
            .into_iter()
            .filter(|f| {
                let facts = index.file(*f);
                facts.language == "go" && !facts.path.ends_with("_test.go")
            })
            .collect();
        let names: BTreeSet<&str> = files
            .iter()
            .filter_map(|f| package_name(index.file(*f)))
            .collect();
        let mut names = names.into_iter();
        match (names.next(), names.next()) {
            (Some(name), None) => Ok((files, name.to_string())),
            (Some(_), Some(_)) => Err(Reason::Ambiguous),
            (None, _) => Err(Reason::NotFound),
        }
    }

    fn package_dir(
        index: &TreeIndex<'_>,
        importer: usize,
        path: &str,
        modules: &[(String, String)],
    ) -> Result<String, Reason> {
        if path == "." || path == ".." || path.starts_with("./") || path.starts_with("../") {
            return normalize(dir_of(&index.file(importer).path), path).ok_or(Reason::NotFound);
        }
        for (module, dir) in modules {
            if path == module {
                return Ok(dir.clone());
            }
            if let Some(rest) = path
                .strip_prefix(module.as_str())
                .and_then(|r| r.strip_prefix('/'))
            {
                return Ok(join(dir, rest));
            }
        }
        Err(Reason::External)
    }

    fn string_text(syntax: &Syntax) -> String {
        kids(
            syntax,
            &[
                "interpreted_string_literal_content",
                "raw_string_literal_content",
            ],
        )
        .map(|c| c.text.as_str())
        .collect()
    }

    pub(super) fn bindings(
        index: &TreeIndex<'_>,
        file: usize,
        manifests: &BTreeMap<String, String>,
    ) -> Vec<ImportBinding> {
        let modules = modules(manifests);
        let mut out = Vec::new();
        for import in &index.file(file).imports {
            let specs = kids(import, &["import_spec"]).chain(
                kids(import, &["import_spec_list"]).flat_map(|list| kids(list, &["import_spec"])),
            );
            for spec in specs {
                let Some(path) = kid(spec, &["interpreted_string_literal", "raw_string_literal"])
                    .map(string_text)
                else {
                    continue;
                };
                let name = kid(spec, &["package_identifier", "dot", "blank_identifier"]);
                let package =
                    package_dir(index, file, &path, &modules).and_then(|dir| package(index, &dir));
                let (files, package_name) = match package {
                    Ok(found) => found,
                    Err(reason) => {
                        let local = name
                            .filter(|n| n.kind == "package_identifier")
                            .map_or(EDGE_ONLY, |n| n.text.as_str());
                        out.push(binding(Some(local), Err(reason), &path));
                        continue;
                    }
                };
                let local = match name.map(|n| n.kind.as_str()) {
                    Some("package_identifier") => name.map_or(EDGE_ONLY, |n| n.text.as_str()),
                    Some(_) => EDGE_ONLY,
                    None => package_name.as_str(),
                };
                for f in &files {
                    out.push(binding(Some(local), Ok(ImportTarget::File(*f)), &path));
                }
                if name.is_some_and(|n| n.kind == "dot") {
                    for f in &files {
                        let facts = index.file(*f);
                        for (i, def) in facts.definitions.iter().enumerate() {
                            if def.container.is_none()
                                && def.is_public
                                && def.node_kind != "method_declaration"
                            {
                                out.push(binding(
                                    Some(&def.name),
                                    Ok(ImportTarget::Definition((*f, i))),
                                    &path,
                                ));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    pub(super) fn package_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
        let facts = index.file(file);
        let Some(name) = package_name(facts) else {
            return Vec::new();
        };
        files_in_dir(index, dir_of(&facts.path))
            .into_iter()
            .filter(|f| *f != file)
            .filter(|f| {
                let other = index.file(*f);
                other.language == "go" && package_name(other) == Some(name)
            })
            .collect()
    }
}
