//! Graph Database Registry — one graph per (kind, canonical root)
//!
//! KGRAPH-073: a code graph belongs to the canonical root it indexes and
//! lives at `<root>/.fspec/graph/ast-code.nano/`; the learnings graph is
//! global and lives at `<data dir>/graph/learnings.nano/`. The registry keys
//! open graphs by `(GraphKind, canonical root)`; nothing here reads the
//! process working directory.
//!
//! Access goes through leases:
//! - [`GraphRead`] holds the read side of the graph's lock for the whole
//!   query, so a write (index, reset, import) waits for running readers and
//!   readers wait while a write runs.
//! - [`GraphWrite`] holds the write side plus an OS file lock beside the
//!   database (`<db>.lock`), so a second process that tries to write the
//!   same graph fails with an explicit error instead of racing.
//!
//! The first access of a key opens (or, for writes, creates) the database
//! under the write side, so racing first callers open it once.

use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tracing::info;

use super::ast_pipeline::facts::FactCache;
use super::database::GraphDatabase;

/// Bundled schemas, compiled into the binary.
pub const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");
pub const LEARNINGS_SCHEMA: &str = include_str!("../schemas/learnings.pg");

/// The two graph kinds of the dual-graph architecture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GraphKind {
    /// The AST code graph of one canonical root.
    Code,
    /// The global learnings graph; its root is the data directory.
    Learnings,
}

impl GraphKind {
    fn schema(self) -> &'static str {
        match self {
            GraphKind::Code => AST_CODE_SCHEMA,
            GraphKind::Learnings => LEARNINGS_SCHEMA,
        }
    }

    fn database_path(self, root: &Path) -> PathBuf {
        match self {
            GraphKind::Code => root.join(".fspec/graph/ast-code.nano"),
            GraphKind::Learnings => root.join("graph/learnings.nano"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            GraphKind::Code => "code graph",
            GraphKind::Learnings => "learnings graph",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GraphKey {
    kind: GraphKind,
    root: PathBuf,
}

struct GraphSlot {
    state: Arc<RwLock<Option<GraphDatabase>>>,
    writers_waiting: AtomicUsize,
    /// KGRAPH-076: the facts of the root's last index, read and replaced
    /// only by the holder of the write lease.
    facts: Arc<Mutex<FactCache>>,
}

static REGISTRY: LazyLock<Mutex<HashMap<GraphKey, Arc<GraphSlot>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn slot(kind: GraphKind, root: &Path) -> Result<Arc<GraphSlot>, String> {
    let mut registry = REGISTRY
        .lock()
        .map_err(|e| format!("Graph registry lock poisoned: {e}"))?;
    let key = GraphKey {
        kind,
        root: root.to_path_buf(),
    };
    Ok(registry
        .entry(key)
        .or_insert_with(|| {
            Arc::new(GraphSlot {
                state: Arc::new(RwLock::new(None)),
                writers_waiting: AtomicUsize::new(0),
                facts: Arc::new(Mutex::new(FactCache::new())),
            })
        })
        .clone())
}

fn existing_slot(kind: GraphKind, root: &Path) -> Option<Arc<GraphSlot>> {
    let registry = REGISTRY.lock().ok()?;
    registry
        .get(&GraphKey {
            kind,
            root: root.to_path_buf(),
        })
        .cloned()
}

/// The canonical form of a graph root: it must be an existing directory.
pub fn canonical_root(root: &Path) -> Result<PathBuf, String> {
    let canonical = root
        .canonicalize()
        .map_err(|e| format!("Graph root {} cannot be resolved: {e}", root.display()))?;
    if !canonical.is_dir() {
        return Err(format!(
            "Graph root {} is not a directory",
            canonical.display()
        ));
    }
    Ok(canonical)
}

/// Where the code graph of `root` lives on disk.
pub fn code_graph_path(root: &Path) -> PathBuf {
    GraphKind::Code.database_path(root)
}

/// The writer lock file of the code graph of `root`.
pub fn code_graph_lock_path(root: &Path) -> PathBuf {
    lock_path(&code_graph_path(root))
}

fn lock_path(database: &Path) -> PathBuf {
    let mut name = database.as_os_str().to_owned();
    name.push(".lock");
    PathBuf::from(name)
}

fn learnings_root() -> Result<PathBuf, String> {
    codelet_common::get_data_dir()
}

/// Where the learnings graph lives on disk.
pub fn learnings_graph_path() -> Result<PathBuf, String> {
    Ok(GraphKind::Learnings.database_path(&learnings_root()?))
}

/// Whether a database exists at `database` with exactly the compiled schema.
fn on_disk_with_schema(database: &Path, schema: &str) -> bool {
    database.join("schema.ir.json").is_file()
        && is_legacy_storage(database).is_ok_and(|legacy| !legacy)
        && std::fs::read_to_string(database.join("schema.pg")).is_ok_and(|on_disk| {
            GraphDatabase::schema_hash(&on_disk) == GraphDatabase::schema_hash(schema)
        })
}

/// KGRAPH-072: whether `database` holds a graph in nanograph 1.0.0's legacy
/// v3 storage, which nanograph 1.3.0 cannot open: a database directory
/// without the storage-generation marker 1.3.0 writes.
fn is_legacy_storage(database: &Path) -> Result<bool, String> {
    if !database.join("schema.ir.json").is_file() {
        return Ok(false);
    }
    nanograph::store::storage_generation::read_storage_generation(database)
        .map(|generation| generation.is_none())
        .map_err(|e| {
            format!(
                "Failed to read the storage generation of {}: {e}",
                database.display()
            )
        })
}

/// KGRAPH-072 (product owner, 2026-11-07): a learnings graph in legacy v3
/// storage moves to a timestamped backup beside it; the caller then starts
/// a fresh, empty learnings graph. Nothing is migrated.
fn back_up_legacy_learnings(database: &Path) -> Result<(), String> {
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.fZ");
    let mut name = database.as_os_str().to_owned();
    name.push(format!(".v3-backup-{stamp}"));
    let backup = PathBuf::from(name);
    std::fs::rename(database, &backup).map_err(|e| {
        format!(
            "Failed to move the legacy learnings graph {} to {}: {e}",
            database.display(),
            backup.display()
        )
    })?;
    tracing::warn!(
        backup = %backup.display(),
        "the learnings graph was written by nanograph 1.0.0 (legacy v3 storage); \
         it was moved to the backup and a fresh learnings graph is started"
    );
    Ok(())
}

/// Open (or, when `create`, create) the database of `kind` at `database`,
/// handling legacy v3 storage first.
async fn open_database(
    kind: GraphKind,
    root: &Path,
    database: &Path,
    create: bool,
) -> Result<GraphDatabase, String> {
    if is_legacy_storage(database)? {
        match (kind, create) {
            (GraphKind::Code, false) => {
                return Err(format!(
                    "The code graph of {} was written by an older nanograph (legacy v3 storage) \
                     that nanograph 1.3.0 cannot open: run ast_index for this root to rebuild it",
                    root.display()
                ));
            }
            (GraphKind::Code, true) => {
                std::fs::remove_dir_all(database).map_err(|e| {
                    format!(
                        "Failed to delete the legacy code graph {}: {e}",
                        database.display()
                    )
                })?;
                info!(
                    ?database,
                    "deleted a legacy v3 code graph before rebuilding it"
                );
            }
            (GraphKind::Learnings, _) => back_up_legacy_learnings(database)?,
        }
    }
    if create {
        GraphDatabase::open_or_init_with_schema_check(database, kind.schema()).await
    } else {
        // KGRAPH-077: a reader opens the committed state and its version
        // from one metadata read.
        GraphDatabase::open_snapshot_with_schema_check(database, kind.schema())
    }
}

/// Whether `root` has a code graph on disk with the compiled schema. A pure
/// function of on-disk state: what this process opened does not matter.
pub fn code_graph_available(root: &Path) -> bool {
    canonical_root(root)
        .is_ok_and(|root| on_disk_with_schema(&code_graph_path(&root), AST_CODE_SCHEMA))
}

/// Whether the learnings graph exists on disk with the compiled schema.
pub fn learnings_graph_available() -> bool {
    learnings_graph_path().is_ok_and(|path| on_disk_with_schema(&path, LEARNINGS_SCHEMA))
}

/// How many writers are waiting for the write side of `root`'s code graph.
pub fn code_graph_writers_waiting(root: &Path) -> usize {
    canonical_root(root)
        .ok()
        .and_then(|root| existing_slot(GraphKind::Code, &root))
        .map_or(0, |slot| slot.writers_waiting.load(Ordering::SeqCst))
}

/// A read lease: the graph cannot be written while it is held.
pub struct GraphRead {
    guard: OwnedRwLockReadGuard<Option<GraphDatabase>, GraphDatabase>,
    root: PathBuf,
}

impl GraphRead {
    /// The canonical root this graph belongs to.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl std::ops::Deref for GraphRead {
    type Target = GraphDatabase;

    fn deref(&self) -> &GraphDatabase {
        &self.guard
    }
}

fn read_lease(
    guard: OwnedRwLockReadGuard<Option<GraphDatabase>>,
    kind: GraphKind,
    root: PathBuf,
) -> Result<GraphRead, String> {
    let guard = OwnedRwLockReadGuard::try_map(guard, Option::as_ref)
        .map_err(|_| format!("The {} of {} is not open", kind.label(), root.display()))?;
    Ok(GraphRead { guard, root })
}

/// Open the code graph of `root` for reading. Fails, creating nothing, when
/// `root` has no code graph.
pub async fn read_code_graph(root: &Path) -> Result<GraphRead, String> {
    let root = canonical_root(root)?;
    read_graph(GraphKind::Code, root).await
}

/// Open the learnings graph for reading, creating it when it does not exist.
pub async fn read_learnings_graph() -> Result<GraphRead, String> {
    read_graph(GraphKind::Learnings, learnings_root()?).await
}

async fn read_graph(kind: GraphKind, root: PathBuf) -> Result<GraphRead, String> {
    let slot = slot(kind, &root)?;
    let guard = slot.state.clone().read_owned().await;
    if guard.is_some() {
        return read_lease(guard, kind, root);
    }
    drop(guard);
    let mut guard = slot.state.clone().write_owned().await;
    if guard.is_none() {
        let database = kind.database_path(&root);
        let opened = match kind {
            GraphKind::Code => {
                if !database.join("schema.ir.json").is_file() {
                    return Err(format!(
                        "There is no code graph for {}: run ast_index for this root first",
                        root.display()
                    ));
                }
                open_database(kind, &root, &database, false).await?
            }
            GraphKind::Learnings => {
                // Creating the learnings graph or backing up a legacy one
                // writes: hold the writer lock while it happens.
                let _lock = WriterLock::acquire(&lock_path(&database))?;
                open_database(kind, &root, &database, true).await?
            }
        };
        *guard = Some(opened);
    }
    read_lease(guard.downgrade(), kind, root)
}

/// The `.gitignore` every graph directory holds: git ignores every file in
/// it, this one included (BUG-617).
const GRAPH_DIRECTORY_GITIGNORE: &str =
    "# Written by fspec: the graph store is derived data and never committed.\n*\n";

/// Make the graph directory ignore itself. Every write takes the writer
/// lock, so a graph directory gets this file at its first write and has it
/// restored at the next write when it is missing or changed.
fn ignore_graph_directory(directory: &Path) -> Result<(), String> {
    let path = directory.join(".gitignore");
    match std::fs::read_to_string(&path) {
        Ok(content) if content == GRAPH_DIRECTORY_GITIGNORE => return Ok(()),
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Failed to read {}: {e}", path.display())),
    }
    std::fs::write(&path, GRAPH_DIRECTORY_GITIGNORE)
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

/// The OS file lock a writer holds beside the database.
struct WriterLock {
    _file: File,
}

impl WriterLock {
    fn acquire(path: &Path) -> Result<Self, String> {
        let parent = path
            .parent()
            .ok_or_else(|| format!("Invalid graph lock path {}", path.display()))?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create graph directory {}: {e}", parent.display()))?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .map_err(|e| format!("Failed to open graph writer lock {}: {e}", path.display()))?;
        match file.try_lock() {
            Ok(()) => {
                ignore_graph_directory(parent)?;
                Ok(Self { _file: file })
            }
            Err(TryLockError::WouldBlock) => Err(format!(
                "The graph writer lock {} is held by another process; \
                 wait for that process's write to finish",
                path.display()
            )),
            Err(TryLockError::Error(e)) => Err(format!(
                "Failed to take the graph writer lock {}: {e}",
                path.display()
            )),
        }
    }
}

/// Counts a writer as waiting until it gets the write side (or is dropped).
struct Waiting<'a>(&'a AtomicUsize);

impl<'a> Waiting<'a> {
    fn new(counter: &'a AtomicUsize) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        Self(counter)
    }
}

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A write lease: exclusive in this process and, through the lock file,
/// against other processes. Fields drop in order, so the file lock is
/// released before the next in-process writer can take the write side.
pub struct GraphWrite {
    _lock: WriterLock,
    guard: OwnedRwLockWriteGuard<Option<GraphDatabase>>,
    facts: Arc<Mutex<FactCache>>,
    kind: GraphKind,
    root: PathBuf,
}

impl GraphWrite {
    /// The facts of the root's last index in this process (KGRAPH-076).
    pub fn fact_cache(&self) -> Result<FactCache, String> {
        self.facts
            .lock()
            .map(|facts| facts.clone())
            .map_err(|e| format!("Fact cache lock poisoned: {e}"))
    }

    /// Replace the root's fact cache with the facts of the tree just indexed.
    pub fn store_fact_cache(&self, facts: FactCache) -> Result<(), String> {
        *self
            .facts
            .lock()
            .map_err(|e| format!("Fact cache lock poisoned: {e}"))? = facts;
        Ok(())
    }

    /// The canonical root this graph belongs to.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The database, opened or created with the compiled schema.
    pub async fn database(&mut self) -> Result<&GraphDatabase, String> {
        if self.guard.is_none() {
            let database = self.kind.database_path(&self.root);
            let opened = open_database(self.kind, &self.root, &database, true).await?;
            *self.guard = Some(opened);
        }
        self.guard
            .as_ref()
            .ok_or_else(|| format!("The {} could not be opened", self.kind.label()))
    }

    /// Delete the on-disk database and empty the root's fact cache; the next
    /// [`GraphWrite::database`] call creates the database again with the
    /// compiled schema. Returns whether data was deleted.
    pub fn reset(&mut self) -> Result<bool, String> {
        *self.guard = None;
        self.store_fact_cache(FactCache::new())?;
        let database = self.kind.database_path(&self.root);
        if !database.exists() {
            return Ok(false);
        }
        std::fs::remove_dir_all(&database)
            .map_err(|e| format!("Failed to delete graph data at {}: {e}", database.display()))?;
        info!(?database, "deleted on-disk graph data");
        Ok(true)
    }
}

/// Take the write lease of the code graph of `root`.
pub async fn write_code_graph(root: &Path) -> Result<GraphWrite, String> {
    let root = canonical_root(root)?;
    write_graph(GraphKind::Code, root).await
}

/// Take the write lease of the learnings graph.
pub async fn write_learnings_graph() -> Result<GraphWrite, String> {
    write_graph(GraphKind::Learnings, learnings_root()?).await
}

async fn write_graph(kind: GraphKind, root: PathBuf) -> Result<GraphWrite, String> {
    let slot = slot(kind, &root)?;
    let guard = {
        let _waiting = Waiting::new(&slot.writers_waiting);
        slot.state.clone().write_owned().await
    };
    let lock = WriterLock::acquire(&lock_path(&kind.database_path(&root)))?;
    Ok(GraphWrite {
        _lock: lock,
        guard,
        facts: slot.facts.clone(),
        kind,
        root,
    })
}

/// Forget every open graph (called when the data directory changes).
pub fn reset_all_graphs() {
    match REGISTRY.lock() {
        Ok(mut registry) => {
            if !registry.is_empty() {
                info!(count = registry.len(), "resetting all graph databases");
            }
            registry.clear();
        }
        Err(e) => tracing::warn!("Failed to reset all graphs (lock poisoned): {e}"),
    }
}

/// Close every open graph (called on process exit).
pub fn close_all_graphs() {
    reset_all_graphs();
}
