//! Reusable Graph Database Abstraction
//!
//! Wraps a nanograph `Database` with lifecycle management (init/open/close),
//! data loading (batch JSONL), and query execution. Singletons live in
//! `registry.rs`; bundle export/import lives in `bundle.rs`.

use nanograph::query_input::JsonParamMode;
use nanograph::result::RunResult;
use nanograph::store::database::{Database, LoadMode};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::info;

use super::graph_entities::{entities_to_jsonl, GraphEntity};

/// A reusable graph database instance wrapping nanograph's `Database`.
#[derive(Clone)]
pub struct GraphDatabase {
    pub(crate) db: Database,
    path: PathBuf,
    /// Optional bundled query source (`.gq` file content).
    query_source: Option<String>,
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
        let db = Database::init(db_path, schema_source)
            .await
            .map_err(|e| format!("Failed to init graph DB at {}: {e}", db_path.display()))?;

        Ok(Self {
            db,
            path: db_path.to_path_buf(),
            query_source: None,
        })
    }

    /// Open an existing graph database from disk.
    pub async fn open(db_path: &Path) -> Result<Self, String> {
        info!(?db_path, "opening existing graph database");
        let db = Database::open(db_path)
            .await
            .map_err(|e| format!("Failed to open graph DB at {}: {e}", db_path.display()))?;

        Ok(Self {
            db,
            path: db_path.to_path_buf(),
            query_source: None,
        })
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
            // Check if on-disk schema matches compiled schema
            let on_disk_schema_path = db_path.join("schema.pg");
            if on_disk_schema_path.exists() {
                let on_disk_schema = std::fs::read_to_string(&on_disk_schema_path)
                    .map_err(|e| format!("Failed to read on-disk schema: {e}"))?;

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
            }
            Self::open(db_path).await
        } else {
            Self::init(db_path, schema_source).await
        }
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

    /// Load a batch of entities into the graph in a single JSONL call.
    pub async fn load_entities(&self, entities: &[GraphEntity]) -> Result<usize, String> {
        if entities.is_empty() {
            return Ok(0);
        }

        let jsonl = entities_to_jsonl(entities);
        self.load_jsonl(&jsonl).await?;

        let count = entities.len();
        info!(count, "loaded entity batch into graph database");
        Ok(count)
    }

    /// Load raw JSONL data into the graph.
    pub async fn load_jsonl(&self, jsonl: &str) -> Result<(), String> {
        if jsonl.trim().is_empty() {
            return Ok(());
        }

        self.db
            .load(jsonl)
            .await
            .map_err(|e| format!("Failed to load JSONL into graph: {e}"))
    }

    /// Load entities using Overwrite mode — replaces ALL existing data.
    pub async fn load_entities_overwrite(&self, entities: &[GraphEntity]) -> Result<usize, String> {
        if entities.is_empty() {
            return Ok(0);
        }

        let jsonl = entities_to_jsonl(entities);
        self.db
            .load_with_mode(&jsonl, LoadMode::Overwrite)
            .await
            .map_err(|e| format!("Failed to overwrite-load JSONL into graph: {e}"))?;

        let count = entities.len();
        info!(count, "overwrite-loaded entity batch into graph database");
        Ok(count)
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
        let result = self
            .db
            .run_json(query_source, query_name, params, JsonParamMode::Standard)
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
    pub fn stats(&self) -> Result<Value, String> {
        let storage = self.db.snapshot();
        let catalog = self.db.catalog();
        let mut stats = serde_json::Map::new();

        let mut nodes = serde_json::Map::new();
        for name in catalog.node_types.keys() {
            let c: usize = storage
                .node_segments
                .get(name.as_str())
                .map(|s| s.batches.iter().map(|b| b.num_rows()).sum())
                .unwrap_or(0);
            nodes.insert(name.clone(), Value::Number(c.into()));
        }
        stats.insert("nodes".to_string(), Value::Object(nodes));

        let mut edges = serde_json::Map::new();
        for name in catalog.edge_types.keys() {
            let c: usize = storage
                .edge_segments
                .get(name.as_str())
                .map(|s| s.batches.iter().map(|b| b.num_rows()).sum())
                .unwrap_or(0);
            edges.insert(name.clone(), Value::Number(c.into()));
        }
        stats.insert("edges".to_string(), Value::Object(edges));

        Ok(Value::Object(stats))
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
