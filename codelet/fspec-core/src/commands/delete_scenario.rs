//! `delete-scenario` — Rust port of `src/commands/delete-scenario.ts` (RPC-219).
//!
//! Deletes a named scenario from a Gherkin feature file using a LINE-BASED
//! removal: the lenient parser locates the scenario's 1-based start line
//! (`Scenario.position.line`) and the last step's line, the removal span is
//! extended forward over trailing blank lines (stopping at the next
//! structural header), the span is sliced out, consecutive blank lines are
//! collapsed to at most two, and the result is re-parsed to guarantee it is
//! still valid Gherkin before the file is written.
//!
//! When a sibling `<file>.coverage` sidecar exists, the deleted scenario is
//! removed from `coverage.scenarios` and the `totalScenarios /
//! coveredScenarios / coveragePercent` stats are recomputed (Math.round
//! half-up) while every other field is preserved (serde flatten `extra`).
//! A malformed / unreadable sidecar is ignored — deletion still succeeds.
//!
//! ## Recoverable-error contract
//! Mirroring the TS `DeleteScenarioResult { success, message?, error? }`
//! shape: missing files, missing scenarios, invalid Gherkin, and a
//! post-deletion re-parse failure all surface as
//! [`FspecCoreError::InvalidArgs`] so the dispatcher reports
//! `success=false` and the CLI bridge prints `Error: <reason>` + exit 1.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/delete_scenario.rs` is JSON marshalling only.

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

    // ---- Resolve feature path (TS parity, src/commands/delete-scenario.ts:27-34) ----
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

    let scenario_start_line = scenario.position.line;
    let scenario_end_line = scenario
        .steps
        .last()
        .map(|s| s.position.line)
        .unwrap_or(scenario_start_line);

    // ---- Compute removal span (TS parity, lines 97-130) ----
    let lines: Vec<&str> = content.split('\n').collect();

    // Find the actual end of the scenario block (including trailing blanks),
    // stopping at the next structural header.
    let mut actual_end_line = scenario_end_line;
    // TS loops `for (let i = scenarioEndLine; i < lines.length; i++)` over
    // a 0-indexed array where `scenarioEndLine` is a 1-based line number, so
    // the first inspected element is the line AFTER the last step.
    let mut i = scenario_end_line; // 0-based index == 1-based (last step) line
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.starts_with("Scenario:")
            || trimmed.starts_with("Scenario Outline:")
            || trimmed.starts_with("Background:")
            || trimmed.starts_with("Feature:")
            || trimmed.starts_with("Examples:")
        {
            break;
        }
        if trimmed.is_empty() {
            actual_end_line = i;
        } else if i > scenario_end_line {
            break;
        }
        i += 1;
    }

    // Convert to 0-indexed slice bounds.
    let start_index = (scenario_start_line - 1) as usize;
    let end_index = actual_end_line; // remove through this line inclusive (1-based → 0-based+1 below)

    let mut new_lines: Vec<&str> = Vec::with_capacity(lines.len());
    new_lines.extend_from_slice(&lines[..start_index]);
    // `end_index` is the 1-based last line to remove (inclusive); the TS
    // `lines.slice(endIndex + 1)` keeps from `endIndex + 1` onward, which in
    // 0-based terms is the same `end_index` value because `actual_end_line`
    // already counts as a 1-based line number / 0-based index of the blank.
    if end_index + 1 < lines.len() {
        new_lines.extend_from_slice(&lines[end_index + 1..]);
    }

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

    // ---- Coverage sidecar update (best-effort) ----
    let coverage_path = feature_path.with_file_name(format!(
        "{}.coverage",
        feature_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    ));

    let message = match update_coverage(&coverage_path, &args.scenario) {
        Some(()) => format!(
            "Successfully deleted scenario '{}' from {}\n  Updated coverage file",
            args.scenario, file_name
        ),
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

/// Resolve the feature path exactly like the TS reference
/// (src/commands/delete-scenario.ts:27-34).
fn resolve_feature_path(project_root: &Path, feature: &str) -> std::path::PathBuf {
    if feature.ends_with(".feature") || feature.starts_with("spec/features/") {
        project_root.join(feature)
    } else {
        project_root
            .join("spec/features")
            .join(format!("{feature}.feature"))
    }
}

/// Collapse runs of more than two consecutive blank lines down to two.
/// Mirrors TS `trimmedLines` accumulation (lines 132-145).
fn collapse_blank_lines<'a>(lines: &[&'a str]) -> Vec<&'a str> {
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

/// Best-effort coverage sidecar update. Returns `Some(())` when the sidecar
/// existed and was successfully updated, `None` when it was absent or
/// malformed (parity with the TS try/catch that swallows any error and
/// returns the plain success message).
///
/// Operates directly on a `serde_json::Value` (instead of a typed struct) to
/// mirror the TS `JSON.parse` → mutate → `JSON.stringify` round-trip exactly:
/// arbitrary top-level / scenario / stats fields are preserved in their
/// original key order, `coveredScenarios` is counted purely by
/// `testMappings.length > 0` (regardless of mapping shape), and the three
/// recomputed stats keys keep their original position via object-spread
/// override semantics (or are appended when absent).
fn update_coverage(coverage_path: &Path, scenario: &str) -> Option<()> {
    use serde_json::Value;

    let body = std::fs::read_to_string(coverage_path).ok()?;
    let mut coverage: Value = serde_json::from_str(&body).ok()?;

    // TS reads `coverage.scenarios` unconditionally — a missing/non-array
    // `scenarios` field throws in TS (`.length` / `.filter` on undefined),
    // landing in the catch branch with the plain success message and the
    // file left untouched. Bail out the same way here.
    let scenarios = coverage.get("scenarios")?.as_array()?.clone();

    // Filter out the deleted scenario by exact name match.
    let remaining: Vec<Value> = scenarios
        .into_iter()
        .filter(|s| s.get("name").and_then(Value::as_str) != Some(scenario))
        .collect();

    // coveredScenarios = scenarios with a non-empty testMappings array.
    let covered_scenarios = remaining
        .iter()
        .filter(|s| {
            s.get("testMappings")
                .and_then(Value::as_array)
                .is_some_and(|m| !m.is_empty())
        })
        .count();
    let total_scenarios = remaining.len();

    let coverage_percent: i64 = if total_scenarios > 0 {
        // Math.round semantics (half-up for non-negative inputs).
        ((covered_scenarios as f64) / (total_scenarios as f64) * 100.0).round() as i64
    } else {
        0
    };

    // Build the new stats object: `{ ...existingStats, totalScenarios,
    // coveredScenarios, coveragePercent }`. Object-spread override keeps the
    // ORIGINAL key position when a key already exists, otherwise appends.
    let mut stats = match coverage.get("stats") {
        Some(Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    stats.insert("totalScenarios".to_string(), json!(total_scenarios));
    stats.insert("coveredScenarios".to_string(), json!(covered_scenarios));
    stats.insert("coveragePercent".to_string(), json!(coverage_percent));

    if let Value::Object(map) = &mut coverage {
        map.insert("scenarios".to_string(), Value::Array(remaining));
        map.insert("stats".to_string(), Value::Object(stats));
    } else {
        // Top-level was not an object (TS would have thrown on `.scenarios`).
        return None;
    }

    let serialised = serde_json::to_string_pretty(&coverage).ok()?;
    std::fs::write(coverage_path, serialised).ok()?;
    Some(())
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
