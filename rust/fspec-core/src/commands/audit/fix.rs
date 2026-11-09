//! `fspec audit --fix`: the exact mechanical repairs (COV-079, COV-078).
//!
//! - COV-079: shift an implementation mapping whose code moved, or record
//!   its line text;
//! - COV-078: relink a scenario to its exact @step block found once (BUG-502
//!   replace, keeping implementation mappings), remove a coverage entry whose
//!   scenario is not in the feature, and delete a coverage file without its
//!   feature;
//! - BUG-608: rewrite a stats block that does not match the scenarios.
//!
//! Each repair was decided by detection from exact facts; `--fix` only
//! writes it, inside the coverage sidecar transaction, and prints what it
//! changed. A repair whose mapping changed in the meantime is skipped.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Value};

use super::impl_ranges::{compact, ImplRepair};
use super::Problem;
use crate::commands::audit::drift::line_ranges;
use crate::commands::coverage_lineage::SidecarTransaction;
use crate::commands::coverage_prune::remove_scenarios;
use crate::commands::link_coverage::relink_test_mapping;
use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;

/// One mechanical repair.
#[derive(Debug, Clone)]
pub(crate) enum Repair {
    /// COV-079: shift an implementation mapping or record its line text.
    Impl {
        scenario: String,
        test_file: String,
        test_lines: String,
        impl_file: String,
        lines: Vec<i64>,
        change: ImplRepair,
    },
    /// COV-078: the scenario's mappings of `old_file` (detected with
    /// `old_lines`) become `new_file:new_lines`.
    Test {
        scenario: String,
        old_file: String,
        old_lines: Vec<String>,
        new_file: String,
        new_lines: String,
    },
    /// COV-078: remove the coverage entries of a scenario the feature does
    /// not have.
    StaleEntry { scenario: String },
    /// COV-078: delete the coverage file of a feature that does not exist.
    DeleteCoverage,
    /// BUG-608: rewrite the stats block from the scenarios array.
    Stats,
}

/// Apply the repairs of `problems`, one coverage file at a time, and return
/// one line per change.
pub(super) fn apply(
    project_root: &Path,
    problems: &[Problem],
) -> Result<Vec<String>, FspecCoreError> {
    let mut by_feature: BTreeMap<&str, Vec<(&Problem, &Repair)>> = BTreeMap::new();
    for problem in problems {
        if let Some(repair) = &problem.repair {
            by_feature
                .entry(problem.feature.as_str())
                .or_default()
                .push((problem, repair));
        }
    }
    let mut changes = Vec::new();
    for (feature, mut repairs) in by_feature {
        // Implementation repairs find their test mapping by its current
        // lines, so they run before a relink changes those lines.
        repairs.sort_by_key(|(_, repair)| !matches!(repair, Repair::Impl { .. }));
        let path = crate::io::spec_paths::features_dir_of(project_root)
            .join(format!("{feature}.feature.coverage"));
        let _transaction = SidecarTransaction::acquire(&path, "audit")?;
        if let Some((problem, _)) = repairs
            .iter()
            .find(|(_, repair)| matches!(repair, Repair::DeleteCoverage))
        {
            // Re-checked under the lock: the feature file is still missing.
            let feature_path = path.with_extension("");
            if path.exists() && !feature_path.exists() {
                std::fs::remove_file(&path).map_err(|source| FspecCoreError::Io {
                    command: "audit",
                    source,
                })?;
                changes.push(format!(
                    "fixed {} {} {feature}: deleted spec/features/{feature}.feature.coverage",
                    problem.area.name(),
                    problem.code
                ));
            }
            continue;
        }
        let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
            command: "audit",
            source,
        })?;
        let mut coverage: Value =
            serde_json::from_str(&raw).map_err(|error| FspecCoreError::InvalidArgs {
                command: "audit",
                reason: format!("cannot parse {}: {error}", path.display()),
            })?;
        let before = changes.len();
        for (problem, repair) in repairs {
            if let Some(change) = apply_one(&mut coverage, repair) {
                changes.push(format!(
                    "fixed {} {} {} {change}",
                    problem.area.name(),
                    problem.code,
                    feature
                ));
            }
        }
        if changes.len() > before {
            crate::commands::link_coverage::stats::update_stats(&mut coverage);
            write_json_atomic(&path, &coverage).map_err(|error| FspecCoreError::InvalidArgs {
                command: "audit",
                reason: format!("cannot write {}: {error}", path.display()),
            })?;
        }
    }
    Ok(changes)
}

/// Apply `repair` to the parsed coverage file; the change description, or
/// `None` when the mapping is no longer there as detected.
fn apply_one(coverage: &mut Value, repair: &Repair) -> Option<String> {
    match repair {
        Repair::Impl { .. } => apply_impl(coverage, repair),
        Repair::Test {
            scenario,
            old_file,
            old_lines,
            new_file,
            new_lines,
        } => {
            let entry = scenario_entry(coverage, scenario)?;
            let current: Vec<&str> = entry
                .get("testMappings")?
                .as_array()?
                .iter()
                .filter(|row| row.get("file").and_then(Value::as_str) == Some(old_file))
                .filter_map(|row| row.get("lines").and_then(Value::as_str))
                .collect();
            if current != *old_lines {
                return None;
            }
            relink_test_mapping(entry, old_file, new_file, new_lines)?;
            Some(format!(
                "\"{scenario}\" {old_file}:{} -> {new_file}:{new_lines}",
                old_lines.join(",")
            ))
        }
        Repair::StaleEntry { scenario } => {
            let removed = remove_scenarios(coverage, std::slice::from_ref(scenario))?;
            (removed > 0).then(|| format!("\"{scenario}\": removed the coverage entry"))
        }
        Repair::DeleteCoverage => None,
        // BUG-608: `apply` rewrites the stats of every changed file; this
        // repair is a change only while the stats still differ.
        Repair::Stats => {
            let computed = crate::commands::link_coverage::stats::computed_stats(coverage);
            let stored = coverage.get("stats").map(Value::as_object);
            let differs = stored.is_some_and(|stored| {
                let empty = serde_json::Map::new();
                let stored = stored.unwrap_or(&empty);
                computed
                    .iter()
                    .any(|(field, value)| stored.get(field) != Some(value))
            });
            differs.then(|| "rewrote the stats block from the scenarios".to_string())
        }
    }
}

/// The first coverage entry named `scenario`.
fn scenario_entry<'a>(coverage: &'a mut Value, scenario: &str) -> Option<&'a mut Value> {
    coverage
        .get_mut("scenarios")?
        .as_array_mut()?
        .iter_mut()
        .find(|entry| entry.get("name").and_then(Value::as_str) == Some(scenario))
}

/// COV-079: shift an implementation mapping or record its line text.
fn apply_impl(coverage: &mut Value, repair: &Repair) -> Option<String> {
    let Repair::Impl {
        scenario,
        test_file,
        test_lines,
        impl_file,
        lines,
        change,
    } = repair
    else {
        return None;
    };
    let mapping = scenario_entry(coverage, scenario)?
        .get_mut("testMappings")?
        .as_array_mut()?
        .iter_mut()
        .find(|row| {
            row.get("file").and_then(Value::as_str) == Some(test_file)
                && row.get("lines").and_then(Value::as_str) == Some(test_lines)
        })?
        .get_mut("implMappings")?
        .as_array_mut()?
        .iter_mut()
        .find(|implementation| {
            implementation.get("file").and_then(Value::as_str) == Some(impl_file)
                && line_list(implementation.get("lines")).as_deref() == Some(lines.as_slice())
        })?;
    let old = format!("{impl_file}:{}", compact(lines));
    match change {
        ImplRepair::Shift { new_lines } => {
            mapping["lines"] = json!(new_lines);
            Some(format!(
                "\"{scenario}\" {old} -> {impl_file}:{}",
                compact(new_lines)
            ))
        }
        ImplRepair::Record {
            first_line,
            last_line,
        } => {
            mapping["firstLine"] = json!(first_line);
            mapping["lastLine"] = json!(last_line);
            Some(format!(
                "\"{scenario}\" {old}: recorded its first and last line text"
            ))
        }
    }
}

/// A stored implementation line list (`[1, 2]` or `"1-2"`).
fn line_list(lines: Option<&Value>) -> Option<Vec<i64>> {
    match lines? {
        Value::Array(lines) => lines.iter().map(Value::as_i64).collect(),
        Value::String(range) => line_ranges(range).map(|ranges| {
            ranges
                .into_iter()
                .flat_map(|(start, end)| start..=end)
                .filter_map(|line| i64::try_from(line).ok())
                .collect()
        }),
        _ => None,
    }
}
