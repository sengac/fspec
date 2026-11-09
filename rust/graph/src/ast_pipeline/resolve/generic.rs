//! Scope resolution for every language but Rust (KGRAPH-075). A name
//! resolves to the definitions in scope: the enclosing functions (and, when
//! the language declares it, the enclosing types), the file's top level,
//! the file's import bindings, then the language's package peers. Never by
//! name across the whole tree.

use std::collections::{BTreeMap, BTreeSet};

use super::imports::{self, ImportBinding, ImportTarget};
use super::{
    enclosing, non_path_call, single, Answer, DefRef, ImportEdge, LanguageResolver, Reason,
    TreeIndex,
};
use crate::ast_pipeline::facts::{
    languages, BaseOwner, BaseRelation, CallSite, Callee, ContainerFactKind, Entity, Receiver,
};

pub(crate) struct GenericResolver<'i, 't> {
    index: &'i TreeIndex<'t>,
    file: usize,
    class_members_in_scope: bool,
    bindings: Vec<ImportBinding>,
    peers: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Want {
    Function,
    Type,
}

impl<'i, 't> GenericResolver<'i, 't> {
    pub fn new(
        index: &'i TreeIndex<'t>,
        file: usize,
        manifests: &BTreeMap<String, String>,
    ) -> Self {
        let language = index.file(file).language;
        GenericResolver {
            index,
            file,
            class_members_in_scope: languages::by_name(language)
                .is_some_and(|s| s.class_members_in_scope),
            bindings: imports::bindings(index, file, manifests),
            peers: imports::package_peers(index, file),
        }
    }

    fn wanted(&self, def: DefRef, want: Want) -> bool {
        match (self.index.def(def).entity, want) {
            (Entity::Function, Want::Function) => true,
            (Entity::Type(_), Want::Type) => true,
            // A class name called as a function is its constructor.
            (Entity::Type(_), Want::Function) => true,
            _ => false,
        }
    }

    fn top_level(&self, file: usize, name: &str, want: Want) -> BTreeSet<DefRef> {
        self.named(file, None, name, want)
    }

    fn named(
        &self,
        file: usize,
        container: Option<usize>,
        name: &str,
        want: Want,
    ) -> BTreeSet<DefRef> {
        self.index
            .named_in(file, container, name)
            .iter()
            .map(|i| (file, *i))
            .filter(|def| self.wanted(*def, want))
            .collect()
    }

    /// A plain name seen from `container` of this file.
    fn plain(&self, container: Option<usize>, name: &str, want: Want) -> Answer {
        let facts = self.index.file(self.file);
        let mut current = container;
        while let Some(c) = current {
            let kind = facts.containers[c].kind;
            let searched = matches!(
                kind,
                ContainerFactKind::Function | ContainerFactKind::Module
            ) || (self.class_members_in_scope && kind == ContainerFactKind::Type);
            if searched {
                let found = self.named(self.file, Some(c), name, want);
                if !found.is_empty() {
                    return single(found);
                }
            }
            current = facts.containers[c].parent;
        }
        let found = self.top_level(self.file, name, want);
        if !found.is_empty() {
            return single(found);
        }
        let mut found = BTreeSet::new();
        let mut first_error = None;
        for binding in &self.bindings {
            match (&binding.local, &binding.target) {
                (Some(local), Ok(ImportTarget::Definition(def)))
                    if local == name && self.wanted(*def, want) =>
                {
                    found.insert(*def);
                }
                (Some(local), Err(reason)) if local == name => {
                    first_error.get_or_insert(*reason);
                }
                (None, Ok(ImportTarget::File(file))) => {
                    found.extend(self.top_level(*file, name, want))
                }
                _ => {}
            }
        }
        if !found.is_empty() {
            return single(found);
        }
        if let Some(reason) = first_error {
            return Err(reason);
        }
        let found: BTreeSet<DefRef> = self
            .peers
            .iter()
            .flat_map(|peer| self.top_level(*peer, name, want))
            .collect();
        single(found)
    }

    /// The files a name binds as a module (a Go package is several files).
    fn module_files(&self, name: &str) -> Vec<usize> {
        self.bindings
            .iter()
            .filter_map(|b| match (&b.local, &b.target) {
                (Some(local), Ok(ImportTarget::File(file))) if local == name => Some(*file),
                _ => None,
            })
            .collect()
    }

    /// The top-level definitions named `name` in the files `module` binds.
    fn in_module(&self, module: &str, name: &str, want: Want) -> Option<Answer> {
        let files = self.module_files(module);
        if files.is_empty() {
            return None;
        }
        Some(single(
            files
                .iter()
                .flat_map(|f| self.top_level(*f, name, want))
                .collect(),
        ))
    }

    /// The method `name` of type `owner`, or of its bases.
    fn method(&self, owner: DefRef, name: &str, seen: &mut BTreeSet<DefRef>) -> Answer {
        if !seen.insert(owner) {
            return Err(Reason::NotFound);
        }
        let facts = self.index.file(owner.0);
        let Some(container) = self.index.container_of(owner) else {
            return Err(Reason::NotFound);
        };
        let own = self.named(owner.0, Some(container), name, Want::Function);
        let own: BTreeSet<DefRef> = own
            .into_iter()
            .filter(|d| self.index.def(*d).entity == Entity::Function)
            .collect();
        if !own.is_empty() {
            return single(own);
        }
        let mut inherited = BTreeSet::new();
        for base in &facts.bases {
            let base_owner = match base.owner {
                BaseOwner::Definition(d) => d,
                BaseOwner::Container(c) => match facts.containers[c].definition {
                    Some(d) => d,
                    None => continue,
                },
            };
            if base_owner != owner.1 || owner.0 != self.file {
                continue;
            }
            if let Ok(base_def) = self.path(
                facts.definitions[owner.1].container,
                &base.target,
                Want::Type,
            ) {
                if let Ok(m) = self.method(base_def, name, seen) {
                    inherited.insert(m);
                }
            }
        }
        single(inherited)
    }

    /// A path of names: a module binding, a type, then members.
    fn path(&self, container: Option<usize>, path: &[String], want: Want) -> Answer {
        match path {
            [] => Err(Reason::NotFound),
            [name] => self.plain(container, name, want),
            [head, rest @ ..] => {
                if let [name] = rest {
                    if let Some(answer) = self.in_module(head, name, want) {
                        return answer;
                    }
                }
                let mut owner = self.plain(container, head, Want::Type)?;
                for (i, segment) in rest.iter().enumerate() {
                    let last = i + 1 == rest.len();
                    owner = if last && want == Want::Function {
                        self.method(owner, segment, &mut BTreeSet::new())?
                    } else {
                        self.nested_type(owner, segment)?
                    };
                }
                Ok(owner)
            }
        }
    }

    fn nested_type(&self, owner: DefRef, name: &str) -> Answer {
        let Some(container) = self.index.container_of(owner) else {
            return Err(Reason::NotFound);
        };
        single(
            self.named(owner.0, Some(container), name, Want::Type)
                .into_iter()
                .filter(|d| matches!(self.index.def(*d).entity, Entity::Type(_)))
                .collect(),
        )
    }

    fn owner_of(&self, container: Option<usize>) -> Option<DefRef> {
        let facts = self.index.file(self.file);
        let c = enclosing(
            facts,
            container,
            &[ContainerFactKind::Type, ContainerFactKind::Impl],
        )?;
        self.method_owner(self.file, c)
    }
}

impl LanguageResolver for GenericResolver<'_, '_> {
    fn call(&self, file: usize, call: &CallSite) -> Answer {
        let container = self.index.body_scope((file, call.caller));
        match &call.callee {
            Callee::Path(path) => self.path(container, path, Want::Function),
            Callee::Method {
                receiver: Receiver::SelfValue,
                name,
            } => {
                let owner = self
                    .owner_of(container)
                    .ok_or(Reason::ReceiverTypeUnknown)?;
                self.method(owner, name, &mut BTreeSet::new())
            }
            Callee::Method {
                receiver: Receiver::Path(path),
                name,
            } => {
                if let [module] = path.as_slice() {
                    if let Some(answer) = self.in_module(module, name, Want::Function) {
                        return answer;
                    }
                }
                match self.path(container, path, Want::Type) {
                    Ok(owner) if matches!(self.index.def(owner).entity, Entity::Type(_)) => {
                        self.method(owner, name, &mut BTreeSet::new())
                    }
                    _ => Err(Reason::ReceiverTypeUnknown),
                }
            }
            other => Err(non_path_call(other).unwrap_or(Reason::NotFound)),
        }
    }

    fn type_ref(&self, file: usize, function: usize, target: &[String]) -> Answer {
        self.path(self.index.body_scope((file, function)), target, Want::Type)
    }

    fn bases(&self, file: usize, out: &mut Vec<(DefRef, BaseRelation, Answer)>) {
        let facts = self.index.file(file);
        for base in &facts.bases {
            let owner = match base.owner {
                BaseOwner::Definition(d) => Some((file, d)),
                BaseOwner::Container(c) => self.method_owner(file, c),
            };
            let Some(owner) = owner else {
                continue;
            };
            let answer = self.path(
                facts.definitions[owner.1].container,
                &base.target,
                Want::Type,
            );
            out.push((owner, base.relation, answer));
        }
    }

    fn method_owner(&self, file: usize, container: usize) -> Option<DefRef> {
        let facts = self.index.file(file);
        let fact = &facts.containers[container];
        match fact.kind {
            ContainerFactKind::Impl => {
                let path = fact.self_type.as_ref()?;
                self.path(fact.parent, path, Want::Type).ok()
            }
            _ => fact.definition.map(|d| (file, d)),
        }
    }

    fn imports(&self, file: usize, out: &mut Vec<Result<ImportEdge, Reason>>) {
        for binding in &self.bindings {
            out.push(binding.target.clone().map(|target| ImportEdge {
                from_file: file,
                to_file: match target {
                    ImportTarget::File(f) => f,
                    ImportTarget::Definition(def) => def.0,
                },
                import_path: binding.import_path.clone(),
            }));
        }
    }
}
