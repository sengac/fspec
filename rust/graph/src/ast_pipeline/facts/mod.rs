//! Per-file AST facts (KGRAPH-074).
//!
//! Each source file is read once, parsed once with its declared grammar and
//! its syntax tree is walked once. The walk records what the file declares
//! and references as written: definitions with their container chain and
//! byte span, call sites with the callee expression, imports, bases, type
//! references and variables. Facts never depend on another file; edges are
//! built from the facts of the whole tree by `resolve`.

pub mod extract;
pub mod languages;
pub mod spec;

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

pub use spec::{BaseRelation, Entity, LanguageSpec, RefKind, TypeKind};

/// The largest file that is parsed (product owner, 2026-11-07).
pub const MAX_FILE_BYTES: u64 = 1_048_576;

/// Stack of an extraction thread: the walk recurses once per syntax-tree
/// level, and generated sources nest deeply.
const WALK_STACK_BYTES: usize = 256 * 1024 * 1024;

/// The facts of one source file.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct FileFacts {
    /// Path relative to the root, with `/` separators.
    pub path: String,
    /// Hex SHA-256 of the bytes the facts were extracted from.
    pub content_hash: String,
    pub language: &'static str,
    pub line_count: u32,
    /// The file matches a declared test path convention.
    pub is_test: bool,
    pub containers: Vec<Container>,
    pub definitions: Vec<Definition>,
    pub module_declarations: Vec<ModuleDeclaration>,
    pub calls: Vec<CallSite>,
    pub imports: Vec<Syntax>,
    pub declarations: Vec<Syntax>,
    pub bases: Vec<Base>,
    pub type_refs: Vec<TypeRefSite>,
    pub variables: Vec<VariableFact>,
}

/// A scope that definitions sit in.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Container {
    pub kind: ContainerFactKind,
    /// The container's segment in a qualified name.
    pub name: String,
    pub parent: Option<usize>,
    /// The definition this container is, when it is one (class, trait, fn).
    pub definition: Option<usize>,
    /// For an implementation block: the self type and trait as written.
    pub self_type: Option<Vec<String>>,
    pub trait_path: Option<Vec<String>>,
    pub start_byte: usize,
    /// The container is marked as a test container (`#[cfg(test)] mod`).
    pub is_test: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub enum ContainerFactKind {
    Module,
    Type,
    Impl,
    Function,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Definition {
    pub entity: Entity,
    pub node_kind: &'static str,
    pub name: String,
    /// The innermost container.
    pub container: Option<usize>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub line_start: u32,
    pub line_end: u32,
    pub is_async: bool,
    pub is_public: bool,
    pub is_test: bool,
    pub param_count: u32,
    pub parameters: String,
    pub source: String,
    pub truncated: bool,
    pub docstring: String,
    pub decorators: Vec<String>,
    pub complexity: u32,
    /// Member names (enum variants) of a Type.
    pub members: Vec<String>,
}

/// A module stored in another file (`mod x;`).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ModuleDeclaration {
    pub name: String,
    pub container: Option<usize>,
    /// The `#[path = "..."]` value as written.
    pub path_attribute: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CallSite {
    /// The innermost enclosing function definition.
    pub caller: usize,
    pub callee: Callee,
    pub start_byte: usize,
    pub line: u32,
    /// Inside a declared decision node of the caller.
    pub conditional: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Callee {
    /// A path of one or more segments as written (`f`, `Type::new`,
    /// `crate::a::f`).
    Path(Vec<String>),
    /// A method on a receiver.
    Method { receiver: Receiver, name: String },
    /// A name followed by a parenthesized token tree inside a macro
    /// invocation: the grammar does not parse it as a call.
    InMacro(String),
    /// A callee expression that is not a path (a closure, an index, ...),
    /// by node kind.
    Expression(String),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Receiver {
    /// The enclosing type's instance (`self`, `this`).
    SelfValue,
    /// A path receiver (`module.f()` in languages where `.` reaches modules).
    Path(Vec<String>),
    /// Any other expression.
    Expression,
}

/// A syntax snapshot of an import or declaration node: kind, leaf text and
/// named children, in source order.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Syntax {
    pub kind: String,
    pub field: Option<&'static str>,
    pub text: String,
    pub children: Vec<Syntax>,
    pub container: Option<usize>,
    pub start_byte: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum BaseOwner {
    Definition(usize),
    Container(usize),
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Base {
    pub owner: BaseOwner,
    pub relation: BaseRelation,
    pub target: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct TypeRefSite {
    pub function: usize,
    pub ref_kind: RefKind,
    pub target: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct VariableFact {
    pub name: String,
    pub container: Option<usize>,
    pub start_byte: usize,
    pub line_start: u32,
    pub value: String,
    pub is_constant: bool,
}

/// A file the walk did not parse, and why.
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SkippedFile {
    pub path: String,
    pub reason: SkipReason,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Symlink,
    TooLarge,
}

/// A file that could not be read or parsed.
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct FailedFile {
    pub path: String,
    pub error: String,
}

/// The facts of a set of files, sorted by path.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct TreeFacts {
    #[serde(serialize_with = "serialize_files")]
    pub files: Vec<Arc<FileFacts>>,
    pub failed: Vec<FailedFile>,
    /// How many files were parsed, and how many reused cached facts.
    pub parsed: usize,
    pub reused: usize,
}

fn serialize_files<S: serde::Serializer>(
    files: &[Arc<FileFacts>],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(files.iter().map(|facts| facts.as_ref()))
}

/// Facts of earlier parses by relative path (KGRAPH-076); an entry is used
/// only for a file whose bytes have the same contentHash.
pub type FactCache = HashMap<String, Arc<FileFacts>>;

impl TreeFacts {
    /// The cache of these facts, for the next index of the same root.
    pub fn cache(&self) -> FactCache {
        self.files
            .iter()
            .map(|facts| (facts.path.clone(), facts.clone()))
            .collect()
    }
}

/// Extract the facts of `files` (absolute paths under `root`, all of a
/// declared language) on `threads` threads. The result does not depend on
/// the number of threads: files are sorted by path, and each file's facts
/// are in source order.
pub fn extract_tree(
    root: &std::path::Path,
    files: &[std::path::PathBuf],
    threads: usize,
) -> TreeFacts {
    extract_tree_cached(root, files, threads, &FactCache::new())
}

/// [`extract_tree`], reusing the `cache`d facts of every file whose path
/// and contentHash match; each file is read once and hashed from the bytes
/// that are parsed.
pub fn extract_tree_cached(
    root: &std::path::Path,
    files: &[std::path::PathBuf],
    threads: usize,
    cache: &FactCache,
) -> TreeFacts {
    let mut jobs: Vec<(String, &std::path::PathBuf)> = files
        .iter()
        .map(|path| (relative_path(root, path), path))
        .collect();
    jobs.sort_by(|a, b| a.0.cmp(&b.0));
    let threads = threads.max(1).min(jobs.len().max(1));
    let chunk = jobs.len().div_ceil(threads).max(1);
    let mut results: Vec<Result<(Arc<FileFacts>, bool), FailedFile>> =
        Vec::with_capacity(jobs.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .chunks(chunk)
            .map(|part| {
                let handle = std::thread::Builder::new()
                    .stack_size(WALK_STACK_BYTES)
                    .spawn_scoped(scope, move || {
                        part.iter()
                            .map(|(rel, path)| {
                                extract::extract_path_cached(path, rel, cache.get(rel))
                            })
                            .collect::<Vec<_>>()
                    });
                (part, handle)
            })
            .collect();
        for (part, handle) in handles {
            let joined = match handle {
                Ok(handle) => handle
                    .join()
                    .map_err(|_| "extraction thread panicked".to_string()),
                Err(e) => Err(format!("extraction thread not started: {e}")),
            };
            match joined {
                Ok(facts) => results.extend(facts),
                Err(error) => results.extend(part.iter().map(|(rel, _)| {
                    Err(FailedFile {
                        path: rel.clone(),
                        error: error.clone(),
                    })
                })),
            }
        }
    });
    let mut tree = TreeFacts::default();
    for result in results {
        match result {
            Ok((facts, parsed)) => {
                if parsed {
                    tree.parsed += 1;
                } else {
                    tree.reused += 1;
                }
                tree.files.push(facts);
            }
            Err(failed) => tree.failed.push(failed),
        }
    }
    tree
}

/// The number of threads used by an index.
pub fn index_threads() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
}

/// `path` relative to `root`, with `/` separators.
pub fn relative_path(root: &std::path::Path, path: &std::path::Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
