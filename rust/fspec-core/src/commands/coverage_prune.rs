//! BUG-444: prune deleted scenarios from a `.feature.coverage` sidecar.
//!
//! Shared by `delete-scenario` and `delete-scenarios`. The pruned coverage is
//! prepared before the feature file is written, so a broken coverage file
//! fails the command with nothing written.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::commands::link_coverage::stats::update_stats;
use crate::error::FspecCoreError;

/// The coverage sidecar of `feature` (`<feature>.coverage`).
pub(crate) fn coverage_path(feature: &Path) -> PathBuf {
    PathBuf::from(format!("{}.coverage", feature.display()))
}

/// The coverage at `path` without the `deleted` scenarios, stats recomputed.
/// `None` when the feature has no coverage file; an unreadable file, invalid
/// JSON or a missing or non-array `scenarios` is an error naming the file.
pub(crate) fn prune_coverage(
    path: &Path,
    deleted: &[String],
    command: &'static str,
) -> Result<Option<Value>, FspecCoreError> {
    let invalid = |reason: String| FspecCoreError::InvalidArgs { command, reason };
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(invalid(format!("cannot read {}: {error}", path.display()))),
    };
    let mut coverage: Value = serde_json::from_str(&raw)
        .map_err(|error| invalid(format!("cannot parse {}: {error}", path.display())))?;
    remove_scenarios(&mut coverage, deleted)
        .ok_or_else(|| invalid(format!("{} has no scenarios array", path.display())))?;
    update_stats(&mut coverage);
    Ok(Some(coverage))
}

/// Remove the entries of the `deleted` scenarios from parsed coverage; the
/// number removed, or `None` without a `scenarios` array. COV-078: `fspec
/// audit --fix` removes stale entries with it too.
pub(crate) fn remove_scenarios(coverage: &mut Value, deleted: &[String]) -> Option<usize> {
    let scenarios = coverage
        .get_mut("scenarios")
        .and_then(Value::as_array_mut)?;
    let before = scenarios.len();
    scenarios.retain(|scenario| {
        !scenario
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| deleted.iter().any(|d| d == name))
    });
    Some(before - scenarios.len())
}

/// Write pruned coverage back (2-space JSON, as before).
pub(crate) fn write_coverage(
    path: &Path,
    coverage: &Value,
    command: &'static str,
) -> Result<(), FspecCoreError> {
    let body =
        serde_json::to_string_pretty(coverage).map_err(|error| FspecCoreError::InvalidArgs {
            command,
            reason: format!("cannot serialize {}: {error}", path.display()),
        })?;
    // BUG-477: replaced atomically under the spec lock.
    crate::io::locked_file::write_bytes_atomic(path, body.as_bytes()).map_err(|error| {
        FspecCoreError::InvalidArgs {
            command,
            reason: format!("cannot write {}: {error}", path.display()),
        }
    })
}
