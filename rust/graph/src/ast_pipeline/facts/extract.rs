//! The single-pass fact walk (KGRAPH-074). One parse and one walk per file;
//! everything language-specific comes from the file's `LanguageSpec`.

use ast_grep_core::{Doc, Node};
use ast_grep_language::LanguageExt;

use super::languages;
use super::spec::{
    BaseSpec, ContainerKind, Entity, Grammar, LanguageSpec, Marker, NameSpec, Placement, TestPath,
    Visibility,
};
use super::{
    Base, BaseOwner, CallSite, Callee, Container, ContainerFactKind, Definition, FailedFile,
    FileFacts, ModuleDeclaration, Receiver, Syntax, TypeRefSite, VariableFact,
};
use crate::ast_pipeline::metadata;

/// Hex SHA-256 of `bytes`: a file's contentHash (KGRAPH-076).
pub fn content_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// Read, parse and walk one file. A file that is not UTF-8, or whose walk
/// panics, is a failed file with the error.
pub fn extract_path(path: &std::path::Path, rel: &str) -> Result<FileFacts, FailedFile> {
    extract_path_cached(path, rel, None).map(|(facts, _)| std::sync::Arc::unwrap_or_clone(facts))
}

/// Read one file once and hash its bytes. With `cached` facts for the same
/// path and contentHash the file is not parsed (`Ok((facts, false))`);
/// otherwise the same bytes are parsed and walked (`Ok((facts, true))`). A
/// file that is not UTF-8, or whose walk panics, is a failed file with the
/// error.
pub fn extract_path_cached(
    path: &std::path::Path,
    rel: &str,
    cached: Option<&std::sync::Arc<FileFacts>>,
) -> Result<(std::sync::Arc<FileFacts>, bool), FailedFile> {
    let failed = |error: String| FailedFile {
        path: rel.to_string(),
        error,
    };
    let spec = languages::for_path(rel).ok_or_else(|| failed("no declared language".into()))?;
    let bytes = std::fs::read(path).map_err(|e| failed(format!("read failed: {e}")))?;
    let hash = content_hash(&bytes);
    if let Some(facts) = cached.filter(|facts| facts.content_hash == hash) {
        return Ok((facts.clone(), false));
    }
    let source = String::from_utf8(bytes).map_err(|e| failed(format!("not UTF-8: {e}")))?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        extract_hashed_source(spec, rel, &source, hash)
    }))
    .map(|facts| (std::sync::Arc::new(facts), true))
    .map_err(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .unwrap_or_else(|| "non-string panic payload".to_string());
        failed(format!("extraction panicked: {message}"))
    })
}

/// The facts of `source`, a file at `rel` written in `spec`'s language.
pub fn extract_source(spec: &'static LanguageSpec, rel: &str, source: &str) -> FileFacts {
    extract_hashed_source(spec, rel, source, content_hash(source.as_bytes()))
}

/// The facts of `source`, whose bytes hash to `content_hash`.
fn extract_hashed_source(
    spec: &'static LanguageSpec,
    rel: &str,
    source: &str,
    content_hash: String,
) -> FileFacts {
    let mut walk = Walk {
        spec,
        source,
        facts: FileFacts {
            path: rel.to_string(),
            content_hash,
            language: spec.name,
            line_count: source.lines().count() as u32,
            is_test: is_test_path(spec, rel),
            containers: Vec::new(),
            definitions: Vec::new(),
            module_declarations: Vec::new(),
            calls: Vec::new(),
            imports: Vec::new(),
            declarations: Vec::new(),
            bases: Vec::new(),
            type_refs: Vec::new(),
            variables: Vec::new(),
        },
        containers: Vec::new(),
        functions: Vec::new(),
    };
    match spec.grammar {
        Grammar::Builtin(lang) => walk.node(&lang.ast_grep(source).root()),
        Grammar::Dart => walk.node(&codelet_tools::dart_lang::DartLang.ast_grep(source).root()),
    };
    walk.facts
}

fn is_test_path(spec: &LanguageSpec, rel: &str) -> bool {
    let mut parts: Vec<&str> = rel.split('/').collect();
    let file = parts.pop().unwrap_or("");
    let stem = file.split_once('.').map_or(file, |(stem, _)| stem);
    let without_ext = file.rsplit_once('.').map_or(file, |(head, _)| head);
    spec.test_paths.iter().any(|convention| match convention {
        TestPath::Directory(dir) => parts.iter().any(|part| part == dir),
        TestPath::FilePrefix(prefix) => file.starts_with(prefix),
        TestPath::StemSuffix(suffix) => without_ext.ends_with(suffix) && stem == without_ext,
        TestPath::NamePart(part) => {
            let pieces: Vec<&str> = file.split('.').collect();
            pieces.len() > 2 && pieces[1..pieces.len() - 1].contains(part)
        }
    })
}

/// The open function of the walk: its definition and how many decision
/// nodes enclose the current node inside it.
struct OpenFunction {
    definition: usize,
    decision_depth: usize,
}

struct Walk<'s> {
    spec: &'static LanguageSpec,
    source: &'s str,
    facts: FileFacts,
    /// Open containers, innermost last.
    containers: Vec<usize>,
    functions: Vec<OpenFunction>,
}

impl Walk<'_> {
    fn container(&self) -> Option<usize> {
        self.containers.last().copied()
    }

    /// Walk `node`. Returns the definition whose body is the next sibling,
    /// with its function container and the body's kind.
    fn node<D: Doc>(&mut self, node: &Node<'_, D>) -> Option<(usize, Option<usize>, &'static str)> {
        let kind = node.kind();
        let kind = kind.as_ref();
        let spec = self.spec;

        if spec.imports.contains(&kind) {
            let snapshot = self.snapshot(node, None);
            self.facts.imports.push(snapshot);
            return None;
        }
        if spec.declarations.contains(&kind) {
            let snapshot = self.snapshot(node, None);
            self.facts.declarations.push(snapshot);
        }
        if let Some(macros) = &spec.macros {
            if kind == macros.invocation {
                self.macro_calls(node);
                return None;
            }
        }

        let mut pushed_container = false;
        let mut pushed_function = false;
        let mut pushed_owner = false;
        let mut defined: Option<usize> = None;
        let mut decision = false;

        if let Some(owner) = spec.method_owners.iter().find(|o| o.kind == kind) {
            let self_type = node
                .field(owner.field)
                .and_then(|f| {
                    f.dfs()
                        .find(|n| owner.type_kinds.contains(&n.kind().as_ref()))
                })
                .map(|t| path_of(spec, &t))
                .filter(|p| !p.is_empty());
            if let Some(self_type) = self_type {
                let parent = self.container();
                let index = self.facts.containers.len();
                self.facts.containers.push(Container {
                    kind: ContainerFactKind::Impl,
                    name: self_type.last().cloned().unwrap_or_default(),
                    parent,
                    definition: None,
                    self_type: Some(self_type),
                    trait_path: None,
                    start_byte: node.range().start,
                    is_test: parent.is_some_and(|p| self.facts.containers[p].is_test),
                });
                self.containers.push(index);
                pushed_owner = true;
            }
        }

        if let Some(def_spec) = spec
            .definitions
            .iter()
            .find(|d| d.kind == kind && d.requires_field.is_none_or(|f| node.field(f).is_some()))
        {
            let name = name_of(spec, node, def_spec.name);
            if !name.is_empty() {
                let index = self.definition(node, def_spec, name);
                defined = Some(index);
                if let Some(container) = def_spec.container {
                    self.push_container(
                        node,
                        container,
                        Some(index),
                        self.facts.definitions[index].name.clone(),
                    );
                    pushed_container = true;
                }
                if def_spec.entity == Entity::Function {
                    self.functions.push(OpenFunction {
                        definition: index,
                        decision_depth: 0,
                    });
                    pushed_function = true;
                    if !pushed_container {
                        let name = self.facts.definitions[index].name.clone();
                        self.push_container(node, ContainerKind::Function, Some(index), name);
                        pushed_container = true;
                    }
                }
                self.bases_of(node, BaseOwner::Definition(index));
            }
        } else if let Some(container_spec) = spec
            .containers
            .iter()
            .find(|c| c.kind == kind && c.requires_field.is_none_or(|f| node.field(f).is_some()))
        {
            let name = match (container_spec.name, container_spec.container) {
                (Some(name), _) => name_of(spec, node, name),
                (None, ContainerKind::Impl { self_type, .. }) => node
                    .field(self_type)
                    .map(|t| path_of(spec, &t))
                    .and_then(|p| p.last().cloned())
                    .unwrap_or_default(),
                (None, _) => String::new(),
            };
            let index = self.push_container(node, container_spec.container, None, name);
            pushed_container = true;
            self.bases_of(node, BaseOwner::Container(index));
        } else if let Some(decl) = spec
            .module_declarations
            .iter()
            .find(|m| m.kind == kind && node.field(m.without_field).is_none())
        {
            let name = name_of(spec, node, decl.name);
            let path_attribute = decl.path_attribute.and_then(|(attribute, string_kind)| {
                self.decorator_nodes(node)
                    .into_iter()
                    .find_map(|d| path_attribute_value(spec, &d, attribute, string_kind))
            });
            self.facts.module_declarations.push(ModuleDeclaration {
                name,
                container: self.container(),
                path_attribute,
            });
        }

        if let Some(function) = self.functions.last() {
            let caller = function.definition;
            let conditional = function.decision_depth > 0;
            if let Some(call) = spec.calls.iter().find(|c| c.kind == kind) {
                let callee = match (call.callee, call.receiver) {
                    (Some(name_field), Some(receiver_field)) => {
                        node.field(name_field).map(|name| {
                            let name = name.text().to_string();
                            match node.field(receiver_field) {
                                Some(receiver) => Callee::Method {
                                    receiver: self.receiver(&receiver),
                                    name,
                                },
                                None => Callee::Path(vec![name]),
                            }
                        })
                    }
                    (Some(field), None) => node.field(field).map(|c| self.callee(&c)),
                    (None, _) => node
                        .children()
                        .find(|c| c.is_named())
                        .map(|c| self.callee(&c)),
                };
                if let Some(callee) = callee {
                    self.facts.calls.push(CallSite {
                        caller,
                        callee,
                        start_byte: node.range().start,
                        line: node.start_pos().line() as u32 + 1,
                        conditional,
                    });
                }
            }
            if spec.decisions.iter().any(|d| {
                d.kind == kind
                    && (d.operators.is_empty()
                        || node
                            .field("operator")
                            .is_some_and(|op| d.operators.contains(&op.text().as_ref())))
            }) {
                decision = true;
                self.facts.definitions[caller].complexity += 1;
            }
        }

        if self.functions.is_empty() {
            if let Some(var) = spec.variables.iter().find(|v| v.kind == kind) {
                let container_ok = match self.container() {
                    None => true,
                    Some(c) => matches!(
                        self.facts.containers[c].kind,
                        ContainerFactKind::Module | ContainerFactKind::Type
                    ),
                };
                let name = name_of(spec, node, var.name);
                if container_ok && !name.is_empty() {
                    let value = var
                        .value
                        .and_then(|f| node.field(f))
                        .map(|v| v.text().to_string())
                        .unwrap_or_default();
                    self.facts.variables.push(VariableFact {
                        name,
                        container: self.container(),
                        start_byte: node.range().start,
                        line_start: node.start_pos().line() as u32 + 1,
                        value,
                        is_constant: var.constant,
                    });
                }
            }
        }

        if decision {
            if let Some(f) = self.functions.last_mut() {
                f.decision_depth += 1;
            }
        }
        if !self.functions.is_empty() {
            if let Some(sibling_calls) = &spec.sibling_calls {
                self.sibling_calls(node, sibling_calls);
            }
        }
        let mut pending: Option<(usize, Option<usize>, &'static str)> = None;
        for child in node.children().filter(|c| c.is_named()) {
            if let Some((definition, container, body_kind)) = pending.take() {
                if child.kind() == body_kind {
                    self.body(&child, definition, container);
                    continue;
                }
            }
            pending = self.node(&child);
        }
        if decision {
            if let Some(f) = self.functions.last_mut() {
                f.decision_depth -= 1;
            }
        }
        if pushed_function {
            self.functions.pop();
        }
        if pushed_container {
            self.containers.pop();
        }
        if pushed_owner {
            self.containers.pop();
        }
        // A definition wrapped in a node of its own (Dart `method_signature`)
        // has its body beside the wrapper: pass the last child's body owner up.
        let Some(definition) = defined else {
            return pending;
        };
        let (_, body_kind) = spec.body_siblings.iter().find(|(k, _)| *k == kind)?;
        let container = self
            .facts
            .containers
            .iter()
            .rposition(|c| c.definition == Some(definition));
        Some((definition, container, body_kind))
    }

    /// Walk a definition's sibling body inside that definition, and extend
    /// the definition's span, lines and source over it.
    fn body<D: Doc>(&mut self, body: &Node<'_, D>, definition: usize, container: Option<usize>) {
        self.functions.push(OpenFunction {
            definition,
            decision_depth: 0,
        });
        if let Some(c) = container {
            self.containers.push(c);
        }
        self.node(body);
        if container.is_some() {
            self.containers.pop();
        }
        self.functions.pop();
        let def = &mut self.facts.definitions[definition];
        def.end_byte = body.range().end;
        def.line_end = body.end_pos().line() as u32 + 1;
        let (source, truncated) =
            metadata::extract_source(&self.source[def.start_byte..def.end_byte]);
        def.source = source;
        def.truncated = truncated;
    }

    /// Calls written as a name followed by selectors (Dart `f(x)`,
    /// `A.b()`, `this.c()`): one call fact per selector holding arguments.
    fn sibling_calls<D: Doc>(&mut self, node: &Node<'_, D>, calls: &super::spec::SiblingCallSpec) {
        let Some(function) = self.functions.last() else {
            return;
        };
        let caller = function.definition;
        let conditional = function.decision_depth > 0;
        let spec = self.spec;
        let children: Vec<Node<'_, D>> = node.children().filter(|c| c.is_named()).collect();
        let mut i = 0;
        while i < children.len() {
            let head = &children[i];
            let head_kind = head.kind();
            let is_self = spec.self_receivers.contains(&head_kind.as_ref());
            if !is_self && !spec.paths.names.contains(&head_kind.as_ref()) {
                i += 1;
                continue;
            }
            let mut path: Vec<String> = if is_self {
                Vec::new()
            } else {
                vec![head.text().to_string()]
            };
            let mut exact = true;
            let mut j = i + 1;
            while j < children.len() && children[j].kind() == calls.selector {
                let selector = &children[j];
                if let Some(member) = selector
                    .children()
                    .find(|c| calls.member_selectors.contains(&c.kind().as_ref()))
                {
                    match member
                        .children()
                        .find(|c| spec.paths.names.contains(&c.kind().as_ref()))
                    {
                        Some(name) => path.push(name.text().to_string()),
                        None => exact = false,
                    }
                } else if selector.children().any(|c| c.kind() == calls.call_part) {
                    let callee = match (exact, path.split_last()) {
                        (true, Some((name, []))) if !is_self => Callee::Path(vec![name.clone()]),
                        (true, Some((name, []))) => Callee::Method {
                            receiver: Receiver::SelfValue,
                            name: name.clone(),
                        },
                        (true, Some((name, receiver))) if !is_self => Callee::Method {
                            receiver: Receiver::Path(receiver.to_vec()),
                            name: name.clone(),
                        },
                        (_, Some((name, _))) => Callee::Method {
                            receiver: Receiver::Expression,
                            name: name.clone(),
                        },
                        (_, None) => Callee::Expression(selector.kind().to_string()),
                    };
                    self.facts.calls.push(CallSite {
                        caller,
                        callee,
                        start_byte: head.range().start,
                        line: head.start_pos().line() as u32 + 1,
                        conditional,
                    });
                    // What follows is a member of the call's result.
                    exact = false;
                } else {
                    exact = false;
                }
                j += 1;
            }
            i = j.max(i + 1);
        }
    }

    fn push_container<D: Doc>(
        &mut self,
        node: &Node<'_, D>,
        kind: ContainerKind,
        definition: Option<usize>,
        name: String,
    ) -> usize {
        let spec = self.spec;
        let (fact_kind, self_type, trait_path) = match kind {
            ContainerKind::Module => (ContainerFactKind::Module, None, None),
            ContainerKind::Type => (ContainerFactKind::Type, None, None),
            ContainerKind::Function => (ContainerFactKind::Function, None, None),
            ContainerKind::Impl {
                self_type,
                trait_field,
            } => (
                ContainerFactKind::Impl,
                node.field(self_type).map(|t| path_of(spec, &t)),
                trait_field
                    .and_then(|f| node.field(f))
                    .map(|t| path_of(spec, &t)),
            ),
        };
        let parent = self.container();
        let parent_test = parent.is_some_and(|p| self.facts.containers[p].is_test);
        let is_test = parent_test
            || self
                .decorator_nodes(node)
                .iter()
                .any(|d| self.is_test_marker(d, true));
        let index = self.facts.containers.len();
        self.facts.containers.push(Container {
            kind: fact_kind,
            name,
            parent,
            definition,
            self_type,
            trait_path,
            start_byte: node.range().start,
            is_test,
        });
        self.containers.push(index);
        index
    }

    fn definition<D: Doc>(
        &mut self,
        node: &Node<'_, D>,
        def_spec: &super::spec::DefinitionSpec,
        name: String,
    ) -> usize {
        let spec = self.spec;
        let range = node.range();
        let (source, truncated) = metadata::extract_source(&self.source[range.clone()]);
        let parameters = def_spec.parameters.and_then(|f| node.field(f));
        let in_type = self
            .container()
            .is_some_and(|c| self.facts.containers[c].kind == ContainerFactKind::Type);
        let parameter_names: Vec<String> = parameters
            .as_ref()
            .map(|p| parameter_names(spec, def_spec, p, in_type))
            .unwrap_or_default();
        let param_count = parameter_names.len() as u32;
        let decorator_nodes = self.decorator_nodes(node);
        let decorators: Vec<String> = decorator_nodes
            .iter()
            .map(|d| d.text().trim().to_string())
            .collect();
        let container = self.container();
        let in_test_container = container.is_some_and(|c| self.facts.containers[c].is_test);
        let is_test = self.facts.is_test
            || in_test_container
            || decorator_nodes
                .iter()
                .any(|d| self.is_test_marker(d, false));
        let is_async = markers(node, spec.async_markers);
        let is_public = self.is_public(node, &name);
        let members = def_spec
            .members
            .iter()
            .flat_map(|m| {
                node.dfs()
                    .filter(|n| n.kind() == m.kind)
                    .map(|n| name_of(spec, &n, m.name))
                    .collect::<Vec<_>>()
            })
            .filter(|n| !n.is_empty())
            .collect();
        let index = self.facts.definitions.len();
        self.facts.definitions.push(Definition {
            entity: def_spec.entity,
            node_kind: def_spec.kind,
            name,
            container,
            start_byte: range.start,
            end_byte: range.end,
            line_start: node.start_pos().line() as u32 + 1,
            line_end: node.end_pos().line() as u32 + 1,
            is_async,
            is_public,
            is_test,
            param_count,
            parameters: parameter_names.join(", "),
            source,
            truncated,
            docstring: self.docstring(node),
            decorators,
            complexity: if def_spec.entity == Entity::Function {
                1
            } else {
                0
            },
            members,
        });
        if def_spec.entity == Entity::Function {
            for field in spec.type_ref_fields {
                // A field, or for field-less grammars the named children of
                // that kind.
                let targets: Vec<Node<'_, D>> = match node.field(field.field) {
                    Some(target) => vec![target],
                    None => node
                        .children()
                        .filter(|c| c.kind() == field.field)
                        .collect(),
                };
                for target in targets {
                    let mut names = Vec::new();
                    match field.item_field {
                        None => type_names(spec, &target, field.type_kinds, &mut names),
                        Some(item_field) => {
                            for item in target.children().filter(|c| c.is_named()) {
                                if let Some(typed) = item.field(item_field) {
                                    type_names(spec, &typed, field.type_kinds, &mut names);
                                }
                            }
                        }
                    }
                    for name in names {
                        self.facts.type_refs.push(TypeRefSite {
                            function: index,
                            ref_kind: field.ref_kind,
                            target: name,
                        });
                    }
                }
            }
        }
        index
    }

    fn bases_of<D: Doc>(&mut self, node: &Node<'_, D>, owner: BaseOwner) {
        let spec = self.spec;
        let kind = node.kind();
        for base in spec.bases.iter().filter(|b| b.kind == kind) {
            for target in base_targets(spec, node, base) {
                if !target.is_empty() {
                    self.facts.bases.push(Base {
                        owner: owner.clone(),
                        relation: base.relation,
                        target,
                    });
                }
            }
        }
    }

    fn callee<D: Doc>(&self, node: &Node<'_, D>) -> Callee {
        let spec = self.spec;
        let kind = node.kind();
        if let Some((_, field)) = spec.paths.wrappers.iter().find(|(k, _)| *k == kind) {
            if let Some(inner) = node.field(field) {
                return self.callee(&inner);
            }
        }
        if let Some(member) = spec.paths.members.iter().find(|m| m.kind == kind) {
            let name = node.field(member.name).map(|n| n.text().to_string());
            let receiver = node.field(member.path).map(|r| self.receiver(&r));
            if let (Some(name), Some(receiver)) = (name, receiver) {
                return Callee::Method { receiver, name };
            }
        }
        let path = path_of(spec, node);
        if path.is_empty() {
            Callee::Expression(node.kind().to_string())
        } else {
            Callee::Path(path)
        }
    }

    fn receiver<D: Doc>(&self, node: &Node<'_, D>) -> Receiver {
        if self.spec.self_receivers.contains(&node.kind().as_ref())
            || self
                .spec
                .self_receiver_texts
                .contains(&node.text().as_ref())
        {
            return Receiver::SelfValue;
        }
        let path = path_of(self.spec, node);
        if path.is_empty() {
            Receiver::Expression
        } else {
            Receiver::Path(path)
        }
    }

    /// Calls written inside a macro's token trees: a declared name token
    /// followed by a token tree that opens with `(`.
    fn macro_calls<D: Doc>(&mut self, node: &Node<'_, D>) {
        let Some(function) = self.functions.last() else {
            return;
        };
        let caller = function.definition;
        let conditional = function.decision_depth > 0;
        let Some(macros) = &self.spec.macros else {
            return;
        };
        for tree in node.dfs().filter(|n| n.kind() == macros.token_tree) {
            let children: Vec<_> = tree.children().collect();
            for pair in children.windows(2) {
                let (name, next) = (&pair[0], &pair[1]);
                if macros.names.contains(&name.kind().as_ref())
                    && next.kind() == macros.token_tree
                    && next.child(0).is_some_and(|open| open.text() == "(")
                {
                    self.facts.calls.push(CallSite {
                        caller,
                        callee: Callee::InMacro(name.text().to_string()),
                        start_byte: name.range().start,
                        line: name.start_pos().line() as u32 + 1,
                        conditional,
                    });
                }
            }
        }
    }

    fn snapshot<D: Doc>(&self, node: &Node<'_, D>, field: Option<&'static str>) -> Syntax {
        let body = node.field("body").map(|b| b.range());
        let children = node
            .children()
            .filter(|c| c.is_named() && Some(c.range()) != body)
            .map(|c| self.snapshot(&c, None))
            .collect::<Vec<_>>();
        Syntax {
            kind: node.kind().to_string(),
            field,
            text: if children.is_empty() {
                node.text().to_string()
            } else {
                String::new()
            },
            children,
            container: self.container(),
            start_byte: node.range().start,
        }
    }

    /// The decorator nodes of a definition or container.
    fn decorator_nodes<'r, D: Doc>(&self, node: &Node<'r, D>) -> Vec<Node<'r, D>> {
        let mut found = Vec::new();
        for decorator in self.spec.decorators {
            placed(
                self.spec,
                node,
                decorator.placement,
                decorator.kind,
                &mut found,
            );
        }
        found.sort_by_key(|n| n.range().start);
        found
    }

    fn is_test_marker<D: Doc>(&self, decorator: &Node<'_, D>, on_container: bool) -> bool {
        let spec = self.spec;
        let Some((path, arguments)) = decorator_path(spec, decorator) else {
            return false;
        };
        spec.test_markers.iter().any(|marker| {
            marker.path == path
                && marker.on_container == on_container
                && marker
                    .arguments
                    .is_none_or(|a| arguments.as_deref() == Some(a))
        })
    }

    fn docstring<D: Doc>(&self, node: &Node<'_, D>) -> String {
        let mut docs = Vec::new();
        for doc in self.spec.docs {
            let mut found = Vec::new();
            placed(self.spec, node, doc.placement, doc.kind, &mut found);
            for n in found {
                let text = n.text();
                let wanted = doc
                    .requires_child
                    .is_none_or(|k| n.children().any(|c| c.kind() == k))
                    && (doc.prefixes.is_empty()
                        || doc.prefixes.iter().any(|p| text.starts_with(p)));
                if wanted {
                    docs.push((n.range().start, text.trim().to_string()));
                }
            }
        }
        docs.sort();
        docs.into_iter()
            .map(|(_, t)| t)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn is_public<D: Doc>(&self, node: &Node<'_, D>, name: &str) -> bool {
        match &self.spec.visibility {
            Visibility::ChildKind(kinds) => {
                node.children().any(|c| kinds.contains(&c.kind().as_ref()))
            }
            Visibility::Modifier { modifiers, tokens } => node.children().any(|c| {
                tokens.contains(&c.kind().as_ref())
                    || (c.kind() == *modifiers
                        && c.dfs().any(|m| tokens.contains(&m.kind().as_ref())))
            }),
            Visibility::ParentKind(kinds) => node
                .parent()
                .is_some_and(|p| kinds.contains(&p.kind().as_ref())),
            Visibility::PrivatePrefix(prefix) => !name.starts_with(prefix),
            Visibility::UpperInitial => name.chars().next().is_some_and(char::is_uppercase),
            Visibility::Unless { modifiers, tokens } => !node.children().any(|c| {
                c.kind() == *modifiers && c.dfs().any(|m| tokens.contains(&m.kind().as_ref()))
            }),
            Visibility::Always => true,
        }
    }
}

/// Nodes of `kind` placed relative to `node` as declared.
fn placed<'r, D: Doc>(
    spec: &LanguageSpec,
    node: &Node<'r, D>,
    placement: Placement,
    kind: &str,
    out: &mut Vec<Node<'r, D>>,
) {
    match placement {
        Placement::PrecedingSiblings => {
            let mut current = node.prev();
            while let Some(sibling) = current {
                if !sibling.is_named() {
                    current = sibling.prev();
                    continue;
                }
                if sibling.kind() == kind {
                    out.push(sibling.clone());
                    current = sibling.prev();
                } else if is_doc_or_decorator(spec, &sibling) {
                    // Doc comments and decorators interleave; each
                    // declaration keeps only its own kind.
                    current = sibling.prev();
                } else {
                    break;
                }
            }
        }
        Placement::Children => out.extend(node.children().filter(|c| c.kind() == kind)),
        Placement::ParentChildren(parent_kind) => {
            if let Some(parent) = node.parent().filter(|p| p.kind() == parent_kind) {
                out.extend(parent.children().filter(|c| c.kind() == kind));
            }
        }
        Placement::ChildChildren(child_kind) => {
            for child in node.children().filter(|c| c.kind() == child_kind) {
                out.extend(child.children().filter(|c| c.kind() == kind));
            }
        }
        Placement::BodyFirst { body, statement } => {
            if let Some(first) = node
                .field(body)
                .and_then(|b| b.children().find(|c| c.is_named()))
                .filter(|f| f.kind() == statement)
            {
                if let Some(string) = first.children().find(|c| c.kind() == kind) {
                    out.push(string);
                }
            }
        }
    }
}

/// The names of the parameters in a parameter list, without types and
/// receivers, as the language declares them.
fn parameter_names<D: Doc>(
    spec: &LanguageSpec,
    def_spec: &super::spec::DefinitionSpec,
    list: &Node<'_, D>,
    in_type: bool,
) -> Vec<String> {
    let items: Vec<Node<'_, D>> = list
        .children()
        .filter(|c| c.is_named() && !def_spec.non_parameters.contains(&c.kind().as_ref()))
        .collect();
    let Some(rule) = &spec.parameter_names else {
        return items.iter().map(|p| p.text().to_string()).collect();
    };
    let named = |n: &Node<'_, D>| -> Option<String> {
        if rule.kinds.contains(&n.kind().as_ref()) {
            return Some(n.text().to_string());
        }
        n.dfs()
            .find(|d| rule.kinds.contains(&d.kind().as_ref()))
            .map(|d| d.text().to_string())
    };
    let mut names = Vec::new();
    let mut first = true;
    for item in items {
        if rule.receiver_kinds.contains(&item.kind().as_ref()) {
            first = false;
            continue;
        }
        if first && in_type && rule.first_is_receiver_in_type {
            first = false;
            continue;
        }
        first = false;
        let by_field: Vec<Node<'_, D>> = rule
            .fields
            .iter()
            .flat_map(|f| item.field_children(f).collect::<Vec<_>>())
            .collect();
        if by_field.is_empty() {
            names.extend(named(&item));
        } else {
            names.extend(by_field.iter().filter_map(named));
        }
    }
    names
}

fn is_doc_or_decorator<D: Doc>(spec: &LanguageSpec, node: &Node<'_, D>) -> bool {
    let kind = node.kind();
    spec.docs.iter().any(|d| d.kind == kind) || spec.decorators.iter().any(|d| d.kind == kind)
}

fn markers<D: Doc>(node: &Node<'_, D>, markers: &[Marker]) -> bool {
    markers.iter().any(|m| match m.inside {
        None => node.children().any(|c| c.kind() == m.token),
        Some(inside) => node
            .children()
            .filter(|c| c.kind() == inside)
            .any(|c| c.dfs().any(|t| t.kind() == m.token)),
    })
}

/// The name of a node as declared.
pub(crate) fn name_of<D: Doc>(spec: &LanguageSpec, node: &Node<'_, D>, name: NameSpec) -> String {
    match name {
        NameSpec::Field(field) => node
            .field(field)
            .map(|n| n.text().to_string())
            .unwrap_or_default(),
        NameSpec::Descend {
            field,
            until,
            through,
        } => {
            let mut passed = through.is_empty();
            let mut current = node.field(field);
            while let Some(n) = current {
                if through.contains(&n.kind().as_ref()) {
                    passed = true;
                }
                if until.contains(&n.kind().as_ref()) {
                    return if passed {
                        n.text().to_string()
                    } else {
                        String::new()
                    };
                }
                current = n.field(field);
            }
            String::new()
        }
        NameSpec::ChildKind(kinds) => node
            .children()
            .find(|c| kinds.contains(&c.kind().as_ref()))
            .map(|n| n.text().to_string())
            .unwrap_or_default(),
        NameSpec::LastField(field) => node
            .field_children(field)
            .last()
            .map(|n| n.text().to_string())
            .unwrap_or_default(),
        NameSpec::PathField(field) => node
            .field(field)
            .map(|n| path_of(spec, &n))
            .and_then(|p| p.last().cloned())
            .unwrap_or_default(),
    }
}

/// The path of names an expression is, or empty when it is not a path.
pub(crate) fn path_of<D: Doc>(spec: &LanguageSpec, node: &Node<'_, D>) -> Vec<String> {
    let kind = node.kind();
    let kind = kind.as_ref();
    if spec.paths.names.contains(&kind) || spec.self_receivers.contains(&kind) {
        return vec![node.text().to_string()];
    }
    if let Some((_, field)) = spec.paths.wrappers.iter().find(|(k, _)| *k == kind) {
        return node
            .field(field)
            .map(|n| path_of(spec, &n))
            .unwrap_or_default();
    }
    if let Some(scoped) = spec
        .paths
        .scoped
        .iter()
        .chain(spec.paths.members.iter())
        .find(|s| s.kind == kind)
    {
        let Some(name) = node.field(scoped.name) else {
            return Vec::new();
        };
        let mut path = match node.field(scoped.path) {
            Some(prefix) => {
                let prefix = path_of(spec, &prefix);
                if prefix.is_empty() {
                    return Vec::new();
                }
                prefix
            }
            None => Vec::new(),
        };
        let name_path = path_of(spec, &name);
        if name_path.is_empty() {
            path.push(name.text().to_string());
        } else {
            path.extend(name_path);
        }
        return path;
    }
    Vec::new()
}

/// Every type name inside a signature field, outermost first: a declared
/// type kind is one name (its path); other nodes are searched inside.
fn type_names<D: Doc>(
    spec: &LanguageSpec,
    node: &Node<'_, D>,
    kinds: &[&str],
    out: &mut Vec<Vec<String>>,
) {
    if kinds.contains(&node.kind().as_ref()) {
        let path = path_of(spec, node);
        if !path.is_empty() {
            out.push(path);
        }
        return;
    }
    for child in node.children().filter(|c| c.is_named()) {
        type_names(spec, &child, kinds, out);
    }
}

fn base_targets<D: Doc>(
    spec: &LanguageSpec,
    node: &Node<'_, D>,
    base: &BaseSpec,
) -> Vec<Vec<String>> {
    let mut reached = vec![node.clone()];
    for field in base.fields {
        let mut next = Vec::new();
        for n in &reached {
            if let Some(f) = n.field(field) {
                next.push(f);
            } else {
                // A field-less step: the named children of that kind.
                next.extend(n.children().filter(|c| c.kind() == *field));
            }
        }
        reached = next;
    }
    let mut targets = Vec::new();
    for n in reached {
        let items: Vec<_> = if base.each_child {
            n.children().filter(|c| c.is_named()).collect()
        } else {
            vec![n]
        };
        for item in items {
            if base.through_kinds.is_empty() || base.through_kinds.contains(&item.kind().as_ref()) {
                let path = path_of(spec, &item);
                if !path.is_empty() {
                    targets.push(path);
                }
            }
        }
    }
    targets
}

/// The path and argument text of a decorator: its first descendant that is
/// a declared path, and that node's next named sibling.
fn decorator_path<D: Doc>(
    spec: &LanguageSpec,
    decorator: &Node<'_, D>,
) -> Option<(String, Option<String>)> {
    let path_node = decorator.dfs().skip(1).find(|n| {
        let kind = n.kind();
        let kind = kind.as_ref();
        spec.paths.names.contains(&kind)
            || spec.paths.scoped.iter().any(|s| s.kind == kind)
            || spec.paths.members.iter().any(|s| s.kind == kind)
    })?;
    let path = path_of(spec, &path_node);
    if path.is_empty() {
        return None;
    }
    let mut next = path_node.next();
    while let Some(n) = &next {
        if n.is_named() {
            break;
        }
        next = n.next();
    }
    Some((
        path.join(spec.paths.separator),
        next.map(|n| n.text().to_string()),
    ))
}

/// `#[path = "x.rs"]`: the string value of an attribute whose path is
/// `attribute`.
fn path_attribute_value<D: Doc>(
    spec: &LanguageSpec,
    decorator: &Node<'_, D>,
    attribute: &str,
    string_kind: &str,
) -> Option<String> {
    let (path, _) = decorator_path(spec, decorator)?;
    if path != attribute {
        return None;
    }
    let literal = decorator.dfs().find(|n| n.kind() == string_kind)?;
    Some(literal.text().to_string())
}
