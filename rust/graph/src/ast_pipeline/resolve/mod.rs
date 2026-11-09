//! The global resolve pass (KGRAPH-075).
//!
//! After the facts of every file exist, this pass builds every edge from
//! the facts of the whole tree: Imports, Calls, TypeRef, Implements,
//! Extends and HasMethod. It is pure and in-memory: the same facts and
//! manifests always give the same entities, in the same order. A reference
//! becomes an edge only when the language's rules name exactly one
//! definition; any other reference is counted as unresolved with a reason.

mod generic;
mod imports;
mod rust;

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::{Map, Value};

use super::facts::{
    BaseRelation, CallSite, Callee, ContainerFactKind, Definition, Entity, FileFacts, RefKind,
    TreeFacts,
};
use crate::graph_entities::GraphEntity;

/// A definition: (file index, definition index) into `TreeFacts::files`.
pub type DefRef = (usize, usize);

/// Why a reference is not an edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// It names something outside the root (a dependency, the standard
    /// library, a language built-in).
    External,
    /// Nothing in scope has that name.
    NotFound,
    /// More than one definition has that name in scope.
    Ambiguous,
    /// A method on a receiver whose type is not known from facts.
    ReceiverTypeUnknown,
    /// Written inside a macro's token tree, which the grammar does not parse.
    MacroTokenTree,
    /// The file belongs to no crate's module tree.
    NotInModuleTree,
    /// The callee is an expression, not a path (a closure or a call result).
    CalleeExpression,
}

/// The kind of a reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKindCount {
    Calls,
    Imports,
    TypeRefs,
    Bases,
}

/// Unresolved reference counts: kind -> reason -> count.
pub type UnresolvedCounts = BTreeMap<RefKindCount, BTreeMap<Reason, u64>>;

/// What a language resolver answers for one reference.
pub type Answer = Result<DefRef, Reason>;

/// A file-to-file import found by a language resolver.
pub struct ImportEdge {
    pub from_file: usize,
    pub to_file: usize,
    pub import_path: String,
}

/// The per-language half of the resolve pass.
pub(crate) trait LanguageResolver {
    fn call(&self, file: usize, call: &CallSite) -> Answer;
    fn type_ref(&self, file: usize, function: usize, target: &[String]) -> Answer;
    /// Bases of definitions and containers: (owner type, relation, base).
    fn bases(&self, file: usize, out: &mut Vec<(DefRef, BaseRelation, Answer)>);
    /// The type each method-holding container belongs to.
    fn method_owner(&self, file: usize, container: usize) -> Option<DefRef>;
    fn imports(&self, file: usize, out: &mut Vec<Result<ImportEdge, Reason>>);
}

/// The resolved graph of a tree.
pub struct Graph {
    pub entities: Vec<GraphEntity>,
    pub unresolved: UnresolvedCounts,
}

/// Resolve `tree` into graph entities. `manifests` maps the relative path
/// of every package manifest under the root (Cargo.toml, ...) to its text.
pub fn build_graph(tree: &TreeFacts, manifests: &BTreeMap<String, String>) -> Graph {
    let index = TreeIndex::new(tree);
    let rust = rust::RustResolver::new(&index, manifests);
    let mut unresolved = UnresolvedCounts::new();
    let mut count = |kind: RefKindCount, reason: Reason| {
        *unresolved
            .entry(kind)
            .or_default()
            .entry(reason)
            .or_default() += 1;
    };

    let mut calls: BTreeMap<(DefRef, DefRef), (u32, bool)> = BTreeMap::new();
    let mut type_refs: BTreeSet<(DefRef, DefRef, RefKind)> = BTreeSet::new();
    let mut bases: BTreeSet<(DefRef, BaseRelation, DefRef)> = BTreeSet::new();
    let mut has_method: BTreeSet<(DefRef, DefRef)> = BTreeSet::new();
    let mut imports: BTreeMap<(usize, usize), BTreeSet<String>> = BTreeMap::new();

    for (file_index, file) in tree.files.iter().enumerate() {
        let generic;
        let resolver: &dyn LanguageResolver = if file.language == "rust" {
            &rust
        } else {
            generic = generic::GenericResolver::new(&index, file_index, manifests);
            &generic
        };
        for call in &file.calls {
            let caller = (file_index, call.caller);
            match resolver.call(file_index, call) {
                Ok(callee) if is_function(&index, callee) => {
                    let entry = calls.entry((caller, callee)).or_insert((0, true));
                    entry.0 += 1;
                    entry.1 &= call.conditional;
                }
                // A path naming a type or a variant constructs it: a
                // TypeRef, not a call.
                Ok(target) => {
                    type_refs.insert((caller, target, RefKind::Constructor));
                }
                Err(reason) => count(RefKindCount::Calls, reason),
            }
        }
        for site in &file.type_refs {
            match resolver.type_ref(file_index, site.function, &site.target) {
                Ok(target) if !is_function(&index, target) => {
                    type_refs.insert(((file_index, site.function), target, site.ref_kind));
                }
                Ok(_) => count(RefKindCount::TypeRefs, Reason::NotFound),
                Err(reason) => count(RefKindCount::TypeRefs, reason),
            }
        }
        let mut found = Vec::new();
        resolver.bases(file_index, &mut found);
        for (owner, relation, answer) in found {
            match answer {
                Ok(base) if !is_function(&index, base) && base != owner => {
                    bases.insert((owner, relation, base));
                }
                Ok(_) => count(RefKindCount::Bases, Reason::NotFound),
                Err(reason) => count(RefKindCount::Bases, reason),
            }
        }
        for (def_index, def) in file.definitions.iter().enumerate() {
            if def.entity != Entity::Function {
                continue;
            }
            let Some(container) = def.container else {
                continue;
            };
            let kind = file.containers[container].kind;
            if matches!(kind, ContainerFactKind::Impl | ContainerFactKind::Type) {
                if let Some(owner) = resolver.method_owner(file_index, container) {
                    has_method.insert((owner, (file_index, def_index)));
                }
            }
        }
        let mut found = Vec::new();
        resolver.imports(file_index, &mut found);
        for answer in found {
            match answer {
                Ok(edge) if edge.from_file != edge.to_file => {
                    imports
                        .entry((edge.from_file, edge.to_file))
                        .or_default()
                        .insert(edge.import_path);
                }
                Ok(_) => {}
                Err(reason) => count(RefKindCount::Imports, reason),
            }
        }
    }

    let mut entities = Vec::new();
    for (file_index, file) in tree.files.iter().enumerate() {
        emit_file(&index, file_index, file, &mut entities);
    }
    for ((from, to), paths) in imports {
        let mut props = Map::new();
        props.insert(
            "importPath".into(),
            Value::String(paths.into_iter().collect::<Vec<_>>().join(", ")),
        );
        entities.push(edge(
            "Imports",
            file_slug(&tree.files[from]),
            file_slug(&tree.files[to]),
            props,
        ));
    }
    for ((caller, callee), (call_count, conditional)) in calls {
        let mut props = Map::new();
        props.insert("callCount".into(), Value::from(call_count));
        props.insert("isConditional".into(), Value::Bool(conditional));
        entities.push(edge("Calls", index.slug(caller), index.slug(callee), props));
    }
    for (function, target, ref_kind) in type_refs {
        let mut props = Map::new();
        props.insert(
            "refKind".into(),
            Value::String(ref_kind.schema_name().into()),
        );
        entities.push(edge(
            "TypeRef",
            index.slug(function),
            index.slug(target),
            props,
        ));
    }
    for (owner, relation, base) in bases {
        let name = match relation {
            BaseRelation::Extends => "Extends",
            BaseRelation::Implements => "Implements",
        };
        entities.push(edge(name, index.slug(owner), index.slug(base), Map::new()));
    }
    for (owner, method) in has_method {
        entities.push(edge(
            "HasMethod",
            index.slug(owner),
            index.slug(method),
            Map::new(),
        ));
    }
    Graph {
        entities,
        unresolved,
    }
}

fn is_function(index: &TreeIndex<'_>, def: DefRef) -> bool {
    index.def(def).entity == Entity::Function
}

/// Container (None: top level) -> name -> definitions, for one file.
type NamesByContainer<'t> = BTreeMap<Option<usize>, BTreeMap<&'t str, Vec<usize>>>;

/// Lookups over the facts of the whole tree.
pub(crate) struct TreeIndex<'t> {
    pub tree: &'t TreeFacts,
    pub file_by_path: BTreeMap<&'t str, usize>,
    /// Qualified name (path::chain::name) of every definition.
    qualified: Vec<Vec<String>>,
    /// Per file: container (None: top level) -> name -> definitions.
    named: Vec<NamesByContainer<'t>>,
    /// Per file: definition -> the container it is (class, trait, fn body).
    own_container: Vec<BTreeMap<usize, usize>>,
}

impl<'t> TreeIndex<'t> {
    fn new(tree: &'t TreeFacts) -> Self {
        let file_by_path = tree
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.as_str(), i))
            .collect();
        let qualified = tree
            .files
            .iter()
            .map(|file| {
                file.definitions
                    .iter()
                    .map(|def| {
                        let mut parts = vec![file.path.clone()];
                        parts.extend(container_chain(file, def.container));
                        parts.push(def.name.clone());
                        parts.join("::")
                    })
                    .collect()
            })
            .collect();
        let named = tree
            .files
            .iter()
            .map(|file| {
                let mut map: BTreeMap<Option<usize>, BTreeMap<&'t str, Vec<usize>>> =
                    BTreeMap::new();
                for (i, def) in file.definitions.iter().enumerate() {
                    map.entry(def.container)
                        .or_default()
                        .entry(def.name.as_str())
                        .or_default()
                        .push(i);
                }
                map
            })
            .collect();
        let own_container = tree
            .files
            .iter()
            .map(|file| {
                file.containers
                    .iter()
                    .enumerate()
                    .filter_map(|(c, container)| container.definition.map(|d| (d, c)))
                    .collect()
            })
            .collect();
        TreeIndex {
            tree,
            file_by_path,
            qualified,
            named,
            own_container,
        }
    }

    /// The definitions named `name` directly inside `container` of `file`.
    pub fn named_in(&self, file: usize, container: Option<usize>, name: &str) -> &[usize] {
        self.named[file]
            .get(&container)
            .and_then(|names| names.get(name))
            .map_or(&[], Vec::as_slice)
    }

    /// The container a definition is (its class, trait or function body).
    pub fn container_of(&self, def: DefRef) -> Option<usize> {
        self.own_container[def.0].get(&def.1).copied()
    }

    pub fn file(&self, file: usize) -> &'t FileFacts {
        &self.tree.files[file]
    }

    /// The scope a definition's body is: its own function container, or
    /// for a definition that is no container, the container it sits in.
    pub fn body_scope(&self, def: DefRef) -> Option<usize> {
        let file = self.file(def.0);
        self.container_of(def)
            .filter(|c| file.containers[*c].kind == ContainerFactKind::Function)
            .or(file.definitions[def.1].container)
    }

    pub fn def(&self, def: DefRef) -> &'t Definition {
        &self.tree.files[def.0].definitions[def.1]
    }

    pub fn qualified_name(&self, def: DefRef) -> &str {
        &self.qualified[def.0][def.1]
    }

    /// A definition's identity: path, container chain, name and start byte.
    pub fn slug(&self, def: DefRef) -> String {
        format!("{}@{}", self.qualified_name(def), self.def(def).start_byte)
    }
}

/// The names of the containers from the outermost to `container`.
fn container_chain(file: &FileFacts, container: Option<usize>) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = container;
    while let Some(c) = current {
        chain.push(file.containers[c].name.clone());
        current = file.containers[c].parent;
    }
    chain.reverse();
    chain
}

/// A File node's key: its path relative to the root with `/` separators.
pub fn file_slug(file: &FileFacts) -> String {
    file.path.clone()
}

fn edge(edge_type: &str, from: String, to: String, properties: Map<String, Value>) -> GraphEntity {
    GraphEntity::Edge {
        edge_type: edge_type.to_string(),
        from_slug: from,
        to_slug: to,
        properties,
    }
}

fn emit_file(
    index: &TreeIndex<'_>,
    file_index: usize,
    file: &FileFacts,
    out: &mut Vec<GraphEntity>,
) {
    let slug = file_slug(file);
    let mut props = Map::new();
    props.insert("slug".into(), Value::String(slug.clone()));
    props.insert("path".into(), Value::String(file.path.clone()));
    props.insert("language".into(), Value::String(file.language.into()));
    props.insert("lineCount".into(), Value::from(file.line_count));
    props.insert("isTest".into(), Value::Bool(file.is_test));
    props.insert(
        "contentHash".into(),
        Value::String(file.content_hash.clone()),
    );
    out.push(GraphEntity::Node {
        node_type: "File".into(),
        slug: slug.clone(),
        properties: props,
    });
    for (def_index, def) in file.definitions.iter().enumerate() {
        let def_ref = (file_index, def_index);
        let def_slug = index.slug(def_ref);
        let mut props = Map::new();
        props.insert("slug".into(), Value::String(def_slug.clone()));
        props.insert("name".into(), Value::String(def.name.clone()));
        props.insert(
            "qualifiedName".into(),
            Value::String(index.qualified_name(def_ref).to_string()),
        );
        props.insert("isPublic".into(), Value::Bool(def.is_public));
        props.insert("lineStart".into(), Value::from(def.line_start));
        props.insert("lineEnd".into(), Value::from(def.line_end));
        props.insert("source".into(), Value::String(def.source.clone()));
        props.insert("docstring".into(), Value::String(def.docstring.clone()));
        props.insert(
            "decorators".into(),
            Value::String(def.decorators.join(", ")),
        );
        props.insert("language".into(), Value::String(file.language.into()));
        props.insert("truncated".into(), Value::Bool(def.truncated));
        let (node_type, contains) = match def.entity {
            Entity::Function => {
                props.insert("isAsync".into(), Value::Bool(def.is_async));
                props.insert("isTest".into(), Value::Bool(def.is_test));
                props.insert("paramCount".into(), Value::from(def.param_count));
                props.insert("cyclomaticComplexity".into(), Value::from(def.complexity));
                props.insert("parameters".into(), Value::String(def.parameters.clone()));
                ("Function", "Contains")
            }
            Entity::Type(kind) => {
                props.insert("typeKind".into(), Value::String(kind.schema_name().into()));
                ("Type", "ContainsType")
            }
        };
        out.push(GraphEntity::Node {
            node_type: node_type.into(),
            slug: def_slug.clone(),
            properties: props,
        });
        let mut edge_props = Map::new();
        edge_props.insert("lineStart".into(), Value::from(def.line_start));
        out.push(edge(contains, slug.clone(), def_slug, edge_props));
    }
    for var in &file.variables {
        let mut parts = vec![file.path.clone()];
        parts.extend(container_chain(file, var.container));
        parts.push(var.name.clone());
        let var_slug = format!("{}@{}", parts.join("::"), var.start_byte);
        let class_scope = var
            .container
            .filter(|c| file.containers[*c].kind == ContainerFactKind::Type);
        let mut props = Map::new();
        props.insert("slug".into(), Value::String(var_slug.clone()));
        props.insert("name".into(), Value::String(var.name.clone()));
        props.insert("path".into(), Value::String(file.path.clone()));
        props.insert("lineStart".into(), Value::from(var.line_start));
        props.insert("value".into(), Value::String(cap_value(&var.value)));
        props.insert(
            "scope".into(),
            Value::String(
                if class_scope.is_some() {
                    "class"
                } else {
                    "module"
                }
                .into(),
            ),
        );
        if let Some(c) = class_scope {
            props.insert(
                "scopeName".into(),
                Value::String(file.containers[c].name.clone()),
            );
        }
        props.insert("isConstant".into(), Value::Bool(var.is_constant));
        props.insert("language".into(), Value::String(file.language.into()));
        out.push(GraphEntity::Node {
            node_type: "Variable".into(),
            slug: var_slug.clone(),
            properties: props,
        });
        let mut edge_props = Map::new();
        edge_props.insert("lineStart".into(), Value::from(var.line_start));
        out.push(edge("ContainsVariable", slug.clone(), var_slug, edge_props));
    }
}

/// A variable's stored value: at most 200 bytes, cut on a character boundary.
fn cap_value(value: &str) -> String {
    const MAX: usize = 200;
    if value.len() <= MAX {
        value.to_string()
    } else {
        value[..value.floor_char_boundary(MAX)].to_string()
    }
}

/// The innermost container of `kind` around `container`, inclusive.
pub(crate) fn enclosing(
    file: &FileFacts,
    container: Option<usize>,
    kinds: &[ContainerFactKind],
) -> Option<usize> {
    let mut current = container;
    while let Some(c) = current {
        if kinds.contains(&file.containers[c].kind) {
            return Some(c);
        }
        current = file.containers[c].parent;
    }
    None
}

/// One definition, or why not.
pub(crate) fn single(candidates: BTreeSet<DefRef>) -> Answer {
    let mut iter = candidates.into_iter();
    match (iter.next(), iter.next()) {
        (Some(one), None) => Ok(one),
        (Some(_), Some(_)) => Err(Reason::Ambiguous),
        (None, _) => Err(Reason::NotFound),
    }
}

/// Calls whose callee is not a path.
pub(crate) fn non_path_call(callee: &Callee) -> Option<Reason> {
    match callee {
        Callee::InMacro(_) => Some(Reason::MacroTokenTree),
        Callee::Expression(_) => Some(Reason::CalleeExpression),
        Callee::Method {
            receiver: super::facts::Receiver::Expression,
            ..
        }
        | Callee::Method {
            receiver: super::facts::Receiver::Path(_),
            ..
        } => Some(Reason::ReceiverTypeUnknown),
        _ => None,
    }
}
