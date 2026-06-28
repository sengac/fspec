//! `delete-scenarios` — Rust port of `src/commands/delete-scenarios-by-tag.ts`
//! (RPC-220; the registered command name is `delete-scenarios`).
//!
//! Bulk-deletes scenarios whose SCENARIO-level tags match ALL of the supplied
//! tags (AND logic) across every `spec/features/**/*.feature` file. The
//! recursive walk reuses [`crate::io::feature_glob::glob_feature_files`]; a
//! missing `spec/features/` directory maps to an empty list so the canonical
//! `"No feature files found"` message is preserved.
//!
//! ## Line-range computation
//! The Rust `gherkin-0.16.0` AST stores scenario tags as a bare `Vec<String>`
//! with NO per-tag position, and `scenario.position.line` points at the
//! `Scenario:` keyword line (after any tags). To reproduce the TS
//! `lineStart = firstTag.location.line` we scan BACKWARD from the keyword line
//! over contiguous tag (`@…`) and comment (`#…`) lines to find the true start
//! of the scenario block. `lineEnd` is the start line of the next scenario /
//! background block, or end-of-file (parity with TS lines 92-127).
//!
//! ## Deletion
//! For each file, matching scenarios are removed bottom-up (descending start
//! line) by splicing out `[start, end)`. Runs of 4+ blank lines are collapsed
//! to 3 (parity with TS `replace(/\n{4,}/g, '\n\n\n')`). The result is
//! re-parsed; a parse failure aborts with an error envelope. Coverage sidecars
//! are updated best-effort (deleted scenario names removed, stats recomputed).
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
        let mut lines: Vec<String> = content.split('\n').map(str::to_string).collect();

        // Sort by line_start descending so bottom-up splicing keeps indices valid.
        let mut sorted = scenarios.clone();
        sorted.sort_by_key(|s| std::cmp::Reverse(s.line_start));

        for scenario in &sorted {
            // TS: lines.splice(startIndex, endIndex - startIndex) where
            // startIndex = lineStart-1, endIndex = lineEnd-1 (both 0-based);
            // removes [startIndex, endIndex).
            let start_index = scenario.line_start.saturating_sub(1);
            let end_index = scenario.line_end.saturating_sub(1);
            if start_index >= lines.len() {
                continue;
            }
            let end_index = end_index.min(lines.len());
            if end_index > start_index {
                lines.drain(start_index..end_index);
            }
        }

        let joined = lines.join("\n");
        // Collapse runs of 4+ newlines to exactly 3 (\n\n\n).
        let new_content = collapse_excessive_blank_lines(&joined);

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

        std::fs::write(&abs, &new_content).map_err(|source| FspecCoreError::Io {
            command: "delete-scenarios",
            source,
        })?;
        files_modified += 1;

        // ---- Best-effort coverage sidecar update ----
        let coverage_path = abs.with_file_name(format!(
            "{}.coverage",
            abs.file_name().and_then(|n| n.to_str()).unwrap_or("")
        ));
        let deleted_names: Vec<String> = scenarios.iter().map(|s| s.name.clone()).collect();
        update_coverage(&coverage_path, &deleted_names);
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
/// their computed line ranges, mirroring TS lines 79-137.
fn collect_matching_scenarios(
    content: &str,
    feature: &gherkin::Feature,
    file: &str,
    wanted: &[String],
) -> Vec<ScenarioInfo> {
    let lines: Vec<&str> = content.split('\n').collect();
    let total_lines = lines.len();

    // Gather the keyword line of every scenario / background for end-boundary
    // computation. Scenario keyword line is `position.line`; background too.
    let mut block_start_lines: Vec<usize> = Vec::new();
    for s in &feature.scenarios {
        block_start_lines.push(scenario_block_start_line(&lines, s.position.line));
    }
    if let Some(bg) = &feature.background {
        block_start_lines.push(scenario_block_start_line(&lines, bg.position.line));
    }
    block_start_lines.sort_unstable();

    let mut out: Vec<ScenarioInfo> = Vec::new();

    for scenario in &feature.scenarios {
        let scenario_tags: Vec<String> = scenario.tags.iter().map(|t| format!("@{t}")).collect();

        let has_all = wanted.iter().all(|t| scenario_tags.contains(t));
        if !has_all {
            continue;
        }

        // TS scenarioLineStart = scenario.location.line (keyword line, 1-based).
        let scenario_keyword_line = scenario.position.line;
        // TS firstTagLine = first tag line if tagged, else keyword line. We
        // reconstruct it by scanning backward over tag/comment lines.
        let line_start = scenario_block_start_line(&lines, scenario_keyword_line);

        // TS lineEnd: smallest other-block start line strictly greater than
        // this scenario's KEYWORD line, else end-of-file (lines.length).
        let mut line_end = total_lines;
        for &other_start in &block_start_lines {
            if other_start > scenario_keyword_line && other_start < line_end {
                line_end = other_start;
            }
        }

        out.push(ScenarioInfo {
            file: file.to_string(),
            name: scenario.name.clone(),
            tags: scenario_tags,
            line_start,
            line_end,
        });
    }

    out
}

/// Given the 1-based keyword line of a scenario/background, scan backward over
/// contiguous tag (`@…`) and comment (`#…`) lines to find the 1-based line at
/// which the block actually starts (parity with TS firstTag.location.line).
fn scenario_block_start_line(lines: &[&str], keyword_line: usize) -> usize {
    if keyword_line == 0 {
        return 1;
    }
    let mut start = keyword_line; // 1-based
                                  // Index of the line ABOVE the keyword line is keyword_line-2 (0-based).
    let mut idx = keyword_line as isize - 2;
    while idx >= 0 {
        let trimmed = lines[idx as usize].trim();
        if trimmed.starts_with('@') {
            start = (idx + 1) as usize; // back up over the tag line
            idx -= 1;
        } else {
            break;
        }
    }
    start
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

/// Collapse runs of 4+ consecutive newlines into exactly 3 (`\n\n\n`),
/// mirroring TS `newContent.replace(/\n{4,}/g, '\n\n\n')`.
fn collapse_excessive_blank_lines(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut newline_run = 0usize;
    for c in content.chars() {
        if c == '\n' {
            newline_run += 1;
            if newline_run <= 3 {
                out.push('\n');
            }
            // runs of 4+ are clamped to 3 emitted newlines
        } else {
            newline_run = 0;
            out.push(c);
        }
    }
    out
}

/// Best-effort coverage sidecar update: remove deleted scenario names and
/// recompute stats. Silently no-ops on any error (parity with TS try/catch).
fn update_coverage(coverage_path: &Path, deleted_names: &[String]) {
    let _ = update_coverage_inner(coverage_path, deleted_names);
}

fn update_coverage_inner(coverage_path: &Path, deleted_names: &[String]) -> Option<()> {
    let body = std::fs::read_to_string(coverage_path).ok()?;
    let mut coverage: Value = serde_json::from_str(&body).ok()?;

    let scenarios = coverage.get("scenarios")?.as_array()?.clone();

    let remaining: Vec<Value> = scenarios
        .into_iter()
        .filter(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .map(|n| !deleted_names.iter().any(|d| d == n))
                .unwrap_or(true)
        })
        .collect();

    let covered = remaining
        .iter()
        .filter(|s| {
            s.get("testMappings")
                .and_then(Value::as_array)
                .is_some_and(|m| !m.is_empty())
        })
        .count();
    let total = remaining.len();
    let percent: i64 = if total > 0 {
        ((covered as f64) / (total as f64) * 100.0).round() as i64
    } else {
        0
    };

    let mut stats = match coverage.get("stats") {
        Some(Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    stats.insert("totalScenarios".to_string(), json!(total));
    stats.insert("coveredScenarios".to_string(), json!(covered));
    stats.insert("coveragePercent".to_string(), json!(percent));

    if let Value::Object(map) = &mut coverage {
        map.insert("scenarios".to_string(), Value::Array(remaining));
        map.insert("stats".to_string(), Value::Object(stats));
    } else {
        return None;
    }

    let serialised = serde_json::to_string_pretty(&coverage).ok()?;
    std::fs::write(coverage_path, serialised).ok()?;
    Some(())
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

    #[test]
    fn collapse_clamps_runs_to_three_newlines() {
        let input = "a\n\n\n\n\nb"; // 5 newlines
        assert_eq!(collapse_excessive_blank_lines(input), "a\n\n\nb");
    }

    #[test]
    fn collapse_keeps_small_runs() {
        let input = "a\n\nb\n\n\nc";
        assert_eq!(collapse_excessive_blank_lines(input), "a\n\nb\n\n\nc");
    }

    #[test]
    fn block_start_backs_over_tags() {
        let lines = vec!["Feature: F", "", "  @spike", "  @wip", "  Scenario: A"];
        // keyword on line 5
        assert_eq!(scenario_block_start_line(&lines, 5), 3);
    }

    #[test]
    fn block_start_no_tags_is_keyword_line() {
        let lines = vec!["Feature: F", "", "  Scenario: A"];
        assert_eq!(scenario_block_start_line(&lines, 3), 3);
    }
}
