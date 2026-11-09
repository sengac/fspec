//! Portable Graph Bundle Export / Import (KGRAPH-069)
//!
//! Extends `GraphDatabase` with methods for exporting the entire graph to a
//! `.astbundle` ZIP archive and importing it back with schema validation.
//!
//! Extracted from `database.rs` to keep that file focused on core lifecycle
//! (init/open/close, data loading, querying).

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

        let entities = self.export_all_entities().await?;
        let jsonl = entities_to_jsonl(&entities);

        // Count nodes and edges
        let (node_count, edge_count) =
            entities.iter().fold((0u64, 0u64), |(n, e), ent| match ent {
                GraphEntity::Node { .. } => (n + 1, e),
                GraphEntity::Edge { .. } => (n, e + 1),
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
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

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

        let file =
            std::fs::File::open(bundle_path).map_err(|e| format!("Failed to open bundle: {e}"))?;
        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {e}"))?;

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

    /// Read all nodes and edges of the graph as `GraphEntity` values.
    ///
    /// KGRAPH-072: nanograph 1.3.0's export API reads the committed datasets
    /// and returns portable rows (`{type,data}` / `{edge,from,to,data}`, edge
    /// endpoints by `@key`), the same shape the loader takes.
    pub async fn export_all_entities(&self) -> Result<Vec<GraphEntity>, String> {
        let rows = super::database::erased(nanograph::store::export::build_export_rows_at_path(
            self.path(),
            false,
            false,
        ))
        .await
        .map_err(|e| format!("Failed to export graph at {}: {e}", self.path().display()))?;
        let jsonl = rows
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        jsonl_to_entities(&jsonl)
    }
}
