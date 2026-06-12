//! `export-dependencies` — Rust port of `src/commands/export-dependencies.ts`
//! (RPC-227).
//!
//! Loads `spec/work-units.json` (auto-creating it when missing, via
//! [`ensure_work_units_file`] — TS parity with `ensureWorkUnitsFile`), then
//! renders the dependency graph either as a Mermaid `graph TB` diagram
//! (`format == "mermaid"`) or as a JSON dependency map keyed by work-unit id
//! (every OTHER format value — including `dot` — falls through to the JSON
//! branch, mirroring the TS `if (format === 'mermaid') … else …` shape). The
//! rendered content is written to `output` (parent directories are created
//! recursively) and the function returns the success message string
//! `✓ Dependencies exported to <output>`.
//!
//! Two-front-doors invariant: the CLI bridge and the LLM dispatcher both call
//! this `run(args_json, project_root)` function; no rendering logic is
//! duplicated at the CLI surface.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::types::work_unit::{WorkUnit, WorkUnitsData};

/// CLI / dispatcher arguments accepted by `export-dependencies`. Field names
/// mirror the positional arguments produced by the TS Commander wrapper:
/// `<format>` and `<output>`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ExportDependenciesArgs {
    /// `"mermaid"` → Mermaid diagram; any other value (including `"dot"` and
    /// `"json"`) → JSON dependency map.
    format: Option<String>,
    /// Destination file path (relative paths resolve against `project_root`).
    output: Option<String>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ExportDependenciesArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "export-dependencies",
            reason: format!("failed to parse args: {e}"),
        })?;

    let output = args.output.ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "export-dependencies",
        reason: "missing required argument: output".to_string(),
    })?;
    let format = args.format.unwrap_or_default();

    // Load work-units.json (auto-create when missing; escalate parse errors
    // with the canonical "Failed to parse work-units.json" substring).
    let data = ensure_work_units_file(project_root)?;

    let content = if format == "mermaid" {
        generate_mermaid_diagram(&data)
    } else {
        generate_json_map(&data)
    };

    // Resolve the output path relative to project_root when relative, then
    // create parent directories and write the file.
    let out_path = resolve_output(project_root, &output);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "export-dependencies",
            source,
        })?;
    }
    std::fs::write(&out_path, content).map_err(|source| FspecCoreError::Io {
        command: "export-dependencies",
        source,
    })?;

    Ok(format!("✓ Dependencies exported to {output}"))
}

/// Resolve `output` against `project_root` when it is a relative path.
fn resolve_output(project_root: &Path, output: &str) -> PathBuf {
    let p = Path::new(output);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        project_root.join(p)
    }
}

/// Extract a string array from a work-unit's `extra` JSON map, returning an
/// empty vector when the field is absent or not an array.
fn str_array(wu: &WorkUnit, key: &str) -> Vec<String> {
    wu.extra
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Build the Mermaid `graph TB` diagram — verbatim port of
/// `generateMermaidDiagram` (`src/commands/export-dependencies.ts:20-80`).
fn generate_mermaid_diagram(data: &WorkUnitsData) -> String {
    let mut lines: Vec<String> = vec!["graph TB".to_string()];

    // Node lines — one per work unit, in insertion order.
    for (id, wu) in &data.work_units {
        let status_class = match wu.status.as_str() {
            "done" => ":::done",
            "blocked" => ":::blocked",
            _ => "",
        };
        let label = if wu.title.is_empty() {
            id.clone()
        } else {
            wu.title.clone()
        };
        lines.push(format!("  {id}[\"{label}\"]{status_class}"));
    }

    // Edge lines — blocks (solid), dependsOn (dashed), relatesTo (bidi, deduped).
    let mut added_edges: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (id, wu) in &data.work_units {
        for target in str_array(wu, "blocks") {
            let key = format!("{id}-blocks-{target}");
            if added_edges.insert(key) {
                lines.push(format!("  {id} -->|blocks| {target}"));
            }
        }
        for target in str_array(wu, "dependsOn") {
            let key = format!("{id}-dependsOn-{target}");
            if added_edges.insert(key) {
                lines.push(format!("  {id} -.->|depends on| {target}"));
            }
        }
        for target in str_array(wu, "relatesTo") {
            let key = format!("{id}-relatesTo-{target}");
            let reverse = format!("{target}-relatesTo-{id}");
            if !added_edges.contains(&key) && !added_edges.contains(&reverse) {
                lines.push(format!("  {id} <-.->|relates to| {target}"));
                added_edges.insert(key);
            }
        }
    }

    // Style classes trailer.
    lines.push(String::new());
    lines.push("  classDef done fill:#90EE90".to_string());
    lines.push("  classDef blocked fill:#FFB6C1".to_string());

    lines.join("\n")
}

/// Build the JSON dependency map — verbatim port of the `else` branch
/// (`src/commands/export-dependencies.ts:94-112`). Each work unit maps to an
/// object with `blocks`, `blockedBy`, `dependsOn`, and `relatesTo` arrays.
/// Insertion order is preserved (workspace-wide `serde_json/preserve_order`).
fn generate_json_map(data: &WorkUnitsData) -> String {
    let mut map = serde_json::Map::new();
    for (id, wu) in &data.work_units {
        let entry = json!({
            "blocks": str_array(wu, "blocks"),
            "blockedBy": str_array(wu, "blockedBy"),
            "dependsOn": str_array(wu, "dependsOn"),
            "relatesTo": str_array(wu, "relatesTo"),
        });
        map.insert(id.clone(), entry);
    }
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_else(|_| "{}".to_string())
}
