//! Shared coverage-lineage primitives for the participating coverage writers
//! (REFAC-020).
//!
//! `link-coverage`, `unlink-coverage` and global/selected `generate-coverage`
//! hold the sidecar transaction (the project's spec write lock,
//! `spec/.fspec-write.lock`, BUG-551) across read → check → write.
//! Indexed mutations select one exact `testMappings` row with a paired
//! canonical whole-scenario SHA-256 that is re-verified inside that
//! transaction. Scenario/create/delete/report writers are nonparticipants.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::FspecCoreError;

mod canonical;
mod selector;
mod transaction;

pub(crate) use canonical::scenario_hash;
pub(crate) use selector::Selector;
pub(crate) use transaction::SidecarTransaction;

/// Legacy sidecar resolution shared by link/unlink/show: tolerate a trailing
/// `.feature` and join `spec/features/<name>.feature.coverage`.
pub(crate) struct SidecarPaths {
    pub file_name: String,
    pub feature_path: PathBuf,
    pub coverage_path: PathBuf,
}

pub(crate) fn sidecar_paths(project_root: &Path, feature_name: &str) -> SidecarPaths {
    let features_dir = crate::io::spec_paths::features_dir_of(project_root);
    let stripped = feature_name
        .strip_suffix(".feature")
        .unwrap_or(feature_name);
    let file_name = format!("{stripped}.feature");
    SidecarPaths {
        coverage_path: features_dir.join(format!("{file_name}.coverage")),
        feature_path: features_dir.join(&file_name),
        file_name,
    }
}

/// Locate exactly one raw coverage scenario by name. Indexed selection never
/// chooses between same-name scenarios.
pub(crate) fn unique_scenario_index(
    coverage: &Value,
    scenario: &str,
    command: &'static str,
) -> Result<usize, FspecCoreError> {
    let matches: Vec<usize> = coverage
        .get("scenarios")
        .and_then(Value::as_array)
        .map(|scenarios| {
            scenarios
                .iter()
                .enumerate()
                .filter(|(_, s)| s.get("name").and_then(Value::as_str) == Some(scenario))
                .map(|(index, _)| index)
                .collect()
        })
        .unwrap_or_default();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(invalid(
            command,
            format!("Scenario not found: \"{scenario}\""),
        )),
        _ => Err(invalid(
            command,
            format!(
                "Ambiguous scenario \"{scenario}\": {} coverage scenarios share this name; indexed selection refuses to choose",
                matches.len()
            ),
        )),
    }
}

pub(crate) fn invalid(command: &'static str, reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs { command, reason }
}
