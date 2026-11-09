//! `delete-scenarios` — Rust port of `src/commands/delete-scenarios-by-tag.ts`
//! (RPC-220; the registered command name is `delete-scenarios`).
//!
//! Bulk-deletes scenarios whose SCENARIO-level tags match ALL of the supplied
//! tags (AND logic) across every `spec/features/**/*.feature` file. The
//! recursive walk reuses [`crate::io::feature_glob::glob_feature_files`]; a
//! missing `spec/features/` directory maps to an empty list so the canonical
//! `"No feature files found"` message is preserved.
//!
//! ## Scenario blocks
//! BUG-645: each matching scenario's block is the one delete-scenario removes
//! ([`crate::commands::delete_scenario::scenario_block`], BUG-486): from the
//! tag and comment lines directly above its header to just before the next
//! scenario-level header (Scenario, Scenario Outline, Example, Background,
//! Rule) and that header's own tag and comment lines. `lineStart` is the
//! block's first line and `lineEnd` the first line of the next block, or the
//! line count when the block runs to the end of the file (1-based).
//!
//! ## Deletion
//! For each file, matching scenarios are removed bottom-up (descending header
//! line) with delete-scenario's block removal, so deleting the last scenarios
//! leaves the previous content followed by one newline and a following Rule
//! is kept. Runs of more than two blank lines are collapsed to two. The
//! result is re-parsed; a parse failure aborts with an error envelope. The
//! coverage sidecar drops the deleted scenarios (BUG-444), stats recomputed.
//!
//! ## Result envelope
//! `{success, deletedCount, fileCount, message?, scenarios?, error?}`. The
//! empty-tag rejection returns `success:false` + `error` (parity with
//! delete-features); the dispatcher derives failure from the inner payload.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11).

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::commands::delete_scenario::{
    collapse_blank_lines, scenario_block, without_scenario_block,
};
use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct DeleteScenariosArgs {
    tags: Vec<String>,
    dry_run: bool,
}

/// One matching scenario's metadata, mirroring the TS `ScenarioInfo`.
#[derive(Debug, Clone)]
struct ScenarioInfo {
    file: String,
    name: String,
    tags: Vec<String>,
    /// The 0-based header line.
    header: usize,
    line_start: usize,
    line_end: usize,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteScenariosArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-scenarios",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Require at least one tag (TS CLI rejects empty; dispatcher parity
    // with delete-features: inner success=false + error) ----
    if args.tags.is_empty() {
        return ok(json!({
            "success": false,
            "deletedCount": 0,
            "fileCount": 0,
            "error": "At least one --tag is required",
        }));
    }

    // ---- Enumerate feature files (DirectoryNotFound → empty list) ----
    let files = match glob_feature_files(project_root) {
        Ok(f) => f,
        Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
        Err(other) => return Err(other),
    };

    if files.is_empty() {
        return ok(json!({
            "success": true,
            "deletedCount": 0,
            "fileCount": 0,
            "message": "No feature files found",
        }));
    }

    // ---- Find scenarios matching ALL tags (AND logic), grouped by file ----
    // Preserve file iteration order (glob_feature_files is sorted).
    let mut matching: Vec<(String, Vec<ScenarioInfo>)> = Vec::new();

    for file in &files {
        let abs = project_root.join(file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let feature = match parse_feature_lenient(&content) {
            Ok(f) => f,
            Err(_) => continue, // skip invalid Gherkin
        };

        let file_scenarios = collect_matching_scenarios(&content, &feature, file, &args.tags);
        if !file_scenarios.is_empty() {
            matching.push((file.clone(), file_scenarios));
        }
    }

    // ---- Count total scenarios to delete ----
    let total_scenarios: usize = matching.iter().map(|(_, s)| s.len()).sum();

    if total_scenarios == 0 {
        return ok(json!({
            "success": true,
            "deletedCount": 0,
            "fileCount": 0,
            "message": "No scenarios found matching tags",
        }));
    }

    // ---- Dry-run: report without modifying ----
    if args.dry_run {
        let all: Vec<Value> = matching
            .iter()
            .flat_map(|(_, scenarios)| scenarios.iter().map(scenario_info_to_value))
            .collect();
        let file_count = matching.len();
        return ok(json!({
            "success": true,
            "deletedCount": total_scenarios,
            "fileCount": file_count,
            "message": format!(
                "Would delete {total_scenarios} scenario(s) from {file_count} file(s)"
            ),
            "scenarios": all,
        }));
    }

    // ---- Perform deletions ----
    let mut files_modified: usize = 0;

    for (file, scenarios) in &matching {
        let abs = project_root.join(file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut lines: Vec<&str> = content.split('\n').collect();

        // BUG-645: bottom-up, so the header lines above stay valid.
        let mut headers: Vec<usize> = scenarios.iter().map(|s| s.header).collect();
        headers.sort_unstable_by(|a, b| b.cmp(a));
        for header in headers {
            lines = without_scenario_block(&lines, header);
        }
        let new_content = collapse_blank_lines(&lines).join("\n");

        // ---- Validate result re-parses ----
        if let Err(e) = parse_feature_lenient(&new_content) {
            return ok(json!({
                "success": false,
                "deletedCount": 0,
                "fileCount": 0,
                "error": format!(
                    "Validation failed after deleting scenarios from {file}: {e}"
                ),
            }));
        }

        // BUG-444: prepare the coverage update before writing the feature.
        let coverage_path = crate::commands::coverage_prune::coverage_path(&abs);
        // BUG-477: the coverage prune and rewrite hold the sidecar transaction.
        let _transaction = if coverage_path.exists() {
            Some(
                crate::commands::coverage_lineage::SidecarTransaction::acquire(
                    &coverage_path,
                    "delete-scenarios",
                )?,
            )
        } else {
            None
        };
        let deleted_names: Vec<String> = scenarios.iter().map(|s| s.name.clone()).collect();
        let pruned = crate::commands::coverage_prune::prune_coverage(
            &coverage_path,
            &deleted_names,
            "delete-scenarios",
        )?;

        std::fs::write(&abs, &new_content).map_err(|source| FspecCoreError::Io {
            command: "delete-scenarios",
            source,
        })?;
        files_modified += 1;
        if let Some(coverage) = pruned {
            crate::commands::coverage_prune::write_coverage(
                &coverage_path,
                &coverage,
                "delete-scenarios",
            )?;
        }
    }

    ok(json!({
        "success": true,
        "deletedCount": total_scenarios,
        "fileCount": files_modified,
        "message": format!(
            "Deleted {total_scenarios} scenario(s) from {files_modified} file(s). All modified files validated successfully."
        ),
    }))
}

/// Collect scenarios in `feature` whose tags match ALL `wanted` tags, with
/// their blocks ([`scenario_block`]).
fn collect_matching_scenarios(
    content: &str,
    feature: &gherkin::Feature,
    file: &str,
    wanted: &[String],
) -> Vec<ScenarioInfo> {
    let lines: Vec<&str> = content.split('\n').collect();
    feature
        .scenarios
        .iter()
        .filter_map(|scenario| {
            let tags: Vec<String> = scenario.tags.iter().map(|t| format!("@{t}")).collect();
            if !wanted.iter().all(|t| tags.contains(t)) {
                return None;
            }
            let header = scenario.position.line.checked_sub(1)?;
            let (start, next) = scenario_block(&lines, header);
            Some(ScenarioInfo {
                file: file.to_string(),
                name: scenario.name.clone(),
                tags,
                header,
                line_start: start + 1,
                line_end: next.map_or(lines.len(), |next| next + 1),
            })
        })
        .collect()
}

fn scenario_info_to_value(s: &ScenarioInfo) -> Value {
    json!({
        "file": s.file,
        "name": s.name,
        "tags": s.tags,
        "lineStart": s.line_start,
        "lineEnd": s.line_end,
    })
}

/// Serialise an inner-envelope value to the `Ok(String)` returned by the
/// dispatcher entry point.
fn ok(value: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "delete-scenarios",
        reason: format!("failed to serialise response: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: DeleteScenariosArgs =
            serde_json::from_str(r#"{"tags":["@spike"],"dryRun":true}"#).unwrap();
        assert_eq!(a.tags, vec!["@spike".to_string()]);
        assert!(a.dry_run);
    }
}
