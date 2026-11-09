//! AUDIT-005: the workflow gates of `update-work-unit-status` call the audit
//! core `fspec audit` uses.
//!
//! Feature: spec/features/workflow-gates-call-the-audit-core.feature
//!
//! The work unit's features (its linked features, else the features tagged
//! with its ID) are audited over the coverage area on the way to
//! implementing or validating, and over the coverage and spec areas on the
//! way to done. Any problem blocks the move; the error lists the problems in
//! `fspec audit`'s line format with their fix commands. The one rule the
//! gate adds is the workflow's: before validating, every test mapping of a
//! live scenario has an implementation mapping, checked on the coverage the
//! audit read.

use std::path::Path;

use super::transitions::msg;
use crate::commands::audit::{work_unit_audit, Area, AuditedFeature, Problem};
use crate::error::FspecCoreError;

/// Audit work unit `id`'s features for the move to `new_status`; with
/// `require_impl`, also require an implementation mapping on every test
/// mapping of a live scenario.
pub(super) fn check_audit(
    project_root: &Path,
    id: &str,
    new_status: &str,
    areas: &[Area],
    require_impl: bool,
) -> Result<(), FspecCoreError> {
    let audit = work_unit_audit(project_root, id, areas)?;
    let mut problems = audit.problems;
    if require_impl {
        problems.extend(audit.features.iter().flat_map(missing_impl_mappings));
    }
    if problems.is_empty() {
        return Ok(());
    }
    let areas: Vec<String> = areas
        .iter()
        .map(|area| format!("--area {}", area.name()))
        .collect();
    let lines: Vec<String> = problems.iter().map(Problem::line).collect();
    Err(msg(format!(
        "Cannot move {id} to {new_status}: {} problem(s) in its features.\n\n{}\n\n\
Repair each problem with its command, then check again with: fspec audit --work-unit {id} {}",
        problems.len(),
        lines.join("\n"),
        areas.join(" ")
    )))
}

/// The test mappings of `feature`'s live scenarios that have no
/// implementation mapping.
fn missing_impl_mappings(feature: &AuditedFeature) -> Vec<Problem> {
    let Some(coverage) = &feature.coverage else {
        return Vec::new(); // the coverage area reports the coverage file
    };
    let name = feature.name.as_str();
    feature
        .live_scenarios
        .iter()
        .filter_map(|scenario| {
            coverage
                .scenarios
                .iter()
                .find(|entry| &entry.name == scenario)
        })
        .flat_map(|entry| {
            entry
                .test_mappings
                .iter()
                .filter(|mapping| mapping.impl_mappings.is_empty())
                .map(move |mapping| {
                    Problem::new(
                        Area::Coverage,
                        "no-impl-mapping",
                        name,
                        "the test mapping has no implementation mapping, which validating requires"
                            .to_string(),
                    )
                    .scenario(&entry.name)
                    .at(&format!("{}:{}", mapping.file, mapping.lines))
                    .fix(format!(
                        "fspec link-coverage {name} --scenario \"{}\" --test-file {} --impl-file <file> --impl-lines <lines>",
                        entry.name, mapping.file
                    ))
                })
        })
        .collect()
}
