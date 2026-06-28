//! `search-implementation` — Rust port of
//! `src/commands/search-implementation.ts` (RPC-296, part of QRY-002).
//!
//! Read-only command: reads every `spec/features/*.feature.coverage` sidecar,
//! extracts each implementation-file reference from
//! `scenarios[].testMappings[].implMappings[]`, then performs a simple
//! (non-AST) substring search for the supplied `function` name in the on-disk
//! content of each referenced file. Files that contain the substring are
//! reported with the work-unit ids derived from the owning feature names.
//!
//! Returns a JSON envelope (field order mirrors the TS result object):
//!
//! ```json
//! {
//!   "searchedFiles": <usize>,
//!   "files": [ { "content", "filePath", "workUnits": [ { "workUnitId" } ] }, ... ],
//!   "message"?: "✓ Found \"<function>\" in N file(s)"
//! }
//! ```
//!
//! The `message` field is emitted ONLY in the default (`format == "table"`)
//! path — mirroring `show_test_patterns`: the CLI bridge surfaces it as the
//! green summary line, while the `--json` envelope stays byte-equivalent to
//! the TS `JSON.stringify({ searchedFiles, files })`.
//!
//! Two-front-doors invariant: the LLM dispatcher AND the standalone clap CLI
//! both call this single `run` function.
//!
//! ## Parity notes
//!
//! * `searchedFiles` counts every impl-mapping reference (parity with TS
//!   `implFiles.length`), NOT the number of matching files.
//! * Missing `spec/features/` directory → `searchedFiles: 0`, empty `files`
//!   (parity with TS `glob([...])` returning `[]`).
//! * Implementation files that cannot be read on disk are skipped silently.
//! * `workUnitId` is the owning feature name upper-cased (parity with the TS
//!   `featureName.toUpperCase()`), e.g. `user-login` → `USER-LOGIN`.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;

/// CLI / dispatcher arguments accepted by `search-implementation`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SearchImplementationArgs {
    #[serde(default)]
    function: Option<String>,
    // Accepted for arg-shape parity with the TS Commander.js flag set; the
    // table/json split is decided by `json`, and `show_work_units` does not
    // alter the structured envelope (the CLI renders it). Silence dead_code.
    #[serde(default)]
    #[allow(dead_code)]
    show_work_units: Option<bool>,
    #[serde(default)]
    json: Option<bool>,
}

/// One implementation-file reference extracted from a coverage sidecar.
struct ImplRef {
    file_path: String,
    feature_name: String,
}

/// Dispatcher / CLI entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: SearchImplementationArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "search-implementation",
            reason: format!("failed to parse args: {e}"),
        })?;

    let function = args
        .function
        .as_deref()
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "search-implementation",
            reason: "missing required 'function' argument".to_string(),
        })?;

    let json_out = args.json.unwrap_or(false);

    // Collect every impl-file reference across all coverage sidecars, in
    // encounter order. `searchedFiles` is the total reference count.
    let impl_refs = read_impl_refs(project_root);
    let searched_files = impl_refs.len();

    // Determine the ordered, unique set of impl files whose on-disk content
    // contains the function name. Cache reads so each file is read once.
    let mut content_cache: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let mut matching_order: Vec<String> = Vec::new();

    for r in &impl_refs {
        let entry = content_cache
            .entry(r.file_path.clone())
            .or_insert_with(|| std::fs::read_to_string(project_root.join(&r.file_path)).ok());
        let contains = entry
            .as_ref()
            .map(|c| c.contains(function))
            .unwrap_or(false);
        if contains && !matching_order.contains(&r.file_path) {
            matching_order.push(r.file_path.clone());
        }
    }

    // Build the files array. For each matching file, work-units are the
    // upper-cased owning feature names (deduplicated, in encounter order).
    let mut files: Vec<Value> = Vec::new();
    for file_path in &matching_order {
        let content = content_cache
            .get(file_path)
            .and_then(Clone::clone)
            .unwrap_or_default();

        let mut work_unit_ids: Vec<String> = Vec::new();
        for r in &impl_refs {
            if &r.file_path == file_path {
                let id = r.feature_name.to_uppercase();
                if !work_unit_ids.contains(&id) {
                    work_unit_ids.push(id);
                }
            }
        }
        let work_units: Vec<Value> = work_unit_ids
            .into_iter()
            .map(|id| json!({ "workUnitId": id }))
            .collect();

        let mut obj = serde_json::Map::new();
        obj.insert("content".to_string(), json!(content));
        obj.insert("filePath".to_string(), json!(file_path));
        obj.insert("workUnits".to_string(), Value::Array(work_units));
        files.push(Value::Object(obj));
    }

    let file_count = files.len();

    let mut envelope = serde_json::Map::new();
    envelope.insert("searchedFiles".to_string(), json!(searched_files));
    envelope.insert("files".to_string(), Value::Array(files));
    if !json_out {
        // Mirrors the TS green summary line: `✓ Found "<fn>" in N file(s)`.
        envelope.insert(
            "message".to_string(),
            json!(format!(
                "\u{2713} Found \"{function}\" in {file_count} file(s)"
            )),
        );
    }

    serde_json::to_string_pretty(&Value::Object(envelope)).map_err(|e| {
        FspecCoreError::InvalidArgs {
            command: "search-implementation",
            reason: format!("failed to serialize result: {e}"),
        }
    })
}

/// Walk `spec/features/*.feature.coverage` and return every implementation-file
/// reference (`implMappings[].file`) paired with its owning feature name (the
/// sidecar filename minus the `.feature.coverage` suffix). Encounter order is
/// preserved (files sorted by name, then scenario/test/impl declaration order).
/// Tolerates a missing directory and silently skips unparseable sidecars.
fn read_impl_refs(project_root: &Path) -> Vec<ImplRef> {
    let features_dir = project_root.join("spec").join("features");
    let Ok(entries) = std::fs::read_dir(&features_dir) else {
        return Vec::new();
    };

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        let is_cov = p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.ends_with(".feature.coverage"))
            .unwrap_or(false);
        if is_cov {
            paths.push(p);
        }
    }
    paths.sort();

    let mut refs: Vec<ImplRef> = Vec::new();
    for path in paths {
        let Ok(raw) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let feature_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.replace(".feature.coverage", ""))
            .unwrap_or_default();
        let Some(scenarios) = v.get("scenarios").and_then(Value::as_array) else {
            continue;
        };
        for scenario in scenarios {
            let Some(test_mappings) = scenario.get("testMappings").and_then(Value::as_array) else {
                continue;
            };
            for tm in test_mappings {
                let Some(impl_mappings) = tm.get("implMappings").and_then(Value::as_array) else {
                    continue;
                };
                for im in impl_mappings {
                    if let Some(fp) = im.get("file").and_then(Value::as_str) {
                        refs.push(ImplRef {
                            file_path: fp.to_string(),
                            feature_name: feature_name.clone(),
                        });
                    }
                }
            }
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_defaults() {
        let a: SearchImplementationArgs = serde_json::from_str("{}").unwrap();
        assert!(a.function.is_none());
        assert!(a.show_work_units.is_none());
        assert!(a.json.is_none());
    }

    #[test]
    fn args_parse_full() {
        let a: SearchImplementationArgs =
            serde_json::from_str(r#"{"function":"loadConfig","showWorkUnits":true,"json":true}"#)
                .unwrap();
        assert_eq!(a.function.as_deref(), Some("loadConfig"));
        assert_eq!(a.show_work_units, Some(true));
        assert_eq!(a.json, Some(true));
    }
}
