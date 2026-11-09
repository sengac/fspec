//! AST Extraction Pipeline (KGRAPH-074/075)
//!
//! 1. `walk` lists the source files and package manifests of a root.
//! 2. `facts` parses each source file once and walks its syntax tree once,
//!    in parallel, using the file's declared language table.
//! 3. `resolve` builds every edge from the facts of the whole tree.
//! 4. The dependency extractors add Dependency nodes from the manifests.
//!
//! The result depends only on the tree: the same files always give the
//! same entities in the same order.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::graph_entities::GraphEntity;

pub mod cargo_dep_extractor;
pub mod composer_dep_extractor;
pub mod csproj_dep_extractor;
pub mod facts;
pub mod gemfile_dep_extractor;
pub mod gomod_dep_extractor;
pub mod helpers;
pub mod java_dep_extractor;
pub mod metadata;
pub mod npm_dep_extractor;
pub mod pip_dep_extractor;
pub mod pubspec_dep_extractor;
pub mod resolve;
pub mod sbt_dep_extractor;
pub mod swift_dep_extractor;

use facts::{FactCache, FailedFile, SkipReason, SkippedFile, TreeFacts, MAX_FILE_BYTES};
use resolve::UnresolvedCounts;

/// Directories never indexed, whatever the ignore files say.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    ".git",
    ".fspec",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    "venv",
    ".venv",
    "vendor",
    "Pods",
    ".gradle",
    "build",
];

/// Package manifests the resolver reads, by file name.
const MANIFESTS: &[&str] = &["Cargo.toml", "go.mod", "pubspec.yaml"];

/// What the walk of a root found.
#[derive(Debug, Default)]
pub struct Walked {
    /// Source files of a declared language, sorted by path.
    pub sources: Vec<PathBuf>,
    /// Relative path -> text of every package manifest.
    pub manifests: BTreeMap<String, String>,
    /// Source files that are not parsed, and why.
    pub skipped: Vec<SkippedFile>,
}

/// List the source files and manifests of `root`. Symbolic links are not
/// followed: a link with a declared source extension is reported as
/// skipped, as is a source file larger than `MAX_FILE_BYTES`.
pub fn walk(root: &Path, respect_gitignore: bool) -> Walked {
    let mut walked = Walked::default();
    // KGRAPH-073: the files of a root depend only on the root's own tree:
    // ignore files above it and the user's global excludes never apply.
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .parents(false)
        .require_git(false)
        .follow_links(false)
        .git_ignore(respect_gitignore)
        .git_global(false)
        .git_exclude(false)
        .build();
    for entry in walker.flatten() {
        let path = entry.path();
        let rel = facts::relative_path(root, path);
        if rel.split('/').any(|part| SKIP_DIRS.contains(&part)) {
            continue;
        }
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        let file_name = rel.rsplit('/').next().unwrap_or(&rel);
        let is_source = facts::languages::for_path(&rel).is_some();
        if file_type.is_symlink() {
            if is_source {
                walked.skipped.push(SkippedFile {
                    path: rel,
                    reason: SkipReason::Symlink,
                });
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if MANIFESTS.contains(&file_name) {
            if let Ok(text) = std::fs::read_to_string(path) {
                walked.manifests.insert(rel.clone(), text);
            }
        }
        if !is_source {
            continue;
        }
        let too_large = entry.metadata().is_ok_and(|m| m.len() > MAX_FILE_BYTES);
        if too_large {
            walked.skipped.push(SkippedFile {
                path: rel,
                reason: SkipReason::TooLarge,
            });
            continue;
        }
        walked.sources.push(path.to_path_buf());
    }
    walked.sources.sort();
    walked.skipped.sort();
    walked
}

/// The source files of `root` (see `walk`).
pub fn walk_source_files(root: &Path, respect_gitignore: bool) -> Vec<PathBuf> {
    walk(root, respect_gitignore).sources
}

/// Everything an index of a root produces.
#[derive(Debug)]
pub struct Indexed {
    pub entities: Vec<GraphEntity>,
    pub unresolved: UnresolvedCounts,
    pub files_parsed: usize,
    /// Files whose cached facts were reused (same path and contentHash).
    pub files_reused: usize,
    pub skipped: Vec<SkippedFile>,
    pub failed: Vec<FailedFile>,
    /// Every package manifest a dependency extractor could not read or
    /// parse (BUG-681).
    pub dependency_failures: Vec<DependencyFailure>,
    /// The facts of this tree, the cache for the next index of the root.
    pub facts: FactCache,
}

/// Walk, extract, resolve and add dependencies for `root` on `threads`
/// threads.
pub fn index_tree(root: &Path, respect_gitignore: bool, threads: usize) -> Indexed {
    index_tree_cached(root, respect_gitignore, threads, &FactCache::new())
}

/// [`index_tree`], parsing only the files whose path and contentHash are
/// not in `cache`. The resolve pass always runs over the whole tree, so the
/// entities are those of a full index.
pub fn index_tree_cached(
    root: &Path,
    respect_gitignore: bool,
    threads: usize,
    cache: &FactCache,
) -> Indexed {
    let walked = walk(root, respect_gitignore);
    let tree: TreeFacts = facts::extract_tree_cached(root, &walked.sources, threads, cache);
    let graph = resolve::build_graph(&tree, &walked.manifests);
    let mut entities = graph.entities;
    let dependency_failures = add_dependencies(root, &mut entities);
    Indexed {
        entities: deduplicate_entities(entities),
        unresolved: graph.unresolved,
        files_parsed: tree.parsed,
        files_reused: tree.reused,
        skipped: walked.skipped,
        facts: tree.cache(),
        failed: tree.failed,
        dependency_failures,
    }
}

/// Walk and extract `root` into graph entities.
pub fn walk_and_extract(root: &Path, respect_gitignore: bool) -> Vec<GraphEntity> {
    index_tree(root, respect_gitignore, facts::index_threads()).entities
}

/// Dependency nodes come from every manifest that names them: keep one
/// node per key (the first, in extractor order) and one DependsOn edge per
/// (manifest, dependency); drop DependsOn edges whose manifest has no File
/// node.
fn deduplicate_entities(entities: Vec<GraphEntity>) -> Vec<GraphEntity> {
    let mut nodes = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut files = BTreeSet::new();
    for entity in &entities {
        if let GraphEntity::Node {
            node_type, slug, ..
        } = entity
        {
            if node_type == "File" {
                files.insert(slug.clone());
            }
        }
    }
    entities
        .into_iter()
        .filter(|entity| match entity {
            GraphEntity::Node {
                node_type, slug, ..
            } => nodes.insert((node_type.clone(), slug.clone())),
            GraphEntity::Edge {
                edge_type,
                from_slug,
                to_slug,
                ..
            } => {
                // Code edges are unique from the resolve pass (TypeRef may
                // link the same pair twice, once per refKind).
                edge_type != "DependsOn"
                    || (files.contains(from_slug)
                        && edges.insert((from_slug.clone(), to_slug.clone())))
            }
        })
        .collect()
}

/// A package manifest a dependency extractor could not read or parse
/// (BUG-681): its path relative to the root, the extractor's language and
/// the exact error. The index still succeeds and lists it.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyFailure {
    pub file: String,
    pub language: String,
    pub error: String,
}

/// Dependency nodes from every supported package manager; returns every
/// manifest that could not be read or parsed, sorted.
fn add_dependencies(root: &Path, entities: &mut Vec<GraphEntity>) -> Vec<DependencyFailure> {
    type Extractor = fn(&Path) -> helpers::Extracted;
    let extractors: &[(&str, Extractor)] = &[
        ("rust", cargo_dep_extractor::extract_cargo_dependencies),
        ("javascript", npm_dep_extractor::extract_npm_dependencies),
        ("python", pip_dep_extractor::extract_python_dependencies),
        ("go", gomod_dep_extractor::extract_go_dependencies),
        ("java", java_dep_extractor::extract_java_dependencies),
        ("php", composer_dep_extractor::extract_composer_dependencies),
        ("ruby", gemfile_dep_extractor::extract_gemfile_dependencies),
        ("csharp", csproj_dep_extractor::extract_csproj_dependencies),
        ("scala", sbt_dep_extractor::extract_sbt_dependencies),
        ("swift", swift_dep_extractor::extract_swift_dependencies),
        ("dart", pubspec_dep_extractor::extract_pubspec_dependencies),
    ];
    let mut failures = Vec::new();
    for (language, extractor) in extractors {
        let extracted = extractor(root);
        entities.extend(extracted.entities);
        failures.extend(extracted.failures.into_iter().map(|f| DependencyFailure {
            file: f.file,
            language: (*language).to_string(),
            error: f.error,
        }));
    }
    failures.sort();
    failures
}
