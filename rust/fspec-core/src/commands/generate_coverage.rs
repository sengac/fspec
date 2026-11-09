//! `generate-coverage` — Rust port of `src/commands/generate-coverage.ts`
//! (RPC-231).
//!
//! Scans `spec/features/*.feature`; for each, resolves the
//! `<feature>.feature.coverage` sidecar and computes one of four statuses
//! mirroring `src/utils/coverage-file.ts` `createCoverageFile`:
//!
//!   * `created`   — no sidecar existed; write a fresh one.
//!   * `recreated` — sidecar existed but contained invalid JSON.
//!   * `updated`   — sidecar existed (valid JSON) but its scenario set drifted
//!     from the feature file (new scenarios added / stale ones dropped);
//!     preserve existing test mappings + unknown fields.
//!   * `skipped`   — sidecar already in sync; leave byte-for-byte unchanged.
//!
//! Scenario names come from [`crate::io::gherkin::parse_feature_lenient`] over
//! the top-level `feature.scenarios`. Created/recreated bodies are written via
//! [`crate::io::locked_file::write_json_atomic`] (2-space JSON, no trailing
//! newline). Updated sidecars are mutated on a `serde_json::Value` so unknown
//! top-level / stats fields survive the round-trip (parity with
//! `delete_scenario::update_coverage`).
//!
//! Output (non-dry-run): a `✓ `-prefixed line joining the nonzero parts
//! `Created N, Updated N, Skipped N, Recreated N (invalid JSON)` (or
//! `No coverage files needed`), ALWAYS followed verbatim by the long
//! link-coverage `<system-reminder>` block. Dry-run: a `Would create N
//! coverage files (DRY RUN)` line + file list + `Would skip/recreate` lines,
//! never reporting updates and never writing.
//!
//! Two-front-doors: the dispatcher and the clap CLI both call [`run`]. The full
//! stdout string is rendered HERE and returned; the CLI bridge prints it
//! verbatim.
//!
//! REFAC-020: global writes now hold the shared sidecar transaction for every
//! sidecar they create or update (re-reading inside it), and `featureName`
//! selects exactly one feature for lossless JSON-planned generation (see
//! [`selected`]); it never falls back to the global scan.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;
use crate::io::locked_file::write_json_atomic;

use super::coverage_lineage::SidecarTransaction;

mod global;
mod plan;
mod report;
mod selected;

use global::process_feature;
#[cfg(test)]
use report::{render_dry_run, render_success};

/// CLI / dispatcher arguments accepted by `generate-coverage`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GenerateCoverageArgs {
    #[serde(default)]
    dry_run: bool,
    /// REFAC-020: generate only this exact feature (JSON plan output).
    #[serde(default)]
    feature_name: Option<String>,
}

fn invalid(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "generate-coverage",
        reason,
    }
}

/// One feature file's processing outcome.
/// BUG-301: what a global update changes in one sidecar.
#[derive(Debug, Default)]
struct Change {
    added: Vec<String>,
    removed: Vec<String>,
    /// Stale scenarios kept because they still carry test mappings.
    blocked: Vec<String>,
}

enum Status {
    Created,
    Recreated,
    Updated(Change),
    Skipped { blocked: Vec<String> },
}

/// Dispatcher / CLI entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GenerateCoverageArgs = serde_json::from_str(args_json)
        .map_err(|e| invalid(format!("failed to parse args: {e}")))?;

    if let Some(feature_name) = args.feature_name.as_deref() {
        return selected::run_selected(project_root, feature_name, args.dry_run);
    }

    let features_dir = crate::io::spec_paths::features_dir_of(project_root);

    // TS: `readdir(featuresDir)` — any failure is surfaced as
    // "Failed to read features directory: <msg>".
    let entries = std::fs::read_dir(&features_dir)
        .map_err(|e| invalid(format!("Failed to read features directory: {e}")))?;

    // Collect `*.feature` filenames (not `*.feature.coverage`), sorted for
    // deterministic ordering.
    let mut feature_files: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".feature") {
            feature_files.push(name);
        }
    }
    feature_files.sort();

    let mut created = 0usize;
    let mut skipped = 0usize;
    let mut recreated = 0usize;
    let mut create_list: Vec<String> = Vec::new();
    let mut updates: Vec<(String, Change)> = Vec::new();
    let mut blocked: Vec<(String, Vec<String>)> = Vec::new();

    for feature_file in &feature_files {
        let feature_path = features_dir.join(feature_file);
        let coverage_name = format!("{feature_file}.coverage");
        let coverage_path = features_dir.join(&coverage_name);

        // BUG-301: dry-run and apply share one plan, so the preview reports
        // exactly what a real run writes.
        let planned = process_feature(&feature_path, &coverage_path, false)?;
        let status = match planned {
            Status::Skipped { .. } => planned,
            _ if args.dry_run => planned,
            _ => {
                let _transaction =
                    SidecarTransaction::acquire(&coverage_path, "generate-coverage")?;
                process_feature(&feature_path, &coverage_path, true)?
            }
        };
        match status {
            Status::Created => {
                created += 1;
                create_list.push(coverage_name);
            }
            Status::Recreated => {
                recreated += 1;
                create_list.push(coverage_name);
            }
            Status::Updated(change) => {
                if !change.blocked.is_empty() {
                    blocked.push((coverage_name.clone(), change.blocked.clone()));
                }
                updates.push((coverage_name, change));
            }
            Status::Skipped { blocked: names } => {
                skipped += 1;
                if !names.is_empty() {
                    blocked.push((coverage_name, names));
                }
            }
        }
    }

    let rendered = if args.dry_run {
        report::render_dry_run(
            created,
            skipped,
            recreated,
            &create_list,
            &updates,
            &blocked,
        )
    } else {
        report::render_success(created, updates.len(), skipped, recreated, &blocked)
    };

    Ok(rendered)
}

#[cfg(test)]
mod tests;
