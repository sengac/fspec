//! `import-example-map` — Rust port of `src/commands/import-example-map.ts`
//! (RPC-238). Inverse of `export-example-map` (RPC-228).
//!
//! Loads `spec/work-units.json` (auto-creating it when missing, via
//! [`ensure_work_units_file`] — TS parity with `ensureWorkUnitsFile`),
//! validates that the requested work unit exists AND is in `specifying`
//! status, reads a JSON file containing `rules` / `examples` / `questions` /
//! `assumptions` arrays, and APPENDS each present array to the corresponding
//! work-unit field (defaulting the existing field to an empty array). The
//! work unit's `updatedAt` is refreshed and the store is written atomically.
//!
//! On success the function returns the TS message string:
//! `✓ Imported <total> items: <r> rules, <e> examples, <q> questions, <a> assumptions`.
//!
//! NOTE: matching the TS implementation, the imported items are appended
//! verbatim (the JSON file is the source of truth for element shape). The TS
//! `import-example-map.ts` does not coerce strings into `RuleItem` objects —
//! it spreads the raw arrays straight onto the work unit — so we preserve
//! that exact behaviour here.
//!
//! Two-front-doors invariant: the CLI bridge and the LLM dispatcher both call
//! this `run(args_json, project_root)` function.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::io_error::format_io_error;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI / dispatcher arguments accepted by `import-example-map`. Field names
/// mirror the positional arguments produced by the TS Commander wrapper:
/// `<workUnitId>` and `<file>`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ImportExampleMapArgs {
    work_unit_id: Option<String>,
    file: Option<String>,
}

/// Dispatcher / CLI entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ImportExampleMapArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "import-example-map",
            reason: format!("failed to parse args: {e}"),
        })?;

    let work_unit_id = args
        .work_unit_id
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "import-example-map",
            reason: "missing required argument: workUnitId".to_string(),
        })?;
    let file = args.file.ok_or_else(|| FspecCoreError::InvalidArgs {
        command: "import-example-map",
        reason: "missing required argument: file".to_string(),
    })?;

    // Load work-units.json (auto-create when missing; escalate parse errors).
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists (TS: src/commands/import-example-map.ts:41-43).
    let wu = data
        .work_units
        .get_mut(&work_unit_id)
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "import-example-map",
            reason: format!("Work unit '{work_unit_id}' does not exist"),
        })?;

    // Validate work unit is in specifying state (TS: 47-52). The error
    // message embeds the status as its TS-lowercase form.
    let status_str = wu.status.as_str();
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "import-example-map",
            reason: format!(
                "Can only import example mapping during discovery/specification phase. {work_unit_id} is in '{status_str}' state."
            ),
        });
    }

    // Read the import JSON file (resolved relative to project_root). TS reads
    // the file at the path the user passed and surfaces the raw Node error:
    //   ENOENT: no such file or directory, open '<file>'  (missing file)
    //   <V8 JSON parser message>                          (malformed JSON)
    // — with NO extra wrapper prefix (RPC-238 parity fix). We mirror that by
    // emitting verbatim `Message` errors; the CLI bridge prepends only the
    // `✗ Failed to import example map:` catch prefix.
    let import_path = resolve_input(project_root, &file);
    let import_raw = std::fs::read_to_string(&import_path)
        .map_err(|source| FspecCoreError::Message(format_io_error(&source, &file)))?;
    let import_data: Value = serde_json::from_str(&import_raw)
        .map_err(|e| FspecCoreError::Message(crate::io::json_error::parse_json_reason(&import_raw, &e)))?;

    // Append each present array onto the matching work-unit field.
    let rules = append_field(wu, &import_data, "rules");
    let examples = append_field(wu, &import_data, "examples");
    let questions = append_field(wu, &import_data, "questions");
    let assumptions = append_field(wu, &import_data, "assumptions");

    // Bump updatedAt (TS: new Date().toISOString()).
    wu.updated_at = iso8601_now();

    // Atomic write back.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let total = rules + examples + questions + assumptions;
    Ok(format!(
        "✓ Imported {total} items: {rules} rules, {examples} examples, {questions} questions, {assumptions} assumptions"
    ))
}

/// If `import_data[key]` is a JSON array, append all its elements to the work
/// unit's `extra[key]` array (initializing it to `[]` first when absent or
/// non-array), and return the number of imported elements. Otherwise return 0
/// and leave the field untouched. Mirrors the TS pattern:
/// `workUnit.rules = [...(workUnit.rules || []), ...exampleMapData.rules]`.
fn append_field(
    wu: &mut crate::types::work_unit::WorkUnit,
    import_data: &Value,
    key: &str,
) -> usize {
    let incoming = match import_data.get(key) {
        Some(Value::Array(arr)) => arr,
        _ => return 0,
    };
    if incoming.is_empty() {
        // TS still sets imported.<key> = 0 and spreads an empty array; the
        // resulting field would be created as [] if absent. To stay faithful,
        // ensure the field exists as an array.
        let entry = wu
            .extra
            .entry(key.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
        return 0;
    }

    let entry = wu
        .extra
        .entry(key.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    if let Value::Array(existing) = entry {
        for item in incoming {
            existing.push(item.clone());
        }
    }
    incoming.len()
}

/// Resolve `file` against `project_root` when it is a relative path.
fn resolve_input(project_root: &Path, file: &str) -> std::path::PathBuf {
    let p = Path::new(file);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        project_root.join(p)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: ImportExampleMapArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","file":"emap.json"}"#).unwrap();
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.file.as_deref(), Some("emap.json"));
    }

    #[test]
    fn args_parse_empty_yields_none() {
        let a: ImportExampleMapArgs = serde_json::from_str("{}").unwrap();
        assert!(a.work_unit_id.is_none());
        assert!(a.file.is_none());
    }
}
