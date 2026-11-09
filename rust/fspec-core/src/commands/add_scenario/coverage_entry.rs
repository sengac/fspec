//! BUG-346: keep a feature's existing coverage file in step with
//! `add-scenario`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::commands::link_coverage::stats::update_stats;
use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

/// Append an empty entry for `scenario` to `<feature>.coverage` and
/// recompute its stats. A feature without a coverage file is left alone
/// (generate-coverage creates one); an unreadable or malformed coverage file
/// is an error naming it.
pub(super) fn add_coverage_entry(feature: &Path, scenario: &str) -> Result<(), FspecCoreError> {
    let path = PathBuf::from(format!("{}.coverage", feature.display()));
    // BUG-477: the coverage read-modify-write holds the sidecar transaction.
    let _transaction = if path.exists() {
        Some(crate::commands::coverage_lineage::SidecarTransaction::acquire(&path, "add-scenario")?)
    } else {
        None
    };
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "add-scenario",
                source,
            })
        }
    };
    let invalid = |reason: String| FspecCoreError::InvalidArgs {
        command: "add-scenario",
        reason,
    };
    let mut coverage: Value = serde_json::from_str(&raw)
        .map_err(|error| invalid(format!("cannot parse {}: {error}", path.display())))?;
    let scenarios = coverage
        .as_object_mut()
        .ok_or_else(|| invalid(format!("{} is not a JSON object", path.display())))?
        .entry("scenarios")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| invalid(format!("{} scenarios is not an array", path.display())))?;
    if !scenarios.iter().any(|entry| entry["name"] == scenario) {
        scenarios.push(json!({ "name": scenario, "testMappings": [] }));
    }
    update_stats(&mut coverage);
    write_json_atomic(&path, &coverage)
}
