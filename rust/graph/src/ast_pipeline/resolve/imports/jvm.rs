//! Package and namespace rules of Java, Kotlin, Scala and C# (KGRAPH-075).
//!
//! A file's package (Java, Kotlin, Scala) or namespaces (C#) are its
//! package and namespace declaration facts; a file without one is in the
//! root (default, global) package. An import path names a package of the
//! tree by its longest declared prefix; the rest of the path names a
//! top-level definition of a file of that package, then members of it. A
//! path no prefix of which is a package declared under the root is
//! external. Java, Kotlin and Scala share the JVM's packages: a file sees
//! the types (only) of the other two languages' files.

use std::collections::BTreeMap;
use std::ops::Bound;

use super::super::{DefRef, Reason, TreeIndex};
use super::{ImportBinding, ImportTarget};
use crate::ast_pipeline::facts::{Entity, FileFacts, Syntax};

/// The languages that share the JVM's package namespace.
const JVM: &[&str] = &["java", "kotlin", "scala"];

fn family(language: &str) -> &'static [&'static str] {
    if language == "csharp" {
        &["csharp"]
    } else {
        JVM
    }
}

/// The names of a dotted path snapshot, in source order.
fn names(syntax: &Syntax, out: &mut Vec<String>) {
    match syntax.kind.as_str() {
        "identifier" | "simple_identifier" | "type_identifier" if syntax.children.is_empty() => {
            out.push(syntax.text.clone())
        }
        // C#: `global::` names the global namespace.
        "alias_qualified_name" if syntax.children.first().is_some_and(|c| c.text == "global") => {
            for child in &syntax.children[1..] {
                names(child, out);
            }
        }
        // C#: a generic name names its generic type.
        "generic_name" => {
            if let Some(name) = syntax.children.first() {
                names(name, out);
            }
        }
        _ => {
            for child in &syntax.children {
                names(child, out);
            }
        }
    }
}

fn path_of(syntax: &Syntax) -> Vec<String> {
    let mut out = Vec::new();
    names(syntax, &mut out);
    out
}

/// The name a package or namespace declaration declares.
fn declared_name(declaration: &Syntax) -> Vec<String> {
    let mut out = Vec::new();
    for child in declaration.children.iter().filter(|c| {
        matches!(
            c.kind.as_str(),
            "identifier"
                | "scoped_identifier"
                | "package_identifier"
                | "qualified_name"
                | "alias_qualified_name"
        )
    }) {
        names(child, &mut out);
    }
    out
}

/// The packages a file's top-level definitions are in. Scala chains
/// package clauses (`package a; package b` is `a.b`); a C# file may
/// declare several namespaces, and its top-level types are taken to be in
/// each of them.
fn file_packages(file: &FileFacts) -> Vec<Vec<String>> {
    let declared: Vec<Vec<String>> = file.declarations.iter().map(declared_name).collect();
    if declared.is_empty() {
        return vec![Vec::new()];
    }
    match file.language {
        "scala" => vec![declared.concat()],
        "csharp" => {
            let mut all = declared;
            all.sort();
            all.dedup();
            all
        }
        _ => declared.into_iter().take(1).collect(),
    }
}

/// Scala: the packages whose members are visible in the file, innermost
/// first: one per chained package clause (`package a.b; package c` sees
/// `a.b.c` and `a.b`).
fn scala_scopes(file: &FileFacts) -> Vec<Vec<String>> {
    let mut current = Vec::new();
    let mut scopes = Vec::new();
    for declaration in &file.declarations {
        current.extend(declared_name(declaration));
        scopes.push(current.clone());
    }
    scopes.reverse();
    scopes
}

/// The files of each package of a language family, in file order.
struct Packages {
    files: BTreeMap<Vec<String>, Vec<usize>>,
}

impl Packages {
    fn new(index: &TreeIndex<'_>, language: &str) -> Self {
        let family = family(language);
        let mut files: BTreeMap<Vec<String>, Vec<usize>> = BTreeMap::new();
        for (i, file) in index.tree.files.iter().enumerate() {
            if family.contains(&file.language) {
                for package in file_packages(file) {
                    files.entry(package).or_default().push(i);
                }
            }
        }
        Packages { files }
    }

    fn get(&self, package: &[String]) -> Option<&[usize]> {
        self.files.get(package).map(Vec::as_slice)
    }

    /// Some file declares this package or a package inside it.
    fn exists_under(&self, package: &[String]) -> bool {
        self.files
            .range::<[String], _>((Bound::Included(package), Bound::Unbounded))
            .next()
            .is_some_and(|(declared, _)| declared.starts_with(package))
    }
}

struct Imports<'i, 't> {
    index: &'i TreeIndex<'t>,
    file: usize,
    language: &'static str,
    packages: Packages,
    out: Vec<ImportBinding>,
}

impl Imports<'_, '_> {
    /// A definition of another JVM language is seen only when it is a type.
    fn visible(&self, def: DefRef) -> bool {
        self.index.file(def.0).language == self.language
            || matches!(self.index.def(def).entity, Entity::Type(_))
    }

    fn top_level(&self, file: usize, name: Option<&str>) -> Vec<DefRef> {
        self.index
            .file(file)
            .definitions
            .iter()
            .enumerate()
            .filter(|(_, d)| d.container.is_none() && name.is_none_or(|n| d.name == n))
            .map(|(i, _)| (file, i))
            .filter(|def| self.visible(*def))
            .collect()
    }

    /// The nested types (and, when `functions`, the methods) of a type.
    fn members(&self, owner: DefRef, name: Option<&str>, functions: bool) -> Vec<DefRef> {
        if !matches!(self.index.def(owner).entity, Entity::Type(_)) {
            return Vec::new();
        }
        let facts = self.index.file(owner.0);
        let Some(container) = facts
            .containers
            .iter()
            .position(|c| c.definition == Some(owner.1))
        else {
            return Vec::new();
        };
        facts
            .definitions
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                d.container == Some(container)
                    && name.is_none_or(|n| d.name == n)
                    && (matches!(d.entity, Entity::Type(_))
                        || (functions && d.entity == Entity::Function))
            })
            .map(|(i, _)| (owner.0, i))
            .filter(|def| self.visible(*def))
            .collect()
    }

    /// `rest` inside the files of a package: a top-level name, then
    /// members; the last segment may name a method when `functions`.
    fn in_package(&self, files: &[usize], rest: &[String], functions: bool) -> Vec<DefRef> {
        let Some((head, tail)) = rest.split_first() else {
            return Vec::new();
        };
        let mut current: Vec<DefRef> = files
            .iter()
            .flat_map(|f| self.top_level(*f, Some(head)))
            .collect();
        for (i, segment) in tail.iter().enumerate() {
            let last = i + 1 == tail.len();
            current = current
                .iter()
                .flat_map(|owner| self.members(*owner, Some(segment), last && functions))
                .collect();
        }
        current
    }

    /// The definitions a full path names, by its longest package prefix.
    /// Java cannot import from the default package.
    fn resolve(&self, path: &[String], functions: bool) -> Result<Vec<DefRef>, Reason> {
        let shortest = if self.language == "java" { 1 } else { 0 };
        for split in (shortest..path.len()).rev() {
            if let Some(files) = self.packages.get(&path[..split]) {
                let found = self.in_package(files, &path[split..], functions);
                if !found.is_empty() {
                    return Ok(found);
                }
                if split > 0 {
                    return Err(Reason::NotFound);
                }
            }
        }
        Err(Reason::External)
    }

    fn bind(
        &mut self,
        local: Option<String>,
        target: Result<ImportTarget, Reason>,
        import_path: &str,
    ) {
        self.out.push(ImportBinding {
            local,
            target,
            import_path: import_path.to_string(),
        });
    }

    /// Definitions, each under `local` or its own name.
    fn bind_definitions(
        &mut self,
        local: Option<&str>,
        definitions: Vec<DefRef>,
        import_path: &str,
    ) {
        for def in definitions {
            let name = local.map_or_else(|| self.index.def(def).name.clone(), str::to_string);
            self.bind(Some(name), Ok(ImportTarget::Definition(def)), import_path);
        }
    }

    /// The files of a package: every top-level name (`local` is `None`),
    /// or the package under the name `local`. A file of another JVM
    /// language contributes its types.
    fn bind_package(&mut self, files: &[usize], local: Option<&str>, import_path: &str) {
        for &file in files {
            if file == self.file {
                continue;
            }
            if self.index.file(file).language == self.language {
                self.bind(
                    local.map(str::to_string),
                    Ok(ImportTarget::File(file)),
                    import_path,
                );
            } else if local.is_none() {
                let types = self.top_level(file, None);
                self.bind_definitions(None, types, import_path);
            }
        }
    }

    /// `import p.*`: a package's top-level names, or the members of a type
    /// (nested types, and methods when `functions`).
    fn wildcard(&mut self, path: &[String], functions: bool, import_path: &str) {
        if path.is_empty() {
            return;
        }
        if let Some(files) = self.packages.get(path) {
            let files = files.to_vec();
            self.bind_package(&files, None, import_path);
            return;
        }
        if self.packages.exists_under(path) {
            // A package of the tree that only holds packages.
            return;
        }
        match self.resolve(path, false) {
            Ok(owners) => {
                let members: Vec<DefRef> = owners
                    .iter()
                    .flat_map(|owner| self.members(*owner, None, functions))
                    .collect();
                self.bind_definitions(None, members, import_path);
            }
            Err(reason) => self.bind(None, Err(reason), import_path),
        }
    }

    /// `import p.Name [as Alias]`: a package (Scala, C# aliases) or the
    /// definitions the path names.
    fn explicit(
        &mut self,
        path: &[String],
        alias: Option<&str>,
        functions: bool,
        import_path: &str,
    ) {
        let Some(last) = path.last() else {
            return;
        };
        let local = alias.unwrap_or(last).to_string();
        if let Some(files) = self.packages.get(path) {
            let files = files.to_vec();
            self.bind_package(&files, Some(&local), import_path);
            return;
        }
        match self.resolve(path, functions) {
            Ok(definitions) => self.bind_definitions(Some(&local), definitions, import_path),
            Err(reason) => self.bind(Some(local), Err(reason), import_path),
        }
    }

    fn java(&mut self, import: &Syntax) {
        let Some(target) = import
            .children
            .iter()
            .find(|c| matches!(c.kind.as_str(), "scoped_identifier" | "identifier"))
        else {
            return;
        };
        let path = path_of(target);
        let written = path.join(".");
        if import.children.iter().any(|c| c.kind == "asterisk") {
            // `import p.*` or `import static p.T.*`: the snapshot does not
            // say which, so the members of a type are its nested types.
            self.wildcard(&path, false, &format!("{written}.*"));
        } else {
            // `import p.T`, `import p.T.Nested`, `import static p.T.member`.
            self.explicit(&path, None, true, &written);
        }
    }

    fn kotlin(&mut self, import: &Syntax) {
        let Some(target) = import.children.iter().find(|c| c.kind == "identifier") else {
            return;
        };
        let path = path_of(target);
        let written = path.join(".");
        let alias = import
            .children
            .iter()
            .find(|c| c.kind == "import_alias")
            .and_then(|a| path_of(a).into_iter().next());
        if import.children.iter().any(|c| c.kind == "wildcard_import") {
            self.wildcard(&path, false, &format!("{written}.*"));
        } else {
            self.explicit(&path, alias.as_deref(), true, &written);
        }
    }

    /// Scala: the path an import's first segment starts, relative to the
    /// enclosing packages (innermost first), then to the root.
    fn scala_absolute(&self, path: Vec<String>) -> Vec<String> {
        if path.first().is_some_and(|p| p == "_root_") {
            return path[1..].to_vec();
        }
        let Some(head) = path.first() else {
            return path;
        };
        for scope in scala_scopes(self.index.file(self.file)) {
            let mut candidate = scope.clone();
            candidate.push(head.clone());
            let names_member = self.packages.get(&scope).is_some_and(|files| {
                files
                    .iter()
                    .any(|f| !self.top_level(*f, Some(head)).is_empty())
            });
            if self.packages.exists_under(&candidate) || names_member {
                let mut absolute = scope;
                absolute.extend(path);
                return absolute;
            }
        }
        path
    }

    fn scala(&mut self, import: &Syntax) {
        let prefix: Vec<String> = import
            .children
            .iter()
            .filter(|c| c.kind == "identifier")
            .map(|c| c.text.clone())
            .collect();
        let prefix = self.scala_absolute(prefix);
        let written = prefix.join(".");
        let mut selected = false;
        for child in import.children.iter().filter(|c| c.kind != "identifier") {
            match child.kind.as_str() {
                "namespace_wildcard" => {
                    selected = true;
                    self.wildcard(&prefix, true, &format!("{written}.{}", child.text));
                }
                "namespace_selectors" => {
                    selected = true;
                    for selector in &child.children {
                        self.scala_selector(&prefix, selector, &written);
                    }
                }
                "as_renamed_identifier" | "arrow_renamed_identifier" => {
                    selected = true;
                    self.scala_selector(&prefix, child, &written);
                }
                _ => {}
            }
        }
        if !selected {
            self.explicit(&prefix, None, true, &written);
        }
    }

    fn scala_selector(&mut self, prefix: &[String], selector: &Syntax, written: &str) {
        match selector.kind.as_str() {
            "identifier" => {
                let mut path = prefix.to_vec();
                path.push(selector.text.clone());
                self.explicit(&path, None, true, &format!("{written}.{}", selector.text));
            }
            "as_renamed_identifier" | "arrow_renamed_identifier" => {
                let (Some(name), Some(alias)) =
                    (selector.children.first(), selector.children.get(1))
                else {
                    return;
                };
                // `Name => _` hides the name.
                if alias.kind == "wildcard" {
                    return;
                }
                let mut path = prefix.to_vec();
                path.push(name.text.clone());
                self.explicit(
                    &path,
                    Some(&alias.text),
                    true,
                    &format!("{written}.{}", name.text),
                );
            }
            "namespace_wildcard" | "wildcard" => {
                self.wildcard(prefix, true, &format!("{written}.{}", selector.text));
            }
            _ => {}
        }
    }

    /// `using N;` (a namespace's types, or with `static` a type's members:
    /// the path says which), `using A = N.T;` (an alias).
    fn csharp(&mut self, using: &Syntax) {
        let (alias, target) = match using.children.as_slice() {
            [alias, target, ..] if alias.kind == "identifier" => {
                (Some(alias.text.as_str()), target)
            }
            [target, ..] => (None, target),
            [] => return,
        };
        let path = path_of(target);
        let written = path.join(".");
        match alias {
            None => self.wildcard(&path, true, &written),
            Some(alias) => {
                if self.packages.get(&path).is_none() && self.packages.exists_under(&path) {
                    return;
                }
                self.explicit(&path, Some(alias), false, &written);
            }
        }
    }
}

pub(crate) fn bindings(
    index: &TreeIndex<'_>,
    file: usize,
    _manifests: &BTreeMap<String, String>,
) -> Vec<ImportBinding> {
    let facts = index.file(file);
    if facts.imports.is_empty() {
        return Vec::new();
    }
    let mut imports = Imports {
        index,
        file,
        language: facts.language,
        packages: Packages::new(index, facts.language),
        out: Vec::new(),
    };
    for import in &facts.imports {
        match facts.language {
            "java" => imports.java(import),
            "kotlin" => imports.kotlin(import),
            "scala" => imports.scala(import),
            "csharp" => imports.csharp(import),
            _ => {}
        }
    }
    imports.out
}

/// The other files of the same language whose top-level definitions are
/// in scope without an import: the files of the same package (Scala: of
/// each chained package clause; C#: of the namespace and of every
/// enclosing namespace, the global one included).
pub(crate) fn package_peers(index: &TreeIndex<'_>, file: usize) -> Vec<usize> {
    let facts = index.file(file);
    let mut visible: Vec<Vec<String>> = match facts.language {
        "scala" => {
            let scopes = scala_scopes(facts);
            if scopes.is_empty() {
                vec![Vec::new()]
            } else {
                scopes
            }
        }
        "csharp" => file_packages(facts)
            .into_iter()
            .flat_map(|namespace| (0..=namespace.len()).map(move |n| namespace[..n].to_vec()))
            .collect(),
        _ => file_packages(facts),
    };
    visible.sort();
    visible.dedup();
    index
        .tree
        .files
        .iter()
        .enumerate()
        .filter(|(i, other)| {
            *i != file
                && other.language == facts.language
                && file_packages(other)
                    .iter()
                    .any(|p| visible.binary_search(p).is_ok())
        })
        .map(|(i, _)| i)
        .collect()
}
