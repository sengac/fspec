//! Portable Graph Bundle Export / Import (KGRAPH-069)
//!
//! Extends `GraphDatabase` with methods for exporting the entire graph to a
//! `.astbundle` ZIP archive and importing it back with schema validation.
//!
//! Extracted from `database.rs` to keep that file focused on core lifecycle
//! (init/open/close, data loading, querying).

use std::collections::HashMap;
use std::path::Path;

use tracing::info;

use super::database::GraphDatabase;
use super::graph_entities::{entities_to_jsonl, jsonl_to_entities, GraphEntity};

impl GraphDatabase {
    /// Export the entire graph to a portable `.astbundle` ZIP archive.
    ///
    /// The bundle contains:
    /// - `entities.jsonl` — all nodes and edges in nanograph JSONL format
    /// - `metadata.json` — version, timestamp, entity counts
    /// - `schema.pg` — the schema source for compatibility validation on import
    pub async fn export_bundle(
        &self,
        output_path: &Path,
        schema_source: &str,
    ) -> Result<(), String> {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        use zip::ZipWriter;

        let entities = self.export_all_entities()?;
        let jsonl = entities_to_jsonl(&entities);

        // Count nodes and edges
        let (node_count, edge_count) = entities.iter().fold((0u64, 0u64), |(n, e), ent| {
            match ent {
                GraphEntity::Node { .. } => (n + 1, e),
                GraphEntity::Edge { .. } => (n, e + 1),
            }
        });

        // Build metadata
        let metadata = serde_json::json!({
            "version": "1.0.0",
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "node_count": node_count,
            "edge_count": edge_count,
        });

        // Write ZIP archive
        let file = std::fs::File::create(output_path)
            .map_err(|e| format!("Failed to create bundle file: {e}"))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("entities.jsonl", options)
            .map_err(|e| format!("ZIP write error: {e}"))?;
        zip.write_all(jsonl.as_bytes())
            .map_err(|e| format!("ZIP write error: {e}"))?;

        zip.start_file("metadata.json", options)
            .map_err(|e| format!("ZIP write error: {e}"))?;
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("JSON serialization error: {e}"))?;
        zip.write_all(metadata_json.as_bytes())
            .map_err(|e| format!("ZIP write error: {e}"))?;

        zip.start_file("schema.pg", options)
            .map_err(|e| format!("ZIP write error: {e}"))?;
        zip.write_all(schema_source.as_bytes())
            .map_err(|e| format!("ZIP write error: {e}"))?;

        zip.finish()
            .map_err(|e| format!("ZIP finalize error: {e}"))?;

        info!(
            node_count,
            edge_count,
            path = %output_path.display(),
            "exported graph bundle"
        );
        Ok(())
    }

    /// Import a `.astbundle` ZIP archive into the graph.
    ///
    /// Validates schema compatibility before loading. Supports two modes:
    /// - `"overwrite"` (default) — replaces all existing data
    /// - `"merge"` — upserts via slug-based key matching
    pub async fn import_bundle(
        &self,
        bundle_path: &Path,
        current_schema: &str,
        mode: &str,
    ) -> Result<(), String> {
        use std::io::Read;

        let file = std::fs::File::open(bundle_path)
            .map_err(|e| format!("Failed to open bundle: {e}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Invalid ZIP archive: {e}"))?;

        // 1. Validate schema compatibility
        let mut schema_content = String::new();
        archive
            .by_name("schema.pg")
            .map_err(|e| format!("Bundle missing schema.pg: {e}"))?
            .read_to_string(&mut schema_content)
            .map_err(|e| format!("Failed to read schema.pg: {e}"))?;

        let bundle_hash = Self::schema_hash(&schema_content);
        let current_hash = Self::schema_hash(current_schema);
        if bundle_hash != current_hash {
            return Err(format!(
                "Schema mismatch: bundle schema hash {}… ≠ current schema hash {}…. \
                 The bundle was created with an incompatible schema version.",
                &bundle_hash[..12],
                &current_hash[..12],
            ));
        }

        // 2. Read entities JSONL
        let mut jsonl_content = String::new();
        archive
            .by_name("entities.jsonl")
            .map_err(|e| format!("Bundle missing entities.jsonl: {e}"))?
            .read_to_string(&mut jsonl_content)
            .map_err(|e| format!("Failed to read entities.jsonl: {e}"))?;

        let entities = jsonl_to_entities(&jsonl_content)?;
        let count = entities.len();

        // 3. Load into graph
        match mode {
            "merge" => {
                self.load_entities(&entities).await?;
            }
            _ => {
                // Default: overwrite
                self.load_entities_overwrite(&entities).await?;
            }
        }

        info!(count, mode, path = %bundle_path.display(), "imported graph bundle");
        Ok(())
    }

    /// Read all nodes and edges from the graph snapshot as `GraphEntity` values.
    ///
    /// Iterates Arrow record batches directly for nodes and uses
    /// `edge_batch_for_save` for edges (resolving numeric IDs to slugs).
    pub fn export_all_entities(&self) -> Result<Vec<GraphEntity>, String> {
        use arrow_array::cast::AsArray;
        use arrow_array::types::UInt64Type;

        let storage = self.db.snapshot();
        let mut entities = Vec::new();

        // Phase 1: Export nodes and build node-id → slug map
        let mut id_to_slug: HashMap<u64, String> = HashMap::new();

        for (type_name, segment) in &storage.node_segments {
            for batch in &segment.batches {
                if batch.num_rows() == 0 {
                    continue;
                }
                let schema = batch.schema();
                // Column 0 is the internal `id` (UInt64)
                let id_col = batch.column(0).as_primitive::<UInt64Type>();

                for row in 0..batch.num_rows() {
                    let node_id = id_col.value(row);
                    let mut props = serde_json::Map::new();

                    // Columns 1..n are user-defined properties
                    for col_idx in 1..batch.num_columns() {
                        let field = schema.field(col_idx);
                        if let Some(val) = arrow_value_to_json(
                            batch.column(col_idx).as_ref(),
                            row,
                            field.data_type(),
                        ) {
                            props.insert(field.name().clone(), val);
                        }
                    }

                    let slug = props
                        .get("slug")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    id_to_slug.insert(node_id, slug.clone());

                    entities.push(GraphEntity::Node {
                        node_type: type_name.clone(),
                        slug,
                        properties: props,
                    });
                }
            }
        }

        // Phase 2: Export edges — resolve numeric IDs to slugs
        for (type_name, _segment) in &storage.edge_segments {
            if let Ok(Some(batch)) = storage.edge_batch_for_save(type_name) {
                if batch.num_rows() == 0 {
                    continue;
                }
                let schema = batch.schema();
                // Columns: [0]=id, [1]=src, [2]=dst, [3..]=props
                let src_col = batch.column(1).as_primitive::<UInt64Type>();
                let dst_col = batch.column(2).as_primitive::<UInt64Type>();

                for row in 0..batch.num_rows() {
                    let from_slug = id_to_slug
                        .get(&src_col.value(row))
                        .cloned()
                        .unwrap_or_default();
                    let to_slug = id_to_slug
                        .get(&dst_col.value(row))
                        .cloned()
                        .unwrap_or_default();

                    let mut props = serde_json::Map::new();
                    for col_idx in 3..batch.num_columns() {
                        let field = schema.field(col_idx);
                        if let Some(val) = arrow_value_to_json(
                            batch.column(col_idx).as_ref(),
                            row,
                            field.data_type(),
                        ) {
                            props.insert(field.name().clone(), val);
                        }
                    }

                    entities.push(GraphEntity::Edge {
                        edge_type: type_name.clone(),
                        from_slug,
                        to_slug,
                        properties: props,
                    });
                }
            }
        }

        Ok(entities)
    }
}

/// Convert a single Arrow array value at `row` to a `serde_json::Value`.
///
/// Returns `None` for null values (so they can be omitted from the property map).
/// Handles the types used in the AST code schema: Utf8, Int32, UInt64, Boolean, Date64.
fn arrow_value_to_json(
    array: &dyn arrow_array::Array,
    row: usize,
    data_type: &arrow_schema::DataType,
) -> Option<serde_json::Value> {
    use arrow_array::cast::AsArray;
    use arrow_schema::DataType;

    if array.is_null(row) {
        return None;
    }

    match data_type {
        DataType::Utf8 => {
            let arr = array.as_string::<i32>();
            Some(serde_json::Value::String(arr.value(row).to_string()))
        }
        DataType::LargeUtf8 => {
            let arr = array.as_string::<i64>();
            Some(serde_json::Value::String(arr.value(row).to_string()))
        }
        DataType::Int32 => {
            let arr = array.as_primitive::<arrow_array::types::Int32Type>();
            Some(serde_json::Value::Number(arr.value(row).into()))
        }
        DataType::UInt64 => {
            let arr = array.as_primitive::<arrow_array::types::UInt64Type>();
            Some(serde_json::Value::Number(arr.value(row).into()))
        }
        DataType::Boolean => {
            let arr = array.as_boolean();
            Some(serde_json::Value::Bool(arr.value(row)))
        }
        DataType::Date64 => {
            let arr = array.as_primitive::<arrow_array::types::Date64Type>();
            let millis = arr.value(row);
            // Convert epoch millis to ISO string
            if let Some(dt) = arrow_array::temporal_conversions::date64_to_datetime(millis) {
                Some(serde_json::Value::String(
                    dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                ))
            } else {
                Some(serde_json::Value::Number(millis.into()))
            }
        }
        _ => {
            // Unsupported type — skip
            None
        }
    }
}
