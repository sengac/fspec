//! Reusable Graph Database Abstraction
//!
//! Wraps a nanograph `Database` with lifecycle management (init/open/close),
//! data loading (batch JSONL), and query execution. Singletons live in
//! `registry.rs`; bundle export/import lives in `bundle.rs`.

use nanograph::query_input::JsonParamMode;
use nanograph::result::RunResult;
use nanograph::store::database::{Database, LoadMode};
use nanograph::store::metadata::DatabaseMetadata;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::info;

use super::graph_entities::{entities_to_jsonl, jsonl_to_entities, GraphEntity};
use super::query_index::QueryIndex;
use std::collections::HashMap;

/// File in a graph directory holding the SHA-256 of the entities the last
/// `ast_index` loaded (KGRAPH-076).
const INDEX_RECORD: &str = "fspec-index.sha256";

/// File in a graph directory holding the unresolved reference counts of the
/// index that last loaded the graph (KGRAPH-078); deleted with the index
/// record by every write.
const UNRESOLVED_RECORD: &str = "fspec-index-unresolved.json";

/// KGRAPH-072: box a nanograph future behind `dyn Future`. nanograph 1.3.0's
/// Lance 6 / DataFusion 53 futures are deeper than the compiler's default
/// type-layout limit; erasing them here keeps every caller's async body
/// (in this crate, its tests and its dependents) within that limit.
pub(crate) fn erased<'a, T>(
    future: impl std::future::Future<Output = T> + Send + 'a,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>> {
    Box::pin(future)
}

/// A reusable graph database instance wrapping nanograph's `Database`.
#[derive(Clone)]
pub struct GraphDatabase {
    pub(crate) db: Database,
    path: PathBuf,
    /// Optional bundled query source (`.gq` file content).
    query_source: Option<String>,
    /// KGRAPH-077: how many queries this graph has run, shared by every
    /// clone of the handle.
    queries: Arc<AtomicU64>,
    /// KGRAPH-077: the version of the graph state this handle reads.
    version: Arc<std::sync::RwLock<Snapshot>>,
    /// KGRAPH-077: the query index of the current version, built once
    /// (single flight) and dropped by every write.
    index: Arc<tokio::sync::Mutex<Option<Arc<QueryIndex>>>>,
}

/// KGRAPH-077: the version of a graph state, from nanograph's manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphVersion {
    /// The manifest's db_version: every write increases it.
    pub db_version: u64,
    /// The commit time of that state in RFC 3339.
    pub indexed_at: String,
}

/// The committed state a handle reads: its version and nanograph's metadata
/// of it (KGRAPH-078: edge rows are read at exactly this state).
#[derive(Clone)]
struct Snapshot {
    version: GraphVersion,
    metadata: DatabaseMetadata,
}

impl Snapshot {
    fn new(metadata: DatabaseMetadata) -> Result<Self, String> {
        Ok(Self {
            version: GraphVersion::from_metadata(&metadata)?,
            metadata,
        })
    }

    fn read(db_path: &Path) -> Result<Self, String> {
        Self::new(GraphVersion::metadata(db_path)?)
    }
}

impl GraphVersion {
    fn from_metadata(metadata: &DatabaseMetadata) -> Result<Self, String> {
        let manifest = metadata.manifest();
        let seconds: i64 = manifest.committed_at.parse().map_err(|e| {
            format!(
                "The graph manifest's commit time {:?} is not unix seconds: {e}",
                manifest.committed_at
            )
        })?;
        let committed = chrono::DateTime::from_timestamp(seconds, 0)
            .ok_or_else(|| format!("The graph manifest's commit time {seconds} is out of range"))?;
        Ok(Self {
            db_version: manifest.db_version,
            indexed_at: committed.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        })
    }

    fn metadata(db_path: &Path) -> Result<DatabaseMetadata, String> {
        DatabaseMetadata::open(db_path).map_err(|e| {
            format!(
                "Failed to read graph metadata at {}: {e}",
                db_path.display()
            )
        })
    }
}

impl GraphDatabase {
    /// Initialize a new graph database at `db_path` with the given schema.
    /// Creates the parent directory if needed.
    pub async fn init(db_path: &Path, schema_source: &str) -> Result<Self, String> {
        let parent = db_path
            .parent()
            .ok_or_else(|| "Invalid graph DB path".to_string())?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create graph directory: {e}"))?;

        info!(?db_path, "initializing new graph database");
        let db = erased(Database::init(db_path, schema_source))
            .await
            .map_err(|e| format!("Failed to init graph DB at {}: {e}", db_path.display()))?;
        let version = Snapshot::read(db_path)?;
        Ok(Self::from_parts(db, db_path, version))
    }

    /// Open an existing graph database from disk.
    pub async fn open(db_path: &Path) -> Result<Self, String> {
        info!(?db_path, "opening existing graph database");
        let db = erased(Database::open(db_path))
            .await
            .map_err(|e| format!("Failed to open graph DB at {}: {e}", db_path.display()))?;
        let version = Snapshot::read(db_path)?;
        Ok(Self::from_parts(db, db_path, version))
    }

    /// KGRAPH-077: open the committed state of an existing database for
    /// reading. The handle and its version come from one metadata read, so
    /// the version is exactly that of the state the handle reads, also when
    /// another process writes the graph meanwhile. Unlike [`Self::open`] it
    /// runs no storage cleanup, which belongs to writers.
    pub fn open_snapshot(db_path: &Path) -> Result<Self, String> {
        info!(?db_path, "opening graph database snapshot");
        let metadata = DatabaseMetadata::open(db_path).map_err(|e| {
            format!(
                "Failed to read graph metadata at {}: {e}",
                db_path.display()
            )
        })?;
        let version = Snapshot::new(metadata.clone())?;
        let db = Database::open_from_metadata(metadata)
            .map_err(|e| format!("Failed to open graph DB at {}: {e}", db_path.display()))?;
        Ok(Self::from_parts(db, db_path, version))
    }

    fn from_parts(db: Database, db_path: &Path, version: Snapshot) -> Self {
        Self {
            db,
            path: db_path.to_path_buf(),
            query_source: None,
            queries: Arc::new(AtomicU64::new(0)),
            version: Arc::new(std::sync::RwLock::new(version)),
            index: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    /// Open an existing database, or initialize a new one if it doesn't exist.
    pub async fn open_or_init(db_path: &Path, schema_source: &str) -> Result<Self, String> {
        if db_path.exists() && db_path.join("schema.ir.json").exists() {
            Self::open(db_path).await
        } else {
            Self::init(db_path, schema_source).await
        }
    }

    /// Open an existing database with schema hash validation, or init a new one.
    /// Returns an error if the on-disk schema differs from the compiled schema.
    pub async fn open_or_init_with_schema_check(
        db_path: &Path,
        schema_source: &str,
    ) -> Result<Self, String> {
        if db_path.exists() && db_path.join("schema.ir.json").exists() {
            Self::open_with_schema_check(db_path, schema_source).await
        } else {
            Self::init(db_path, schema_source).await
        }
    }

    /// Open an existing database, failing when its on-disk schema differs
    /// from the compiled schema.
    pub async fn open_with_schema_check(
        db_path: &Path,
        schema_source: &str,
    ) -> Result<Self, String> {
        Self::check_schema(db_path, schema_source)?;
        Self::open(db_path).await
    }

    /// Open the committed state of an existing database for reading (see
    /// [`Self::open_snapshot`]), failing when its on-disk schema differs
    /// from the compiled schema.
    pub fn open_snapshot_with_schema_check(
        db_path: &Path,
        schema_source: &str,
    ) -> Result<Self, String> {
        Self::check_schema(db_path, schema_source)?;
        Self::open_snapshot(db_path)
    }

    fn check_schema(db_path: &Path, schema_source: &str) -> Result<(), String> {
        let on_disk_schema_path = db_path.join("schema.pg");
        let on_disk_schema = std::fs::read_to_string(&on_disk_schema_path).map_err(|e| {
            format!(
                "Failed to read on-disk schema {}: {e}",
                on_disk_schema_path.display()
            )
        })?;
        let compiled_hash = Self::schema_hash(schema_source);
        let on_disk_hash = Self::schema_hash(&on_disk_schema);
        if compiled_hash != on_disk_hash {
            return Err(format!(
                "Schema has changed (compiled hash: {}… ≠ on-disk hash: {}…). \
                 The existing database at {} is incompatible with the current schema. \
                 Run ast_index with reset: true to delete the old database and rebuild \
                 with the new schema.",
                &compiled_hash[..12],
                &on_disk_hash[..12],
                db_path.display(),
            ));
        }
        Ok(())
    }

    /// Compute a hex-encoded SHA-256 hash of a schema source string.
    pub(crate) fn schema_hash(schema_source: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(schema_source.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Set the bundled query source for named query execution.
    pub fn with_query_source(mut self, source: &str) -> Self {
        self.query_source = Some(source.to_string());
        self
    }

    /// The on-disk path of this database.
    pub fn path(&self) -> &Path {
        &self.path
    }

    // ── Data Loading ──────────────────────────────────────────
    //
    // KGRAPH-072 (product owner, 2026-11-07): fspec never uses a nanograph
    // Merge load. In nanograph 1.3.0 a Merge load of one edge type makes
    // traversals of other edge types return nothing, so every write here
    // replaces the whole graph, every node and edge type, with one
    // Overwrite load: `overwrite` is the only load call in codelet-graph.

    /// Merge a batch of entities into the graph: a node replaces the node
    /// of the same type with the same `@key`, every other row is added, and
    /// the whole graph is written back with one Overwrite load.
    pub async fn load_entities(&self, entities: &[GraphEntity]) -> Result<usize, String> {
        if entities.is_empty() {
            return Ok(0);
        }
        self.merge_entities(entities.to_vec()).await?;
        let count = entities.len();
        info!(count, "merged entity batch into graph database");
        Ok(count)
    }

    /// Merge raw JSONL rows into the graph (see [`GraphDatabase::load_entities`]).
    pub async fn load_jsonl(&self, jsonl: &str) -> Result<(), String> {
        if jsonl.trim().is_empty() {
            return Ok(());
        }
        self.merge_entities(jsonl_to_entities(jsonl)?).await
    }

    /// Replace ALL existing data with `entities`; an empty batch leaves an
    /// empty graph.
    pub async fn load_entities_overwrite(&self, entities: &[GraphEntity]) -> Result<usize, String> {
        self.overwrite(&entities_to_jsonl(entities)).await?;
        let count = entities.len();
        info!(count, "overwrite-loaded entity batch into graph database");
        Ok(count)
    }

    async fn merge_entities(&self, incoming: Vec<GraphEntity>) -> Result<(), String> {
        let mut merged = self.export_all_entities().await?;
        let catalog = self.db.catalog();
        let key_of = |node_type: &str, properties: &serde_json::Map<String, Value>| {
            let key = catalog.node_types.get(node_type)?.key_property.as_ref()?;
            Some((node_type.to_string(), properties.get(key)?.to_string()))
        };
        let mut by_key: HashMap<(String, String), usize> = HashMap::new();
        for (index, entity) in merged.iter().enumerate() {
            if let GraphEntity::Node {
                node_type,
                properties,
                ..
            } = entity
            {
                if let Some(key) = key_of(node_type, properties) {
                    by_key.insert(key, index);
                }
            }
        }
        for entity in incoming {
            let key = match &entity {
                GraphEntity::Node {
                    node_type,
                    properties,
                    ..
                } => key_of(node_type, properties),
                GraphEntity::Edge { .. } => None,
            };
            match key {
                Some(key) => match by_key.get(&key) {
                    Some(&index) => merged[index] = entity,
                    None => {
                        by_key.insert(key, merged.len());
                        merged.push(entity);
                    }
                },
                None => merged.push(entity),
            }
        }
        self.overwrite(&entities_to_jsonl(&merged)).await
    }

    /// Replace the whole graph with raw JSONL rows (see
    /// [`GraphDatabase::load_entities_overwrite`]).
    pub(crate) async fn load_jsonl_overwrite(&self, jsonl: &str) -> Result<(), String> {
        self.overwrite(jsonl).await
    }

    /// The one nanograph load call: replace the whole graph. The index
    /// record is deleted first, so after any write only a new record from
    /// `ast_index` names the graph's entities (KGRAPH-076).
    async fn overwrite(&self, jsonl: &str) -> Result<(), String> {
        for record in [self.index_record_path(), self.path.join(UNRESOLVED_RECORD)] {
            match std::fs::remove_file(&record) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(format!(
                        "Failed to delete the index record {}: {e}",
                        record.display()
                    ))
                }
            }
        }
        let mut index = self.index.lock().await;
        *index = None;
        erased(self.db.load_with_mode(jsonl, LoadMode::Overwrite))
            .await
            .map_err(|e| format!("Failed to overwrite-load JSONL into graph: {e}"))?;
        let version = Snapshot::read(&self.path)?;
        *self
            .version
            .write()
            .map_err(|e| format!("Graph version lock poisoned: {e}"))? = version;
        Ok(())
    }

    // ── Version and query index (KGRAPH-077) ──────────────────

    /// The version of the graph state this handle reads.
    pub fn version(&self) -> Result<GraphVersion, String> {
        self.version
            .read()
            .map(|snapshot| snapshot.version.clone())
            .map_err(|e| format!("Graph version lock poisoned: {e}"))
    }

    /// The portable rows (`{edge, from, to, data}`) of one edge type at the
    /// state this handle reads (KGRAPH-078): a traversal query cannot
    /// return edge properties.
    pub async fn edge_rows(&self, edge_type: &str) -> Result<Vec<Value>, String> {
        let metadata = self
            .version
            .read()
            .map(|snapshot| snapshot.metadata.clone())
            .map_err(|e| format!("Graph version lock poisoned: {e}"))?;
        erased(nanograph::store::export::build_edge_export_rows(
            &metadata, edge_type,
        ))
        .await
        .map_err(|e| format!("Failed to read the {edge_type} edges: {e}"))
    }

    /// The query index of the current graph version: built by a fixed
    /// number of whole-graph queries the first time it is needed after a
    /// write, then shared by every query until the next write.
    pub async fn query_index(&self) -> Result<Arc<QueryIndex>, String> {
        let mut index = self.index.lock().await;
        let version = self.version()?.db_version;
        if let Some(current) = index.as_ref().filter(|i| i.version == version) {
            return Ok(current.clone());
        }
        let built = Arc::new(QueryIndex::load(self, version).await?);
        *index = Some(built.clone());
        Ok(built)
    }

    // ── Index record (KGRAPH-076) ─────────────────────────────

    fn index_record_path(&self) -> PathBuf {
        self.path.join(INDEX_RECORD)
    }

    /// The SHA-256 of the entities `ast_index` last loaded, when no other
    /// write has happened since; `None` when there is no record.
    pub(crate) fn index_record(&self) -> Result<Option<String>, String> {
        let record = self.index_record_path();
        match std::fs::read_to_string(&record) {
            Ok(hash) => Ok(Some(hash)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!(
                "Failed to read the index record {}: {e}",
                record.display()
            )),
        }
    }

    /// The unresolved reference counts of the index that last loaded the
    /// graph; `None` when another write happened since (KGRAPH-078).
    pub fn unresolved_record(&self) -> Result<Option<Value>, String> {
        let record = self.path.join(UNRESOLVED_RECORD);
        match std::fs::read_to_string(&record) {
            Ok(text) => serde_json::from_str(&text).map(Some).map_err(|e| {
                format!(
                    "Failed to parse the unresolved record {}: {e}",
                    record.display()
                )
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!(
                "Failed to read the unresolved record {}: {e}",
                record.display()
            )),
        }
    }

    /// Record the unresolved reference counts of the index whose entities
    /// the graph holds.
    pub(crate) fn write_unresolved_record(&self, unresolved: &Value) -> Result<(), String> {
        let record = self.path.join(UNRESOLVED_RECORD);
        std::fs::write(&record, unresolved.to_string()).map_err(|e| {
            format!(
                "Failed to write the unresolved record {}: {e}",
                record.display()
            )
        })
    }

    /// Record the SHA-256 of the entities just loaded.
    pub(crate) fn write_index_record(&self, hash: &str) -> Result<(), String> {
        let record = self.index_record_path();
        std::fs::write(&record, hash)
            .map_err(|e| format!("Failed to write the index record {}: {e}", record.display()))
    }

    // ── Querying ──────────────────────────────────────────────

    /// Run a named query from the bundled query source.
    /// Requires `with_query_source()` to have been called.
    pub async fn query(&self, query_name: &str, params: Option<&Value>) -> Result<Value, String> {
        let source = self
            .query_source
            .as_deref()
            .ok_or("No query source configured — call with_query_source() first")?;

        self.query_with_source(source, query_name, params).await
    }

    /// Run a named query from an explicit query source string.
    pub async fn query_with_source(
        &self,
        query_source: &str,
        query_name: &str,
        params: Option<&Value>,
    ) -> Result<Value, String> {
        self.queries.fetch_add(1, Ordering::SeqCst);
        let result =
            erased(
                self.db
                    .run_json(query_source, query_name, params, JsonParamMode::Standard),
            )
            .await
            .map_err(|e| format!("Graph query '{query_name}' failed: {e}"))?;

        match result {
            RunResult::Query(qr) => Ok(qr.to_rust_json()),
            RunResult::Mutation(mr) => Ok(serde_json::json!({
                "affected_nodes": mr.affected_nodes,
                "affected_edges": mr.affected_edges,
            })),
        }
    }

    /// How many queries this graph has run since it was opened (KGRAPH-077).
    pub fn queries_run(&self) -> u64 {
        self.queries.load(Ordering::SeqCst)
    }

    // ── Schema Inspection ─────────────────────────────────────

    /// Check if the schema contains a specific node type.
    pub fn has_node_type(&self, name: &str) -> bool {
        self.db.catalog().node_types.contains_key(name)
    }

    /// Check if the schema contains a specific edge type.
    pub fn has_edge_type(&self, name: &str) -> bool {
        self.db.catalog().edge_types.contains_key(name)
    }

    /// List all node type names in the schema.
    pub fn node_type_names(&self) -> Vec<String> {
        self.db.catalog().node_types.keys().cloned().collect()
    }

    /// List all edge type names in the schema.
    pub fn edge_type_names(&self) -> Vec<String> {
        self.db.catalog().edge_types.keys().cloned().collect()
    }

    /// Check if a node type has a specific property.
    pub fn node_has_property(&self, node_type: &str, prop_name: &str) -> bool {
        self.db
            .catalog()
            .node_types
            .get(node_type)
            .is_some_and(|nt| nt.properties.contains_key(prop_name))
    }

    /// Get node/edge type counts from storage.
    ///
    /// KGRAPH-072: the counts are the committed row counts in nanograph's
    /// manifest metadata; a type without a dataset has no rows.
    pub fn stats(&self) -> Result<Value, String> {
        let metadata = DatabaseMetadata::open(&self.path).map_err(|e| {
            format!(
                "Failed to read graph metadata at {}: {e}",
                self.path.display()
            )
        })?;
        let count = |kind: &str, name: &str| -> Value {
            let rows = metadata
                .dataset_entry(kind, name)
                .map_or(0, |entry| entry.row_count);
            Value::Number(rows.into())
        };
        let catalog = self.db.catalog();
        let nodes: serde_json::Map<String, Value> = catalog
            .node_types
            .keys()
            .map(|name| (name.clone(), count("node", name)))
            .collect();
        let edges: serde_json::Map<String, Value> = catalog
            .edge_types
            .keys()
            .map(|name| (name.clone(), count("edge", name)))
            .collect();
        Ok(serde_json::json!({ "nodes": nodes, "edges": edges }))
    }

    /// Describe all node and edge types in human-readable format.
    pub fn describe_schema(&self) -> String {
        let catalog = self.db.catalog();
        let mut desc = String::new();
        desc.push_str("=== Node Types ===\n");
        for (name, nt) in &catalog.node_types {
            desc.push_str(&format!("  {name}\n"));
            for (pn, pt) in &nt.properties {
                desc.push_str(&format!("    - {}: {}\n", pn, pt.display_name()));
            }
        }
        desc.push_str("\n=== Edge Types ===\n");
        for (name, et) in &catalog.edge_types {
            desc.push_str(&format!(
                "  {} ({} -> {})\n",
                name, et.from_type, et.to_type
            ));
            for (pn, pt) in &et.properties {
                desc.push_str(&format!("    - {}: {}\n", pn, pt.display_name()));
            }
        }
        desc
    }
}

impl std::fmt::Debug for GraphDatabase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphDatabase")
            .field("path", &self.path)
            .field("has_query_source", &self.query_source.is_some())
            .finish()
    }
}
