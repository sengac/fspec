//! Exact Rust name resolution (KGRAPH-075).
//!
//! Crates come from the Cargo.toml manifests under the root, the module
//! tree from `mod` declarations, and names from module items and `use`
//! declarations. A path resolves the way the compiler resolves names
//! declared in source; a path that leaves the root is external.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use super::{
    enclosing, non_path_call, single, Answer, DefRef, ImportEdge, LanguageResolver, Reason,
    TreeIndex,
};
use crate::ast_pipeline::facts::{
    BaseOwner, BaseRelation, CallSite, Callee, ContainerFactKind, Entity, FileFacts, Receiver,
    Syntax, TypeKind,
};

/// Names the standard prelude and the extern prelude put in scope without
/// a declaration (Rust 2021). They are outside every root.
const PRELUDE: &[&str] = &[
    "std",
    "core",
    "alloc",
    "proc_macro",
    "test",
    "Option",
    "Some",
    "None",
    "Result",
    "Ok",
    "Err",
    "Vec",
    "String",
    "Box",
    "ToString",
    "ToOwned",
    "Clone",
    "Copy",
    "Send",
    "Sync",
    "Sized",
    "Unpin",
    "Drop",
    "Fn",
    "FnMut",
    "FnOnce",
    "drop",
    "Default",
    "Eq",
    "PartialEq",
    "Ord",
    "PartialOrd",
    "AsRef",
    "AsMut",
    "Into",
    "From",
    "Iterator",
    "IntoIterator",
    "Extend",
    "DoubleEndedIterator",
    "ExactSizeIterator",
    "TryFrom",
    "TryInto",
    "FromIterator",
    "bool",
    "char",
    "str",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "f32",
    "f64",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Ns {
    /// Modules, crates and types: every path segment but the last.
    Type,
    /// Functions and struct constructors: the last segment of a call.
    Value,
    /// Both: what a `use` declaration binds.
    Any,
}

/// (module, name, namespace): a memoised name lookup.
type NameKey = (usize, String, Ns);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Res {
    Module(usize),
    Def(DefRef),
    Unit(usize),
    External,
}

#[derive(Clone, Debug)]
enum Dep {
    Package(String),
    External,
}

/// A crate: a library, binary, test, example or bench target.
struct Unit {
    root_file: String,
    package_dir: String,
    /// Dependency name as written in code -> package.
    deps: BTreeMap<String, Dep>,
    /// The package's library crate name, visible to its other targets.
    own_lib: Option<String>,
    is_lib: bool,
    root_module: usize,
}

struct UseBinding {
    /// The bound name; `None` for a glob.
    name: Option<String>,
    path: Vec<String>,
}

struct Module {
    unit: usize,
    file: usize,
    /// The inline `mod` container, `None` for a file's top level.
    container: Option<usize>,
    parent: Option<usize>,
    /// Declared items: name -> definitions and child modules.
    items: BTreeMap<String, Vec<Res>>,
    uses: Vec<UseBinding>,
}

struct ImplInfo {
    file: usize,
    container: usize,
}

pub(crate) struct RustResolver<'i, 't> {
    index: &'i TreeIndex<'t>,
    units: Vec<Unit>,
    modules: Vec<Module>,
    /// (file, inline module container) -> module ids (one per crate that
    /// reaches the file).
    scopes: BTreeMap<(usize, Option<usize>), Vec<usize>>,
    /// Library crate name of each package directory.
    package_lib: BTreeMap<String, usize>,
    /// Type definition -> the impl blocks whose self type it is.
    impls: BTreeMap<DefRef, Vec<ImplInfo>>,
    /// (file, impl container) -> its self type.
    impl_self: BTreeMap<(usize, usize), Answer>,
    /// (file, impl container) -> its trait.
    impl_traits: RefCell<BTreeMap<(usize, usize), Answer>>,
    in_progress: RefCell<BTreeSet<(usize, usize)>>,
    memo: RefCell<BTreeMap<NameKey, Result<BTreeSet<Res>, Reason>>>,
    glob_memo: RefCell<BTreeMap<NameKey, BTreeSet<Res>>>,
}

impl<'i, 't> RustResolver<'i, 't> {
    pub fn new(index: &'i TreeIndex<'t>, manifests: &BTreeMap<String, String>) -> Self {
        let mut resolver = RustResolver {
            index,
            units: Vec::new(),
            modules: Vec::new(),
            scopes: BTreeMap::new(),
            package_lib: BTreeMap::new(),
            impls: BTreeMap::new(),
            impl_self: BTreeMap::new(),
            impl_traits: RefCell::new(BTreeMap::new()),
            in_progress: RefCell::new(BTreeSet::new()),
            memo: RefCell::new(BTreeMap::new()),
            glob_memo: RefCell::new(BTreeMap::new()),
        };
        resolver.load_units(manifests);
        for unit in 0..resolver.units.len() {
            resolver.load_module_tree(unit);
        }
        resolver.load_impls();
        resolver
    }

    // ── Crates ────────────────────────────────────────────────────────

    fn load_units(&mut self, manifests: &BTreeMap<String, String>) {
        let parsed: BTreeMap<String, toml::Value> = manifests
            .iter()
            .filter(|(path, _)| path.as_str() == "Cargo.toml" || path.ends_with("/Cargo.toml"))
            .filter_map(|(path, text)| {
                Some((manifest_dir(path), text.parse::<toml::Value>().ok()?))
            })
            .collect();
        for (dir, manifest) in &parsed {
            let Some(package) = manifest.get("package") else {
                continue;
            };
            let Some(package_name) = package.get("name").and_then(toml::Value::as_str) else {
                continue;
            };
            let workspace = workspace_of(dir, &parsed);
            let mut deps = BTreeMap::new();
            for table in dependency_tables(manifest) {
                for (key, value) in table {
                    let code_name = key.replace('-', "_");
                    let dep = match dependency_dir(dir, key, value, workspace.as_ref()) {
                        Some(dep_dir) => Dep::Package(dep_dir),
                        None => Dep::External,
                    };
                    deps.insert(code_name, dep);
                }
            }
            let lib = manifest.get("lib");
            let lib_name = lib
                .and_then(|l| l.get("name"))
                .and_then(toml::Value::as_str)
                .map_or_else(|| package_name.replace('-', "_"), str::to_string);
            let lib_path = lib
                .and_then(|l| l.get("path"))
                .and_then(toml::Value::as_str)
                .map_or_else(|| join(dir, "src/lib.rs"), |p| join(dir, p));
            let has_lib = self.index.file_by_path.contains_key(lib_path.as_str());
            let mut roots: Vec<(String, String, bool)> = Vec::new();
            if has_lib {
                roots.push((lib_name.clone(), lib_path, true));
            }
            for (section, auto_dirs) in [
                ("bin", &["src/bin"][..]),
                ("test", &["tests"][..]),
                ("example", &["examples"][..]),
                ("bench", &["benches"][..]),
            ] {
                let mut declared = BTreeSet::new();
                if let Some(targets) = manifest.get(section).and_then(toml::Value::as_array) {
                    for target in targets {
                        let name = target.get("name").and_then(toml::Value::as_str);
                        let path = target.get("path").and_then(toml::Value::as_str);
                        if let (Some(name), Some(path)) = (name, path) {
                            let path = join(dir, path);
                            declared.insert(path.clone());
                            roots.push((name.replace('-', "_"), path, false));
                        }
                    }
                }
                for auto_dir in auto_dirs {
                    let base = join(dir, auto_dir);
                    for path in self.auto_targets(&base) {
                        if declared.insert(path.clone()) {
                            let name = target_name(&base, &path);
                            roots.push((name, path, false));
                        }
                    }
                }
                if section == "bin" {
                    let main = join(dir, "src/main.rs");
                    if self.index.file_by_path.contains_key(main.as_str())
                        && declared.insert(main.clone())
                    {
                        roots.push((package_name.replace('-', "_"), main, false));
                    }
                }
            }
            for (_target_name, root_file, is_lib) in roots {
                self.units.push(Unit {
                    root_file,
                    package_dir: dir.clone(),
                    deps: deps.clone(),
                    own_lib: has_lib.then(|| lib_name.clone()),
                    is_lib,
                    root_module: usize::MAX,
                });
                if is_lib {
                    self.package_lib.insert(dir.clone(), self.units.len() - 1);
                }
            }
        }
    }

    /// `<dir>/*.rs` and `<dir>/*/main.rs` files of the tree.
    fn auto_targets(&self, dir: &str) -> Vec<String> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        self.index
            .file_by_path
            .keys()
            .filter_map(|path| {
                let rest = path.strip_prefix(&prefix)?;
                let parts: Vec<&str> = rest.split('/').collect();
                match parts.as_slice() {
                    [file] if file.ends_with(".rs") => Some((*path).to_string()),
                    [_, "main.rs"] => Some((*path).to_string()),
                    _ => None,
                }
            })
            .collect()
    }

    // ── Module tree ───────────────────────────────────────────────────

    fn load_module_tree(&mut self, unit: usize) {
        let root_file = self.units[unit].root_file.clone();
        let Some(&file) = self.index.file_by_path.get(root_file.as_str()) else {
            return;
        };
        let root = self.add_file_module(unit, file, None, parent_dir(&root_file));
        self.units[unit].root_module = root;
    }

    /// Add the module of `file` (and its inline and declared submodules).
    /// `mod_dir` is the directory its `mod x;` declarations look in.
    fn add_file_module(
        &mut self,
        unit: usize,
        file: usize,
        parent: Option<usize>,
        mod_dir: String,
    ) -> usize {
        // A file reached twice by one crate keeps its first module.
        if let Some(existing) = self
            .scopes
            .get(&(file, None))
            .and_then(|ids| ids.iter().find(|&&m| self.modules[m].unit == unit))
        {
            return *existing;
        }
        let module = self.add_module(unit, file, None, parent);
        self.add_inline_modules(unit, file, None, module, &mod_dir);
        module
    }

    fn add_module(
        &mut self,
        unit: usize,
        file: usize,
        container: Option<usize>,
        parent: Option<usize>,
    ) -> usize {
        let facts = self.index.file(file);
        let mut items: BTreeMap<String, Vec<Res>> = BTreeMap::new();
        for (i, def) in facts.definitions.iter().enumerate() {
            if def.container == container {
                items
                    .entry(def.name.clone())
                    .or_default()
                    .push(Res::Def((file, i)));
            }
        }
        let mut uses = Vec::new();
        for import in &facts.imports {
            if import.container == container {
                flatten_use(import, &[], &mut uses);
            }
        }
        let id = self.modules.len();
        self.modules.push(Module {
            unit,
            file,
            container,
            parent,
            items,
            uses,
        });
        self.scopes.entry((file, container)).or_default().push(id);
        id
    }

    /// The inline `mod x { }` containers directly inside `container` of
    /// `file`, and the `mod x;` declarations there, as children of `module`.
    fn add_inline_modules(
        &mut self,
        unit: usize,
        file: usize,
        container: Option<usize>,
        module: usize,
        mod_dir: &str,
    ) {
        let facts: &'t FileFacts = self.index.file(file);
        for (c, fact) in facts.containers.iter().enumerate() {
            if fact.parent == container && fact.kind == ContainerFactKind::Module {
                let child = self.add_module(unit, file, Some(c), Some(module));
                self.modules[module]
                    .items
                    .entry(fact.name.clone())
                    .or_default()
                    .push(Res::Module(child));
                let child_dir = join(mod_dir, &fact.name);
                self.add_inline_modules(unit, file, Some(c), child, &child_dir);
            }
        }
        for decl in facts
            .module_declarations
            .iter()
            .filter(|d| d.container == container)
        {
            let candidates = match &decl.path_attribute {
                Some(path) => {
                    let base = if container.is_none() {
                        parent_dir(&facts.path)
                    } else {
                        mod_dir.to_string()
                    };
                    vec![join(&base, path)]
                }
                None => vec![
                    join(mod_dir, &format!("{}.rs", decl.name)),
                    join(mod_dir, &format!("{}/mod.rs", decl.name)),
                ],
            };
            let found: Vec<usize> = candidates
                .iter()
                .filter_map(|c| self.index.file_by_path.get(c.as_str()).copied())
                .collect();
            let [child_file] = found.as_slice() else {
                continue;
            };
            let child_path = &self.index.file(*child_file).path;
            let child_dir = if child_path.ends_with("/mod.rs") || decl.path_attribute.is_some() {
                parent_dir(child_path)
            } else {
                join(mod_dir, &decl.name)
            };
            let child = self.add_file_module(unit, *child_file, Some(module), child_dir);
            self.modules[module]
                .items
                .entry(decl.name.clone())
                .or_default()
                .push(Res::Module(child));
        }
    }

    fn load_impls(&mut self) {
        let mut impl_self = BTreeMap::new();
        let mut impls: BTreeMap<DefRef, Vec<ImplInfo>> = BTreeMap::new();
        for (file, facts) in self.index.tree.files.iter().enumerate() {
            if facts.language != "rust" {
                continue;
            }
            for (c, container) in facts.containers.iter().enumerate() {
                if container.kind != ContainerFactKind::Impl {
                    continue;
                }
                let Some(self_type) = &container.self_type else {
                    continue;
                };
                let answer = self.in_scopes(file, container.parent, |module| {
                    self.resolve_path(module, self_type, Ns::Type, (file, container.parent))
                });
                let answer = answer.and_then(|res| match res {
                    Res::Def(def) if matches!(self.index.def(def).entity, Entity::Type(_)) => {
                        Ok(def)
                    }
                    Res::External => Err(Reason::External),
                    _ => Err(Reason::NotFound),
                });
                if let Ok(def) = answer {
                    impls
                        .entry(def)
                        .or_default()
                        .push(ImplInfo { file, container: c });
                }
                impl_self.insert((file, c), answer);
            }
        }
        self.impl_self = impl_self;
        self.impls = impls;
    }

    // ── Scopes ────────────────────────────────────────────────────────

    /// Run `f` in every module the scope at (`file`, `container`) is, and
    /// combine: one answer when the crates agree.
    fn in_scopes<F>(&self, file: usize, container: Option<usize>, f: F) -> Result<Res, Reason>
    where
        F: Fn(usize) -> Result<Res, Reason>,
    {
        let facts = self.index.file(file);
        let module_container = enclosing(facts, container, &[ContainerFactKind::Module]);
        let Some(modules) = self.scopes.get(&(file, module_container)) else {
            return Err(Reason::NotInModuleTree);
        };
        let mut found = BTreeSet::new();
        let mut first_error = None;
        for &module in modules {
            match f(module) {
                Ok(res) => {
                    found.insert(res);
                }
                Err(reason) => {
                    first_error.get_or_insert(reason);
                }
            }
        }
        let mut iter = found.into_iter();
        match (iter.next(), iter.next()) {
            (Some(one), None) => Ok(one),
            (Some(_), Some(_)) => Err(Reason::Ambiguous),
            (None, _) => Err(first_error.unwrap_or(Reason::NotFound)),
        }
    }

    /// Names declared in `module`, then bound by its `use` declarations.
    fn items(&self, module: usize, name: &str, ns: Ns) -> Result<BTreeSet<Res>, Reason> {
        let key = (module, name.to_string(), ns);
        if let Some(cached) = self.memo.borrow().get(&key) {
            return cached.clone();
        }
        let mut found = BTreeSet::new();
        if let Some(items) = self.modules[module].items.get(name) {
            found.extend(items.iter().copied().filter(|r| self.in_ns(*r, ns)));
        }
        let mut first_error = None;
        for (b, binding) in self.modules[module].uses.iter().enumerate() {
            if binding.name.as_deref() != Some(name) {
                continue;
            }
            if !self.in_progress.borrow_mut().insert((module, b)) {
                continue;
            }
            let answer = self.resolve_path(module, &binding.path, ns, self.module_scope(module));
            self.in_progress.borrow_mut().remove(&(module, b));
            match answer {
                Ok(res) => {
                    found.insert(res);
                }
                Err(reason) => {
                    first_error.get_or_insert(reason);
                }
            }
        }
        let result = if found.is_empty() {
            if let Some(Reason::External) = first_error {
                found.insert(Res::External);
            }
            Ok(found)
        } else {
            Ok(found)
        };
        self.memo.borrow_mut().insert(key, result.clone());
        result
    }

    /// Names glob-imported into `module`.
    fn glob_items(&self, module: usize, name: &str, ns: Ns) -> BTreeSet<Res> {
        let key = (module, name.to_string(), ns);
        if let Some(cached) = self.glob_memo.borrow().get(&key) {
            return cached.clone();
        }
        let found = self.resolve_glob_items(module, name, ns);
        self.glob_memo.borrow_mut().insert(key, found.clone());
        found
    }

    fn resolve_glob_items(&self, module: usize, name: &str, ns: Ns) -> BTreeSet<Res> {
        let mut found = BTreeSet::new();
        for (b, binding) in self.modules[module].uses.iter().enumerate() {
            if binding.name.is_some() {
                continue;
            }
            if !self.in_progress.borrow_mut().insert((module, b)) {
                continue;
            }
            let target =
                self.resolve_path(module, &binding.path, Ns::Type, self.module_scope(module));
            if let Ok(res) = target {
                match res {
                    Res::Module(m) => {
                        if let Ok(items) = self.items(m, name, ns) {
                            found.extend(items);
                        }
                        found.extend(self.glob_items(m, name, ns));
                    }
                    Res::Unit(u) => {
                        let m = self.units[u].root_module;
                        if let Ok(items) = self.items(m, name, ns) {
                            found.extend(items);
                        }
                    }
                    Res::Def(def) => {
                        if self.index.def(def).members.iter().any(|m| m == name) {
                            found.insert(Res::Def(def));
                        }
                    }
                    Res::External => {}
                }
            }
            self.in_progress.borrow_mut().remove(&(module, b));
        }
        found
    }

    fn module_scope(&self, module: usize) -> (usize, Option<usize>) {
        (self.modules[module].file, self.modules[module].container)
    }

    fn in_ns(&self, res: Res, ns: Ns) -> bool {
        match (res, ns) {
            (_, Ns::Any) | (Res::External, _) => true,
            (Res::Module(_) | Res::Unit(_), Ns::Type) => true,
            (Res::Module(_) | Res::Unit(_), Ns::Value) => false,
            (Res::Def(def), Ns::Type) => matches!(self.index.def(def).entity, Entity::Type(_)),
            (Res::Def(def), Ns::Value) => matches!(
                self.index.def(def).entity,
                Entity::Function | Entity::Type(TypeKind::Struct)
            ),
        }
    }

    /// The first segment of a path, seen from `module`.
    fn first(&self, module: usize, name: &str, ns: Ns) -> Result<Res, Reason> {
        let declared = self.items(module, name, ns)?;
        if !declared.is_empty() {
            return one(declared);
        }
        let globbed = self.glob_items(module, name, ns);
        if !globbed.is_empty() {
            return one(globbed);
        }
        if ns != Ns::Value {
            let unit = &self.units[self.modules[module].unit];
            let dep = unit.deps.get(name).cloned().or_else(|| {
                (!unit.is_lib && unit.own_lib.as_deref() == Some(name))
                    .then(|| Dep::Package(unit.package_dir.clone()))
            });
            match dep {
                Some(Dep::Package(dir)) => {
                    return match self.package_lib.get(&dir) {
                        Some(&u) => Ok(Res::Unit(u)),
                        None => Err(Reason::NotFound),
                    };
                }
                Some(Dep::External) => return Err(Reason::External),
                None => {}
            }
        }
        if PRELUDE.contains(&name) {
            Err(Reason::External)
        } else {
            Err(Reason::NotFound)
        }
    }

    /// Resolve `path` from `module`; `scope` is the (file, container) the
    /// path is written in, for `Self`.
    fn resolve_path(
        &self,
        module: usize,
        path: &[String],
        ns: Ns,
        scope: (usize, Option<usize>),
    ) -> Result<Res, Reason> {
        let Some((head, rest)) = path.split_first() else {
            return Err(Reason::NotFound);
        };
        let head_ns = if rest.is_empty() { ns } else { Ns::Type };
        let mut current = match head.as_str() {
            "crate" => Res::Module(self.units[self.modules[module].unit].root_module),
            "self" => Res::Module(module),
            "super" => Res::Module(self.modules[module].parent.ok_or(Reason::NotFound)?),
            "Self" => Res::Def(self.self_type(scope.0, scope.1)?),
            _ => self.first(module, head, head_ns)?,
        };
        for (i, segment) in rest.iter().enumerate() {
            let segment_ns = if i + 1 == rest.len() { ns } else { Ns::Type };
            current = match current {
                Res::External => return Err(Reason::External),
                Res::Unit(u) => self.step_module(self.units[u].root_module, segment, segment_ns)?,
                Res::Module(m) if segment == "super" => {
                    Res::Module(self.modules[m].parent.ok_or(Reason::NotFound)?)
                }
                Res::Module(m) => self.step_module(m, segment, segment_ns)?,
                Res::Def(def) => self.associated(def, segment)?,
            };
        }
        if let Res::External = current {
            return Err(Reason::External);
        }
        Ok(current)
    }

    fn step_module(&self, module: usize, name: &str, ns: Ns) -> Result<Res, Reason> {
        let declared = self.items(module, name, ns)?;
        if !declared.is_empty() {
            return one(declared);
        }
        let globbed = self.glob_items(module, name, ns);
        if !globbed.is_empty() {
            return one(globbed);
        }
        Err(Reason::NotFound)
    }

    // ── Types and methods ─────────────────────────────────────────────

    /// The type `Self` and `self` denote at (`file`, `container`).
    fn self_type(&self, file: usize, container: Option<usize>) -> Result<DefRef, Reason> {
        let facts = self.index.file(file);
        let Some(c) = enclosing(
            facts,
            container,
            &[ContainerFactKind::Impl, ContainerFactKind::Type],
        ) else {
            return Err(Reason::ReceiverTypeUnknown);
        };
        match facts.containers[c].kind {
            ContainerFactKind::Impl => self
                .impl_self
                .get(&(file, c))
                .cloned()
                .unwrap_or(Err(Reason::NotFound)),
            _ => facts.containers[c]
                .definition
                .map(|d| (file, d))
                .ok_or(Reason::ReceiverTypeUnknown),
        }
    }

    /// `Type::name`: a variant, or a method of the type (inherent impls
    /// first, then trait impls, then the provided methods of its traits).
    fn associated(&self, def: DefRef, name: &str) -> Result<Res, Reason> {
        let target = self.index.def(def);
        if target.members.iter().any(|m| m == name) {
            return Ok(Res::Def(def));
        }
        self.method(def, name).map(Res::Def)
    }

    fn method(&self, def: DefRef, name: &str) -> Answer {
        let target = self.index.def(def);
        if target.entity == Entity::Type(TypeKind::Trait) {
            return single(self.methods_of_trait(def, name));
        }
        let impls = self.impls.get(&def).map(Vec::as_slice).unwrap_or(&[]);
        let methods_in = |info: &ImplInfo| -> BTreeSet<DefRef> {
            self.index
                .named_in(info.file, Some(info.container), name)
                .iter()
                .map(|i| (info.file, *i))
                .filter(|d| self.index.def(*d).entity == Entity::Function)
                .collect()
        };
        let is_trait_impl = |info: &ImplInfo| {
            self.index.file(info.file).containers[info.container]
                .trait_path
                .is_some()
        };
        let inherent: BTreeSet<DefRef> = impls
            .iter()
            .filter(|i| !is_trait_impl(i))
            .flat_map(methods_in)
            .collect();
        if !inherent.is_empty() {
            return single(inherent);
        }
        let from_traits: BTreeSet<DefRef> = impls
            .iter()
            .filter(|i| is_trait_impl(i))
            .flat_map(methods_in)
            .collect();
        if !from_traits.is_empty() {
            return single(from_traits);
        }
        let mut provided = BTreeSet::new();
        let mut external_trait = !target.decorators.iter().all(|d| !d.starts_with("#[derive"));
        for info in impls.iter().filter(|i| is_trait_impl(i)) {
            match self.impl_trait(info.file, info.container) {
                Ok(trait_def) => provided.extend(self.methods_of_trait(trait_def, name)),
                Err(_) => external_trait = true,
            }
        }
        if !provided.is_empty() {
            return single(provided);
        }
        Err(if external_trait {
            Reason::External
        } else {
            Reason::NotFound
        })
    }

    fn methods_of_trait(&self, trait_def: DefRef, name: &str) -> BTreeSet<DefRef> {
        let Some(container) = self.index.container_of(trait_def) else {
            return BTreeSet::new();
        };
        self.index
            .named_in(trait_def.0, Some(container), name)
            .iter()
            .map(|i| (trait_def.0, *i))
            .filter(|d| self.index.def(*d).entity == Entity::Function)
            .collect()
    }

    fn impl_trait(&self, file: usize, container: usize) -> Answer {
        if let Some(answer) = self.impl_traits.borrow().get(&(file, container)) {
            return *answer;
        }
        let answer = self.resolve_impl_trait(file, container);
        self.impl_traits
            .borrow_mut()
            .insert((file, container), answer);
        answer
    }

    fn resolve_impl_trait(&self, file: usize, container: usize) -> Answer {
        let facts = self.index.file(file);
        let fact = &facts.containers[container];
        let Some(trait_path) = &fact.trait_path else {
            return Err(Reason::NotFound);
        };
        let res = self.in_scopes(file, fact.parent, |module| {
            self.resolve_path(module, trait_path, Ns::Type, (file, fact.parent))
        })?;
        match res {
            Res::Def(def) if self.index.def(def).entity == Entity::Type(TypeKind::Trait) => Ok(def),
            _ => Err(Reason::NotFound),
        }
    }

    /// A plain name defined inside the enclosing functions, innermost first.
    fn block_item(
        &self,
        file: usize,
        container: Option<usize>,
        name: &str,
        ns: Ns,
    ) -> Option<Answer> {
        let facts = self.index.file(file);
        let mut current = container;
        while let Some(c) = current {
            if facts.containers[c].kind != ContainerFactKind::Function {
                return None;
            }
            let found: BTreeSet<DefRef> = self
                .index
                .named_in(file, Some(c), name)
                .iter()
                .map(|i| (file, *i))
                .filter(|def| self.in_ns(Res::Def(*def), ns))
                .collect();
            if !found.is_empty() {
                return Some(single(found));
            }
            current = facts.containers[c].parent;
        }
        None
    }

    fn resolve_in_scope(
        &self,
        file: usize,
        container: Option<usize>,
        path: &[String],
        ns: Ns,
    ) -> Answer {
        if let [name] = path {
            if let Some(answer) = self.block_item(file, container, name, ns) {
                return answer;
            }
        }
        let res = self.in_scopes(file, container, |module| {
            self.resolve_path(module, path, ns, (file, container))
        })?;
        match res {
            Res::Def(def) => Ok(def),
            Res::External => Err(Reason::External),
            Res::Module(_) | Res::Unit(_) => Err(Reason::NotFound),
        }
    }
}

fn one(found: BTreeSet<Res>) -> Result<Res, Reason> {
    let mut iter = found.into_iter();
    match (iter.next(), iter.next()) {
        (Some(Res::External), None) => Err(Reason::External),
        (Some(res), None) => Ok(res),
        (Some(_), Some(_)) => Err(Reason::Ambiguous),
        (None, _) => Err(Reason::NotFound),
    }
}

impl LanguageResolver for RustResolver<'_, '_> {
    fn call(&self, file: usize, call: &CallSite) -> Answer {
        let facts = self.index.file(file);
        let container = self.index.body_scope((file, call.caller));
        if !self.scopes.contains_key(&(
            file,
            enclosing(facts, container, &[ContainerFactKind::Module]),
        )) {
            return Err(Reason::NotInModuleTree);
        }
        match &call.callee {
            Callee::Path(path) => self.resolve_in_scope(file, container, path, Ns::Value),
            Callee::Method {
                receiver: Receiver::SelfValue,
                name,
            } => {
                let owner = self.self_type(file, container)?;
                self.method(owner, name)
            }
            other => Err(non_path_call(other).unwrap_or(Reason::NotFound)),
        }
    }

    fn type_ref(&self, file: usize, function: usize, target: &[String]) -> Answer {
        let container = self.index.body_scope((file, function));
        if target == ["Self"] {
            return self.self_type(file, container);
        }
        self.resolve_in_scope(file, container, target, Ns::Type)
    }

    fn bases(&self, file: usize, out: &mut Vec<(DefRef, BaseRelation, Answer)>) {
        let facts = self.index.file(file);
        for (c, container) in facts.containers.iter().enumerate() {
            if container.kind != ContainerFactKind::Impl || container.trait_path.is_none() {
                continue;
            }
            if let Some(Ok(owner)) = self.impl_self.get(&(file, c)) {
                out.push((*owner, BaseRelation::Implements, self.impl_trait(file, c)));
            }
        }
        for base in &facts.bases {
            let BaseOwner::Definition(owner) = base.owner else {
                continue;
            };
            let container = facts.definitions[owner].container;
            let answer = self.resolve_in_scope(file, container, &base.target, Ns::Type);
            out.push(((file, owner), base.relation, answer));
        }
    }

    fn method_owner(&self, file: usize, container: usize) -> Option<DefRef> {
        let facts = self.index.file(file);
        match facts.containers[container].kind {
            ContainerFactKind::Impl => self.impl_self.get(&(file, container)).and_then(|a| a.ok()),
            _ => facts.containers[container].definition.map(|d| (file, d)),
        }
    }

    fn imports(&self, file: usize, out: &mut Vec<Result<ImportEdge, Reason>>) {
        let facts = self.index.file(file);
        let mut containers: BTreeSet<Option<usize>> = BTreeSet::new();
        containers.insert(None);
        for (c, fact) in facts.containers.iter().enumerate() {
            if fact.kind == ContainerFactKind::Module {
                containers.insert(Some(c));
            }
        }
        for container in containers {
            let Some(modules) = self.scopes.get(&(file, container)) else {
                if container.is_none() {
                    for _ in &facts.imports {
                        out.push(Err(Reason::NotInModuleTree));
                    }
                }
                continue;
            };
            let Some(&first) = modules.first() else {
                continue;
            };
            for b in 0..self.modules[first].uses.len() {
                let binding = &self.modules[first].uses[b];
                let answer = self.in_scopes(file, container, |module| {
                    let binding = &self.modules[module].uses[b];
                    self.resolve_path(module, &binding.path, Ns::Any, (file, container))
                });
                let to_file = answer.and_then(|res| match res {
                    Res::Def(def) => Ok(def.0),
                    Res::Module(m) => Ok(self.modules[m].file),
                    Res::Unit(u) => Ok(self.modules[self.units[u].root_module].file),
                    Res::External => Err(Reason::External),
                });
                out.push(to_file.map(|to_file| ImportEdge {
                    from_file: file,
                    to_file,
                    import_path: binding.path.join("::"),
                }));
            }
        }
    }
}

// ── use trees ─────────────────────────────────────────────────────────

fn flatten_use(node: &Syntax, prefix: &[String], out: &mut Vec<UseBinding>) {
    let children: Vec<&Syntax> = node.children.iter().collect();
    match node.kind.as_str() {
        "use_declaration" => {
            for child in children
                .into_iter()
                .filter(|c| c.kind != "visibility_modifier")
            {
                flatten_use(child, prefix, out);
            }
        }
        "use_list" => {
            for child in children {
                flatten_use(child, prefix, out);
            }
        }
        "scoped_use_list" => {
            let (path, list) = match children.as_slice() {
                [path, list] => (segments(path), *list),
                [list] => (Vec::new(), *list),
                _ => return,
            };
            let mut full = prefix.to_vec();
            full.extend(path);
            flatten_use(list, &full, out);
        }
        "use_as_clause" => {
            if let [path, alias] = children.as_slice() {
                let mut full = prefix.to_vec();
                full.extend(
                    segments(path)
                        .into_iter()
                        .filter(|s| s != "self" || prefix.is_empty()),
                );
                out.push(UseBinding {
                    name: Some(alias.text.clone()),
                    path: full,
                });
            }
        }
        "use_wildcard" => {
            let mut full = prefix.to_vec();
            if let Some(path) = children.first() {
                full.extend(segments(path));
            }
            out.push(UseBinding {
                name: None,
                path: full,
            });
        }
        _ => {
            let mut full = prefix.to_vec();
            let path = segments(node);
            if path == ["self"] && !prefix.is_empty() {
                // use a::{self}: binds `a`.
            } else {
                full.extend(path);
            }
            if let Some(name) = full.last().cloned() {
                out.push(UseBinding {
                    name: Some(name),
                    path: full,
                });
            }
        }
    }
}

fn segments(node: &Syntax) -> Vec<String> {
    match node.kind.as_str() {
        "identifier" | "crate" | "self" | "super" | "type_identifier" => vec![node.text.clone()],
        "scoped_identifier" => node.children.iter().flat_map(segments).collect(),
        _ => Vec::new(),
    }
}

// ── manifests ─────────────────────────────────────────────────────────

fn manifest_dir(path: &str) -> String {
    path.strip_suffix("Cargo.toml")
        .unwrap_or(path)
        .trim_end_matches('/')
        .to_string()
}

fn workspace_of(
    dir: &str,
    manifests: &BTreeMap<String, toml::Value>,
) -> Option<(String, toml::Value)> {
    let mut current = Some(dir.to_string());
    while let Some(d) = current {
        if let Some(manifest) = manifests.get(&d) {
            if let Some(workspace) = manifest.get("workspace") {
                return Some((d, workspace.clone()));
            }
        }
        current = if d.is_empty() {
            None
        } else {
            Some(parent_dir(&d))
        };
    }
    None
}

fn dependency_tables(manifest: &toml::Value) -> Vec<&toml::map::Map<String, toml::Value>> {
    let mut tables = Vec::new();
    for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(t) = manifest.get(key).and_then(toml::Value::as_table) {
            tables.push(t);
        }
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(t) = target.get(key).and_then(toml::Value::as_table) {
                    tables.push(t);
                }
            }
        }
    }
    tables
}

/// The package directory a dependency points at, when it is under the
/// root: its `path`, or for `workspace = true` the `path` of the same key in
/// `[workspace.dependencies]`, relative to the workspace manifest.
fn dependency_dir(
    dir: &str,
    key: &str,
    value: &toml::Value,
    workspace: Option<&(String, toml::Value)>,
) -> Option<String> {
    if let Some(path) = value.get("path").and_then(toml::Value::as_str) {
        return normalize(&join(dir, path));
    }
    if value.get("workspace").and_then(toml::Value::as_bool) == Some(true) {
        let (ws_dir, ws) = workspace?;
        let path = ws.get("dependencies")?.get(key)?.get("path")?.as_str()?;
        return normalize(&join(ws_dir, path));
    }
    None
}

fn target_name(base: &str, path: &str) -> String {
    let rest = path
        .strip_prefix(base)
        .unwrap_or(path)
        .trim_start_matches('/');
    let name = rest
        .strip_suffix("/main.rs")
        .or_else(|| rest.strip_suffix(".rs"))
        .unwrap_or(rest);
    name.replace('-', "_")
}

fn parent_dir(path: &str) -> String {
    path.rsplit_once('/')
        .map_or(String::new(), |(dir, _)| dir.to_string())
}

fn join(dir: &str, rel: &str) -> String {
    let joined = if dir.is_empty() {
        rel.to_string()
    } else {
        format!("{dir}/{rel}")
    };
    normalize(&joined).unwrap_or(joined)
}

/// Resolve `.` and `..` segments; `None` when the path leaves the root.
fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}
