//! `workflow-automation` — Rust port of `src/commands/workflow-automation.ts`
//! (RPC-326).
//!
//! Dispatches one of three workflow-automation actions against a single work
//! unit:
//!
//! * `record-iteration`   — increment the NESTED `metrics.iterations` counter
//!   (distinct from the standalone `record-iteration` command, which uses a
//!   TOP-LEVEL `iterations` field). Mutates + atomically persists.
//! * `auto-advance`        — advance `status` after a workflow event, append a
//!   `stateHistory` entry, and fix the `states` index. Mutates + persists.
//! * `validate-alignment`  — READ-ONLY count of `@<id>` tagged scenarios in
//!   `spec/features/**/*.feature`. NEVER writes work-units.json.
//!
//! Any other action surfaces `Invalid action: <action>`.
//!
//! ## TS source of truth (`src/commands/workflow-automation.ts:36-197`)
//!
//! Error messages are emitted VERBATIM (no wrapping prefix) for parity with
//! the TS functions, which throw bare `Error` instances:
//!   * `Work unit '{id}' does not exist`
//!   * `Work unit '{id}' is in state '{status}', expected '{from}'`
//!   * `Invalid transition: {event} from {from}`
//!   * `Invalid action: {action}`
//!
//! ## Two-front-doors
//!
//! Both the LLM dispatcher AND the standalone binary's clap subcommand call
//! this single function. Unlike `auto-advance`, the TS Commander shell binds
//! correctly (positional `<action> <work-unit-id>` + `--event`/`--from-state`),
//! so the CLI bridge at `rust/fspec/src/workflow_automation.rs` marshals
//! those values straight through — NOT Framing A.

use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::io_error::format_io_error;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::{WorkUnitStates, WorkUnitStatus, WorkUnitsData};

/// CLI / dispatcher arguments accepted by `workflow-automation`. Mirrors the TS
/// `workflowAutomation(action, workUnitId, options)` signature
/// (`src/commands/workflow-automation.ts:173-181`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowAutomationArgs {
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    work_unit_id: Option<String>,
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    from_state: Option<String>,
}

#[derive(Debug, Serialize)]
struct RecordIterationResult {
    success: bool,
    iterations: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoAdvanceResult {
    success: bool,
    new_state: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AlignmentResult {
    aligned: bool,
    scenarios_found: u64,
    features: Vec<String>,
}

fn invalid_args(reason: String) -> FspecCoreError {
    FspecCoreError::InvalidArgs {
        command: "workflow-automation",
        reason,
    }
}

/// Mutable accessor for one of the 7 typed state arrays by its lowercase name.
fn state_vec_mut<'a>(states: &'a mut WorkUnitStates, name: &str) -> Option<&'a mut Vec<String>> {
    match name {
        "backlog" => Some(&mut states.backlog),
        "specifying" => Some(&mut states.specifying),
        "testing" => Some(&mut states.testing),
        "implementing" => Some(&mut states.implementing),
        "validating" => Some(&mut states.validating),
        "done" => Some(&mut states.done),
        "blocked" => Some(&mut states.blocked),
        _ => None,
    }
}

/// Map a known target-state name to its [`WorkUnitStatus`]. The three
/// auto-advance targets (`implementing` / `done` / `testing`) are always
/// recognised; any other value is rejected upstream by the transition table.
fn status_from_str(name: &str) -> Option<WorkUnitStatus> {
    match name {
        "backlog" => Some(WorkUnitStatus::Backlog),
        "specifying" => Some(WorkUnitStatus::Specifying),
        "testing" => Some(WorkUnitStatus::Testing),
        "implementing" => Some(WorkUnitStatus::Implementing),
        "validating" => Some(WorkUnitStatus::Validating),
        "done" => Some(WorkUnitStatus::Done),
        "blocked" => Some(WorkUnitStatus::Blocked),
        _ => None,
    }
}

fn load_work_units(
    project_root: &Path,
) -> Result<(std::path::PathBuf, WorkUnitsData), FspecCoreError> {
    let work_units_path = project_root.join("spec").join("work-units.json");
    let raw = std::fs::read_to_string(&work_units_path)
        .map_err(|e| invalid_args(format_io_error(&e, &work_units_path.display().to_string())))?;
    let data: WorkUnitsData = serde_json::from_str(&raw).map_err(|e| {
        FspecCoreError::JsonSyntax(crate::io::json_error::parse_json_reason(&raw, &e))
    })?;
    Ok((work_units_path, data))
}

/// Dispatcher entry point. Two-front-doors invariant.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: WorkflowAutomationArgs = serde_json::from_str(args_json)
        .map_err(|e| invalid_args(format!("failed to parse args: {e}")))?;

    let action = args.action.as_deref().unwrap_or("");
    let work_unit_id = args.work_unit_id.as_deref().unwrap_or("undefined");

    match action {
        "record-iteration" => record_iteration(project_root, work_unit_id),
        "auto-advance" if args.event.is_some() && args.from_state.is_some() => auto_advance(
            project_root,
            work_unit_id,
            args.from_state.as_deref().unwrap_or(""),
            args.event.as_deref().unwrap_or(""),
        ),
        "validate-alignment" => validate_alignment(project_root, work_unit_id),
        // Mirrors the TS final `else` branch (workflow-automation.ts:194-196):
        // anything unrecognised — including `auto-advance` without both
        // `--event` and `--from-state` — falls through to Invalid action.
        _ => Err(invalid_args(format!("Invalid action: {action}"))),
    }
}

/// `record-iteration` action — increment NESTED `metrics.iterations`
/// (TS `recordWorkUnitIteration`, workflow-automation.ts:36-61).
fn record_iteration(project_root: &Path, work_unit_id: &str) -> Result<String, FspecCoreError> {
    let (path, mut data) = load_work_units(project_root)?;

    let wu = data
        .work_units
        .get_mut(work_unit_id)
        .ok_or_else(|| invalid_args(format!("Work unit '{work_unit_id}' does not exist")))?;

    // `if (!workUnit.metrics) workUnit.metrics = {}`
    let metrics = wu
        .extra
        .entry("metrics".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !metrics.is_object() {
        *metrics = Value::Object(Map::new());
    }
    let metrics_obj = metrics
        .as_object_mut()
        .ok_or_else(|| invalid_args("metrics is not an object".to_string()))?;

    // `workUnit.metrics.iterations = (workUnit.metrics.iterations || 0) + 1`
    let current = metrics_obj
        .get("iterations")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let next = current + 1;
    metrics_obj.insert("iterations".to_string(), Value::from(next));

    wu.updated_at = iso8601_now();

    write_json_atomic(&path, &data)?;

    let result = RecordIterationResult {
        success: true,
        iterations: next,
    };
    serde_json::to_string(&result)
        .map_err(|e| invalid_args(format!("failed to serialize result: {e}")))
}

/// `auto-advance` action — advance state after a workflow event
/// (TS `autoAdvanceWorkUnitState`, workflow-automation.ts:63-129).
fn auto_advance(
    project_root: &Path,
    work_unit_id: &str,
    from_state: &str,
    event: &str,
) -> Result<String, FspecCoreError> {
    let (path, mut data) = load_work_units(project_root)?;

    // Existence check (workflow-automation.ts:75-77).
    if !data.work_units.contains_key(work_unit_id) {
        return Err(invalid_args(format!(
            "Work unit '{work_unit_id}' does not exist"
        )));
    }

    // State match check BEFORE computing next state (workflow-automation.ts:82-86).
    let current_status = data.work_units[work_unit_id].status.as_str().to_string();
    if current_status != from_state {
        return Err(invalid_args(format!(
            "Work unit '{work_unit_id}' is in state '{current_status}', expected '{from_state}'"
        )));
    }

    // Determine next state by (event, from_state) (workflow-automation.ts:91-99).
    let next_state = match (event, from_state) {
        ("tests-pass", "testing") => "implementing",
        ("validation-pass", "validating") => "done",
        ("specs-complete", "specifying") => "testing",
        _ => {
            return Err(invalid_args(format!(
                "Invalid transition: {event} from {from_state}"
            )));
        }
    };
    let next_status = status_from_str(next_state)
        .ok_or_else(|| invalid_args(format!("Invalid transition: {event} from {from_state}")))?;
    let now = iso8601_now();

    // Update states index (workflow-automation.ts:114-124).
    if let Some(vec) = state_vec_mut(&mut data.states, from_state) {
        vec.retain(|id| id != work_unit_id);
    }
    if let Some(vec) = state_vec_mut(&mut data.states, next_state) {
        if !vec.iter().any(|id| id == work_unit_id) {
            vec.push(work_unit_id.to_string());
        }
    }

    // Update the work unit + append stateHistory (workflow-automation.ts:102-126).
    let wu = data
        .work_units
        .get_mut(work_unit_id)
        .ok_or_else(|| invalid_args(format!("Work unit '{work_unit_id}' does not exist")))?;
    wu.status = next_status;

    let history_entry = json!({ "state": next_state, "timestamp": now });
    match wu.extra.get_mut("stateHistory") {
        Some(Value::Array(arr)) => arr.push(history_entry),
        _ => {
            wu.extra.insert(
                "stateHistory".to_string(),
                Value::Array(vec![history_entry]),
            );
        }
    }

    wu.updated_at = now;

    write_json_atomic(&path, &data)?;

    let result = AutoAdvanceResult {
        success: true,
        new_state: next_state.to_string(),
    };
    serde_json::to_string(&result)
        .map_err(|e| invalid_args(format!("failed to serialize result: {e}")))
}

/// `validate-alignment` action — READ-ONLY count of `@<id>` tagged scenarios
/// (TS `validateWorkUnitSpecAlignment`, workflow-automation.ts:131-171).
fn validate_alignment(project_root: &Path, work_unit_id: &str) -> Result<String, FspecCoreError> {
    let (_path, data) = load_work_units(project_root)?;

    if !data.work_units.contains_key(work_unit_id) {
        return Err(invalid_args(format!(
            "Work unit '{work_unit_id}' does not exist"
        )));
    }

    // `new RegExp(\`@${workUnitId}\\b\`, 'g')` — escape the id for safety;
    // normal work-unit ids (alphanumeric + dash) are unaffected.
    let pattern = format!(r"@{}\b", regex::escape(work_unit_id));
    let re = Regex::new(&pattern)
        .map_err(|e| invalid_args(format!("failed to build alignment regex: {e}")))?;

    // TS globs `**/*.feature` under spec/features; a missing directory yields
    // an empty match set rather than an error.
    let files = match glob_feature_files(project_root) {
        Ok(f) => f,
        Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
        Err(e) => return Err(e),
    };

    let mut scenarios_found: u64 = 0;
    let mut features: Vec<String> = Vec::new();
    let prefix = "spec/features/";

    for file in files {
        let abs = project_root.join(&file);
        let content = std::fs::read_to_string(&abs)
            .map_err(|e| invalid_args(format_io_error(&e, &abs.display().to_string())))?;
        let count = re.find_iter(&content).count() as u64;
        if count > 0 {
            scenarios_found += count;
            // TS pushes the path relative to featuresDir (e.g. `login.feature`);
            // glob_feature_files returns it relative to project_root.
            let rel = file.strip_prefix(prefix).unwrap_or(&file).to_string();
            features.push(rel);
        }
    }

    let result = AlignmentResult {
        aligned: scenarios_found > 0,
        scenarios_found,
        features,
    };
    serde_json::to_string(&result)
        .map_err(|e| invalid_args(format!("failed to serialize result: {e}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: WorkflowAutomationArgs = serde_json::from_str(
            r#"{"action":"auto-advance","workUnitId":"AUTH-001","event":"tests-pass","fromState":"testing"}"#,
        )
        .unwrap();
        assert_eq!(a.action.as_deref(), Some("auto-advance"));
        assert_eq!(a.work_unit_id.as_deref(), Some("AUTH-001"));
        assert_eq!(a.event.as_deref(), Some("tests-pass"));
        assert_eq!(a.from_state.as_deref(), Some("testing"));
    }

    #[test]
    fn status_from_str_known_values() {
        assert_eq!(
            status_from_str("implementing"),
            Some(WorkUnitStatus::Implementing)
        );
        assert_eq!(status_from_str("done"), Some(WorkUnitStatus::Done));
        assert_eq!(status_from_str("testing"), Some(WorkUnitStatus::Testing));
        assert_eq!(status_from_str("nope"), None);
    }
}
