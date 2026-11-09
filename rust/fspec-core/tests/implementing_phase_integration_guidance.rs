//! REMIND-015 — the IMPLEMENTING reminder puts integration first, and the
//! SPECIFYING reminder asks who calls the new code.
//!
//! Feature: spec/features/reword-implementing-phase-guidance-to-prevent-llms-skipping-integration-work.feature
//!
//! Each test moves a task through `update-work-unit-status` (a task may go
//! from specifying straight to implementing, so no coverage gate applies)
//! and reads the system reminder from the command's output.

use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A board whose only work unit, the task WORK-001, is in `status`.
fn board(dir: &Path, status: &str) -> TestResult {
    let mut states = serde_json::Map::new();
    for state in [
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let ids: Vec<&str> = if state == status {
            vec!["WORK-001"]
        } else {
            vec![]
        };
        states.insert(state.to_string(), json!(ids));
    }
    std::fs::create_dir_all(dir.join("spec"))?;
    std::fs::write(
        dir.join("spec/work-units.json"),
        serde_json::to_string_pretty(&json!({
            "version": "0.7.1",
            "meta": {"version": "1.0.0", "lastUpdated": "2026-11-01T00:00:00.000Z"},
            "workUnits": {"WORK-001": {
                "id": "WORK-001", "title": "Wire the exporter", "type": "task", "status": status,
                "createdAt": "2026-11-01T00:00:00.000Z", "updatedAt": "2026-11-01T00:00:00.000Z"
            }},
            "states": states,
        }))?,
    )?;
    Ok(())
}

/// Move WORK-001 from `from` to `to` and return the command's output.
fn transition(from: &str, to: &str) -> Result<String, Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir()?;
    board(tmp.path(), from)?;
    let result = dispatch_command(DispatchRequest {
        command: "update-work-unit-status".to_string(),
        args_json: json!({"workUnitId": "WORK-001", "status": to}).to_string(),
        project_root: tmp.path().to_path_buf(),
    });
    if !result.success {
        return Err(format!("the move to {to} failed: {result:?}").into());
    }
    Ok(result.data)
}

/// The text of the one system reminder in `output`.
fn reminder(output: &str) -> Result<&str, Box<dyn std::error::Error>> {
    let start = output
        .find("<system-reminder>")
        .ok_or_else(|| format!("no system reminder in:\n{output}"))?;
    let end = output
        .find("</system-reminder>")
        .ok_or_else(|| format!("unclosed system reminder in:\n{output}"))?;
    Ok(&output[start..end])
}

fn implementing_reminder() -> Result<String, Box<dyn std::error::Error>> {
    let output = transition("specifying", "implementing")?;
    let text = reminder(&output)?;
    assert!(
        text.contains("Work unit WORK-001 is now in IMPLEMENTING status."),
        "{text}"
    );
    Ok(text.to_string())
}

#[test]
fn implementing_phase_shows_creation_plus_connection_prominently() -> TestResult {
    // @step Given a work unit transitions to IMPLEMENTING status
    // @step When the status change reminder is generated
    let text = implementing_reminder()?;

    // @step Then the reminder should contain "IMPLEMENTATION = CREATION + CONNECTION"
    assert!(
        text.contains("IMPLEMENTATION = CREATION + CONNECTION"),
        "{text}"
    );
    Ok(())
}

#[test]
fn implementing_phase_includes_who_calls_this_heuristic() -> TestResult {
    // @step Given a work unit transitions to IMPLEMENTING status
    // @step When the status change reminder is generated
    let text = implementing_reminder()?;

    // @step Then the reminder should contain "WHO CALLS THIS?"
    assert!(text.contains("WHO CALLS THIS?"), "{text}");
    Ok(())
}

#[test]
fn implementing_phase_lists_complete_means_checklist() -> TestResult {
    // @step Given a work unit transitions to IMPLEMENTING status
    // @step When the status change reminder is generated
    let text = implementing_reminder()?;

    // @step Then the reminder should contain "COMPLETE MEANS:"
    assert!(text.contains("COMPLETE MEANS:"), "{text}");

    // @step And the reminder should contain "New code exists AND is connected to the system"
    assert!(
        text.contains("New code exists AND is connected to the system"),
        "{text}"
    );

    // @step And the reminder should contain "Feature works end-to-end"
    assert!(text.contains("Feature works end-to-end"), "{text}");
    Ok(())
}

#[test]
fn implementing_phase_removes_minimization_encouraging_phrases() -> TestResult {
    // @step Given a work unit transitions to IMPLEMENTING status
    // @step When the status change reminder is generated
    let text = implementing_reminder()?;

    // @step Then the reminder should not contain "ONLY enough"
    assert!(!text.contains("ONLY enough"), "{text}");

    // @step And the reminder should not contain "minimum code"
    assert!(!text.contains("minimum code"), "{text}");

    // @step And the reminder should not contain "minimal code"
    assert!(!text.contains("minimal code"), "{text}");

    // @step And the reminder should not contain "Avoid over-implementation"
    assert!(!text.contains("Avoid over-implementation"), "{text}");
    Ok(())
}

#[test]
fn specifying_phase_includes_who_calls_this_prompt_for_integration_points() -> TestResult {
    // @step Given a work unit transitions to SPECIFYING status
    // @step When the status change reminder is generated
    let output = transition("backlog", "specifying")?;
    let text = reminder(&output)?;
    assert!(
        text.contains("Work unit WORK-001 is now in SPECIFYING status."),
        "{text}"
    );

    // @step Then the reminder should contain "WHO CALLS THIS?"
    assert!(text.contains("WHO CALLS THIS?"), "{text}");

    // @step And the reminder should prompt for integration scenarios
    assert!(
        text.contains(
            "If yes, capture integration points as examples that become @integration scenarios."
        ),
        "{text}"
    );
    Ok(())
}
