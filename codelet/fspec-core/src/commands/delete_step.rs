//! `delete-step` — Rust port of `src/commands/delete-step.ts` (RPC-221).
//!
//! Deletes a single step line from a named scenario in a Gherkin feature
//! file. The lenient parser locates the scenario by exact name, then the
//! step is matched when the supplied arg equals either the step text
//! (`Step.value`) OR the full `(keyword + text).trim()`. gherkin-0.16's
//! `Step.keyword` carries a trailing space, so `format!("{}{}", keyword,
//! value)` reproduces the original line text before trimming.
//!
//! Only the single matched step line (`Step.position.line`, 1-based) is
//! removed via a line-based `split('\n')` / `join('\n')` edit; consecutive
//! blank lines are collapsed to at most two and the result is re-parsed to
//! guarantee it is still valid Gherkin before the file is written.
//! delete-step does NOT touch any coverage sidecar.
//!
//! ## Recoverable-error contract
//! Mirroring the TS `DeleteStepResult { success, message?, error? }` shape:
//! missing files, missing scenarios, missing steps, invalid Gherkin, and a
//! post-deletion re-parse failure all surface as
//! [`FspecCoreError::InvalidArgs`] so the dispatcher reports
//! `success=false` and the CLI bridge prints `Error: <reason>` + exit 1.
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/delete_step.rs` is JSON marshalling only.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::gherkin::parse_feature_lenient;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteStepArgs {
    feature: String,
    scenario: String,
    step: String,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DeleteStepArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "delete-step",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Resolve feature path (TS parity, src/commands/delete-step.ts:28-35) ----
    let feature_path = resolve_feature_path(project_root, &args.feature);

    // ---- Read feature file ----
    let content = match std::fs::read_to_string(&feature_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(FspecCoreError::InvalidArgs {
                command: "delete-step",
                reason: format!("Feature file not found: {}", feature_path.display()),
            });
        }
        Err(source) => {
            return Err(FspecCoreError::Io {
                command: "delete-step",
                source,
            });
        }
    };

    // ---- Parse Gherkin ----
    let feature = match parse_feature_lenient(&content) {
        Ok(f) => f,
        Err(e) => {
            return Err(FspecCoreError::InvalidArgs {
                command: "delete-step",
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
            command: "delete-step",
            reason: format!("Scenario '{}' not found in feature file", args.scenario),
        })?;

    // ---- Match the step by text OR (keyword + text).trim() ----
    let target_arg = args.step.trim();
    let step = scenario
        .steps
        .iter()
        .find(|s| {
            let full = format!("{}{}", s.keyword, s.value);
            s.value == args.step || full.trim() == target_arg
        })
        .ok_or(FspecCoreError::InvalidArgs {
            command: "delete-step",
            reason: format!(
                "Step '{}' not found in scenario '{}'",
                args.step, args.scenario
            ),
        })?;

    let step_line = step.position.line;

    // ---- Remove the single step line ----
    let lines: Vec<&str> = content.split('\n').collect();
    let line_index = (step_line - 1) as usize;
    let mut new_lines: Vec<&str> = Vec::with_capacity(lines.len());
    new_lines.extend_from_slice(&lines[..line_index]);
    if line_index + 1 < lines.len() {
        new_lines.extend_from_slice(&lines[line_index + 1..]);
    }

    // ---- Collapse runs of >2 blank lines ----
    let collapsed = collapse_blank_lines(&new_lines);
    let new_content = collapsed.join("\n");

    // ---- Validate result re-parses ----
    if let Err(e) = parse_feature_lenient(&new_content) {
        return Err(FspecCoreError::InvalidArgs {
            command: "delete-step",
            reason: format!("Deletion would result in invalid Gherkin: {e}"),
        });
    }

    // ---- Write the updated feature file ----
    std::fs::write(&feature_path, &new_content).map_err(|source| FspecCoreError::Io {
        command: "delete-step",
        source,
    })?;

    let file_name = feature_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&args.feature)
        .to_string();

    let response = json!({
        "success": true,
        "message": format!(
            "Successfully deleted step from scenario '{}' in {}",
            args.scenario, file_name
        ),
    });

    serde_json::to_string(&response).map_err(|e| FspecCoreError::InvalidArgs {
        command: "delete-step",
        reason: format!("failed to serialise response: {e}"),
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Resolve the feature path exactly like the TS reference
/// (src/commands/delete-step.ts:28-35).
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
/// Mirrors TS `trimmedLines` accumulation (lines 115-127).
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: DeleteStepArgs = serde_json::from_str(
            r#"{"feature":"spec/features/x.feature","scenario":"Login","step":"When x"}"#,
        )
        .unwrap();
        assert_eq!(a.feature, "spec/features/x.feature");
        assert_eq!(a.scenario, "Login");
        assert_eq!(a.step, "When x");
    }
}
