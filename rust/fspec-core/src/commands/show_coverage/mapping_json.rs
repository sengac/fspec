//! Explicit machine readback (`mappingJson`) for indexed coverage mutations.
//!
//! Exposes every raw `testMappings` row with its ordered index and the
//! canonical whole-scenario hash consumed by indexed link/unlink. Read-only:
//! it never takes the sidecar transaction lock, so it creates no files, and it
//! leaves the legacy `format: json` output untouched.

use serde_json::{json, Value};

use super::super::coverage_lineage::{scenario_hash, sidecar_paths};
use super::*;

pub(super) fn show_mapping_json(
    project_root: &Path,
    feature_name: Option<&str>,
) -> Result<String, FspecCoreError> {
    let invalid = |reason: String| FspecCoreError::InvalidArgs {
        command: "show-coverage",
        reason,
    };
    let feature_name = feature_name
        .filter(|name| !name.is_empty())
        .ok_or_else(|| invalid("--mapping-json requires a feature name".to_string()))?;
    let paths = sidecar_paths(project_root, feature_name);
    let raw = std::fs::read_to_string(&paths.coverage_path).map_err(|e| {
        invalid(format!(
            "Coverage file not found: {}.coverage ({e})",
            paths.file_name
        ))
    })?;
    let coverage: Value = serde_json::from_str(&raw).map_err(|e| {
        invalid(format!(
            "Invalid coverage file {}.coverage: {e}",
            paths.file_name
        ))
    })?;
    let scenarios = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Coverage file has no scenarios array".to_string()))?;
    let scenarios = scenarios
        .iter()
        .map(|scenario| {
            let rows = scenario
                .get("testMappings")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .enumerate()
                        .map(|(index, mapping)| {
                            json!({"testMappingIndex": index, "mapping": mapping})
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "name": scenario.get("name").cloned().unwrap_or(Value::Null),
                "scenarioHash": scenario_hash(scenario),
                "testMappings": rows,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&json!({
        "fileName": paths.file_name,
        "scenarios": scenarios,
    }))
    .map_err(|e| invalid(format!("failed to serialize mapping readback: {e}")))
}
