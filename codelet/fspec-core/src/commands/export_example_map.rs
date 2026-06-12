//! `export-example-map` — Rust port of `src/commands/export-example-map.ts`
//! (RPC-228).
//!
//! Loads `spec/work-units.json` (auto-creating it when missing, via
//! [`ensure_work_units_file`] — TS parity with `ensureWorkUnitsFile`),
//! validates that the requested work unit exists, then writes a JSON document
//! with the fields `workUnitId`, `title`, `rules`, `examples`, `questions`,
//! and `assumptions` (in that order). The four example-mapping arrays are
//! copied verbatim from the work unit (defaulting to empty arrays when
//! absent). Parent directories of the output path are created recursively.
//! On success the function returns the message string `✓ Exported to <file>`.
//!
//! Two-front-doors invariant: the CLI bridge and the LLM dispatcher both call
//! this `run(args_json, project_root)` function.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

/// CLI / dispatcher arguments accepted by `export-example-map`. Field names
/// mirror the positional arguments produced by the TS Commander wrapper:
/// `<workUnitId>` and `<file>`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ExportExampleMapArgs {
    work_unit_id: Option<String>,
    file: Option<String>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ExportExampleMapArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "export-example-map",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args.work_unit_id.ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "export-example-map",
        reason: "missing required argument: workUnitId".to_string(),
    })?;
    let file = args.file.ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "export-example-map",
        reason: "missing required argument: file".to_string(),
    })?;

    // Load work-units.json (auto-create when missing; escalate parse errors
    // with the canonical "Failed to parse work-units.json" substring).
    let data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (TS: src/commands/export-example-map.ts:34-36).
    let wu = data
        .work_units
        .get(&work_unit_id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "export-example-map",
            reason: format!("Work unit '{work_unit_id}' does not exist"),
        })?;

    // Build the export document with the canonical field order:
    // workUnitId, title, rules, examples, questions, assumptions.
    let mut export = serde_json::Map::new();
    export.insert("workUnitId".to_string(), Value::String(work_unit_id.clone()));
    export.insert("title".to_string(), Value::String(wu.title.clone()));
    export.insert("rules".to_string(), extra_array(wu.extra.get("rules")));
    export.insert("examples".to_string(), extra_array(wu.extra.get("examples")));
    export.insert(
        "questions".to_string(),
        extra_array(wu.extra.get("questions")),
    );
    export.insert(
        "assumptions".to_string(),
        extra_array(wu.extra.get("assumptions")),
    );

    let content = serde_json::to_string_pretty(&Value::Object(export))
        .unwrap_or_else(|_| "{}".to_string());

    // Resolve the output path relative to project_root, create parents, write.
    let out_path = resolve_output(project_root, &file);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "export-example-map",
            source,
        })?;
    }
    std::fs::write(&out_path, content).map_err(|source| FspecCoreError::Io {
        command: "export-example-map",
        source,
    })?;

    Ok(format!("✓ Exported to {file}"))
}

/// Return the given JSON value when it is an array, otherwise an empty array.
/// Mirrors the TS `workUnit.rules || []` defaulting (and likewise for
/// examples / questions / assumptions).
fn extra_array(value: Option<&Value>) -> Value {
    match value {
        Some(v @ Value::Array(_)) => v.clone(),
        _ => Value::Array(Vec::new()),
    }
}

/// Resolve `file` against `project_root` when it is a relative path.
fn resolve_output(project_root: &Path, file: &str) -> PathBuf {
    let p = Path::new(file);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        project_root.join(p)
    }
}
