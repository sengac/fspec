//! `delete-scenario` — delete one named scenario from a feature file (RPC-219).
//!
//! Deletes a named scenario from a Gherkin feature file using a LINE-BASED
//! removal: the lenient parser locates the scenario's 1-based header line,
//! and the scenario's whole block is sliced out (BUG-486: from the tag and
//! comment lines directly above the header to just before the next
//! scenario-level header's own tag and comment lines), consecutive blank
//! lines are collapsed to at most two, and the result is re-parsed to
//! guarantee it is still valid Gherkin before the file is written.
//!
//! When a sibling `<file>.coverage` sidecar exists, the deleted scenario is
//! removed from `coverage.scenarios` and the `totalScenarios /
//! coveredScenarios / coveragePercent` stats are recomputed (rounded half
//! up) while every other field is preserved (serde flatten `extra`).
//! BUG-444: a malformed or unreadable sidecar fails the command, unwritten.
//!
//! ## Recoverable-error contract
//! The result is `{ success, message }` on success. Missing files,
//! missing scenarios, invalid Gherkin, and a
//! post-deletion re-parse failure all surface as
//! [`FspecCoreError::InvalidArgs`] so the dispatcher reports
//! `success=false` and the CLI bridge prints `Error: <reason>` + exit 1.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `rust/fspec/src/delete_scenario.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteScenarioArgs {
    feature: String,
    scenario: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteScenarioArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-scenario",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Resolve feature path: a path as given, a bare name under spec/features ----
    let feature_path = resolve_feature_path(project_root, &args.feature);

    // ---- Read feature file ----
    let content = match std::fs::read_to_string(&feature_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FspecCoreError::InvalidArgs {
                command: "delete-scenario",
                reason: format!("Feature file not found: {}", feature_path.display()),
            });
        }
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "delete-scenario",
                source,
            });
        }
    };

    // ---- Parse Gherkin ----
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(e) => {
            return Err(FspecCoreError::InvalidArgs {
                command: "delete-scenario",
                reason: format!("Invalid Gherkin syntax: {e}"),
            });
        }
    };

    // ---- Locate the scenario by exact name ----
    let scenario = feature
        .scenarios
        .iter()
        .find(|s| s.name == args.scenario)
        .ok_or(FspecCoreError::InvalidArgs {
            command: "delete-scenario",
            reason: format!("Scenario '{}' not found in feature file", args.scenario),
        })?;

    // ---- BUG-486: remove the scenario's whole block ----
    let lines: Vec<&str> = content.split('\n').collect();
    let new_lines = without_scenario_block(&lines, scenario.position.line - 1);

    // ---- Collapse runs of >2 blank lines ----
    let collapsed = collapse_blank_lines(&new_lines);
    let new_content = collapsed.join("\n");

    // ---- Validate result re-parses ----
    if let Err(e) = parse_feature_lenient(&new_content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-scenario",
            reason: format!("Deletion would result in invalid Gherkin: {e}"),
        });
    }

    // BUG-444: prepare the coverage update first; a broken coverage file
    // fails the command before anything is written.
    let coverage_path = crate::commands::coverage_prune::coverage_path(&feature_path);
    // BUG-477: the coverage prune and rewrite hold the sidecar transaction.
    let _transaction = if coverage_path.exists() {
        Some(
            crate::commands::coverage_lineage::SidecarTransaction::acquire(
                &coverage_path,
                "delete-scenario",
            )?,
        )
    } else {
        None
    };
    let pruned = crate::commands::coverage_prune::prune_coverage(
        &coverage_path,
        std::slice::from_ref(&args.scenario),
        "delete-scenario",
    )?;

    // ---- Write the updated feature file ----
    std::fs::write(&feature_path, &new_content).map_err(|source| FspecCoreError::Io {
        command: "delete-scenario",
        source,
    })?;

    let file_name = feature_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&args.feature)
        .to_string();

    let message = match pruned {
        Some(coverage) => {
            crate::commands::coverage_prune::write_coverage(
                &coverage_path,
                &coverage,
                "delete-scenario",
            )?;
            format!(
                "Successfully deleted scenario '{}' from {}\n  Updated coverage file",
                args.scenario, file_name
            )
        }
        None => format!(
            "Successfully deleted scenario '{}' from {}",
            args.scenario, file_name
        ),
    };

    let response = json!({
        "success": true,
        "message": message,
    });

    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "delete-scenario",
        reason: format!("failed to serialise response: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// BUG-486: the block of the scenario whose header is at 0-based `header`:
/// its first line (the first of the tag and comment lines directly above the
/// header) and the first line of the next block (the next scenario-level
/// header's own tag and comment lines), or `None` when the block runs to the
/// end of the file. It takes the scenario's description, steps with their
/// tables and doc strings, an outline's Examples and trailing comments.
/// BUG-645: delete-scenarios removes its blocks with it too.
pub(crate) fn scenario_block(lines: &[&str], header: usize) -> (usize, Option<usize>) {
    let next =
        next_scenario_header(lines, header + 1).map(|next| attached_header_start(lines, next));
    (attached_header_start(lines, header), next)
}

/// BUG-486: `lines` without the [`scenario_block`] of the scenario whose
/// header is at 0-based `header`.
pub(crate) fn without_scenario_block<'a>(lines: &[&'a str], header: usize) -> Vec<&'a str> {
    let (start, next) = scenario_block(lines, header);
    let mut kept: Vec<&'a str> = lines[..start].to_vec();
    match next {
        Some(next) => kept.extend_from_slice(&lines[next..]),
        None => {
            // The last scenario: end with the previous content and one newline.
            while kept.last().is_some_and(|line| line.trim().is_empty()) {
                kept.pop();
            }
            kept.push("");
        }
    }
    kept
}

/// The first of the tag and comment lines directly above line `header`.
fn attached_header_start(lines: &[&str], header: usize) -> usize {
    let mut start = header;
    while start > 0 {
        let above = lines[start - 1].trim_start();
        if above.starts_with('@') || above.starts_with('#') {
            start -= 1;
        } else {
            break;
        }
    }
    start
}

/// The next scenario-level header at or after `from`, skipping doc strings.
fn next_scenario_header(lines: &[&str], from: usize) -> Option<usize> {
    let mut doc_string: Option<&str> = None;
    for (index, line) in lines.iter().enumerate().skip(from) {
        let trimmed = line.trim_start();
        if let Some(fence) = doc_string {
            if trimmed.starts_with(fence) {
                doc_string = None;
            }
            continue;
        }
        if let Some(fence) = ["\"\"\"", "```"]
            .into_iter()
            .find(|fence| trimmed.starts_with(fence))
        {
            doc_string = Some(fence);
            continue;
        }
        let header = [
            "Scenario:",
            "Scenario Outline:",
            "Scenario Template:",
            "Example:",
            "Background:",
            "Rule:",
        ];
        if header.iter().any(|keyword| trimmed.starts_with(keyword)) {
            return Some(index);
        }
    }
    None
}

/// The feature path: a `.feature` path or one under `spec/features/` is
/// taken as given; a bare name is `<features dir>/<name>.feature`.
fn resolve_feature_path(project_root: &Path, feature: &str) -> std::path::PathBuf {
    if feature.ends_with(".feature") || feature.starts_with("spec/features/") {
        project_root.join(feature)
    } else {
        crate::io::spec_paths::features_dir_of(project_root).join(format!("{feature}.feature"))
    }
}

/// Collapse runs of more than two consecutive blank lines down to two.
/// A blank line resets nothing; any other line resets the count.
pub(crate) fn collapse_blank_lines<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    let mut out: Vec<&str> = Vec::with_capacity(lines.len());
    let mut consecutive_empty = 0usize;
    for &line in lines {
        if line.trim().is_empty() {
            consecutive_empty += 1;
            if consecutive_empty <= 2 {
                out.push(line);
            }
        } else {
            consecutive_empty = 0;
            out.push(line);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: DeleteScenarioArgs =
            serde_json::from_str(r#"{"feature":"spec/features/x.feature","scenario":"Old"}"#)
                .unwrap();
        assert_eq!(a.feature, "spec/features/x.feature");
        assert_eq!(a.scenario, "Old");
    }

    #[test]
    fn collapse_keeps_at_most_two_blanks() {
        let lines = ["a", "", "", "", "b"];
        let out = collapse_blank_lines(&lines);
        assert_eq!(out, vec!["a", "", "", "b"]);
    }
}
