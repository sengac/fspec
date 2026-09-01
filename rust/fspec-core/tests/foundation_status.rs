#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/foundation-discovery-agent-guidance.feature
//
// Dispatcher-contract tests for the NEW `foundation-status` command.
// Each scenario maps to exactly one #[test] with @step comments mirroring the
// Gherkin steps verbatim.
//
// RED PHASE: the command does not exist yet, so these tests FAIL now (the
// dispatcher returns UnknownCommand). They assert the real expected behaviour
// the implementation must satisfy.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "foundation-status".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_draft_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), body).expect("write draft");
}

fn write_foundation_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), body).expect("write foundation.json");
}

/// The canonical 8-field placeholder draft `discover-foundation` writes.
fn placeholder_draft() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "[QUESTION: What is the project name?]",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "[DETECTED: cli-tool]"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "[QUESTION: What problem does this solve?]",
                "description": "[QUESTION: What problem does this solve?]",
                "impact": "high"
            }
        },
        "solutionSpace": { "overview": "[QUESTION: What can users DO?]", "capabilities": [] },
        "personas": [
            {
                "name": "[QUESTION: Who uses this?]",
                "description": "[QUESTION: Who uses this?]",
                "goals": ["[QUESTION: What are their goals?]"]
            }
        ]
    })
}

/// A fully-filled, schema-valid foundation.
fn valid_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "fspec", "vision": "Ship faster", "projectType": "cli-tool" },
        "problemSpace": {
            "primaryProblem": { "title": "Spec drift", "description": "Real pain", "impact": "high" }
        },
        "solutionSpace": {
            "overview": "A CLI",
            "capabilities": [ { "name": "Spec Validation", "description": "Validates" } ]
        },
        "personas": [ { "name": "Developer", "description": "Builds", "goals": ["Ship"] } ]
    })
}

// ---------- scenarios ----------

#[test]
fn foundation_status_reports_missing_phase_when_no_foundation_exists() {
    // @step Given an empty project root with no spec/foundation.json and no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch foundation-status
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");
    let data = result.data.clone();

    // @step And the returned status reports phase 'none'
    assert!(
        data.contains("Foundation: MISSING"),
        "must report missing phase; got:\n{data}"
    );

    // @step And the returned status tells me to run 'fspec discover-foundation' to start
    assert!(
        data.contains("fspec discover-foundation"),
        "must point to discover-foundation; got:\n{data}"
    );
}

#[test]
fn foundation_status_on_fresh_draft_lists_every_remaining_field_with_fix_command_and_example() {
    // @step Given a project root whose spec/foundation.json.draft is the canonical 8-field placeholder draft
    let tmp = TempDir::new().expect("tempdir");
    write_draft_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&placeholder_draft()).unwrap(),
    );

    // @step When I dispatch foundation-status
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");
    let data = result.data.clone();

    // @step And the returned status reports phase 'draft' with 'Progress: 0/8 fields complete'
    assert!(
        data.contains("Foundation: DRAFT"),
        "must report draft phase; got:\n{data}"
    );
    assert!(
        data.contains("Progress: 0/8 fields complete"),
        "must report 0/8 progress; got:\n{data}"
    );

    // @step And the status shows all 8 fields as incomplete with a per-field preview
    for field in [
        "project.name",
        "project.vision",
        "project.projectType",
        "problemSpace.primaryProblem.title",
        "problemSpace.primaryProblem.description",
        "solutionSpace.overview",
        "solutionSpace.capabilities",
        "personas",
    ] {
        assert!(
            data.contains(field),
            "progress table must name {field}; got:\n{data}"
        );
    }

    // @step And the 'Remaining' section lists all 8 fields, each with its fix command
    assert!(
        data.contains("Remaining"),
        "must have a Remaining section; got:\n{data}"
    );
    for alias in [
        "projectName",
        "projectVision",
        "projectType",
        "problemTitle",
        "problemDefinition",
        "solutionOverview",
        "capabilities",
        "personas",
    ] {
        assert!(
            data.contains(alias),
            "Remaining must list {alias}; got:\n{data}"
        );
    }

    // @step And the 'Remaining' section includes an example for the problemDefinition field
    assert!(
        data.contains("Example:"),
        "Remaining must include at least one Example line; got:\n{data}"
    );

    // @step And the status ends with 'When complete: fspec discover-foundation --finalize'
    assert!(
        data.trim_end()
            .ends_with("When complete: fspec discover-foundation --finalize"),
        "must end with the finalize next-action; got:\n{data}"
    );
}

#[test]
fn foundation_status_on_partially_filled_draft_reports_correct_per_field_status() {
    // @step Given a project root whose spec/foundation.json.draft has project.name, project.vision, and project.projectType filled and the other 5 fields still placeholder
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = placeholder_draft();
    draft["project"]["name"] = json!("fspec");
    draft["project"]["vision"] = json!("Ship faster");
    draft["project"]["projectType"] = json!("cli-tool");
    write_draft_raw(tmp.path(), &serde_json::to_string_pretty(&draft).unwrap());

    // @step When I dispatch foundation-status
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");
    let data = result.data.clone();

    // @step And the returned status reports 'Progress: 3/8 fields complete'
    assert!(
        data.contains("Progress: 3/8 fields complete"),
        "must report 3/8; got:\n{data}"
    );

    // @step And the 3 filled fields are marked complete with their current values as previews
    assert!(
        data.contains("fspec"),
        "preview for project.name; got:\n{data}"
    );
    assert!(
        data.contains("Ship faster"),
        "preview for project.vision; got:\n{data}"
    );
    assert!(
        data.contains("cli-tool"),
        "preview for project.projectType; got:\n{data}"
    );

    // @step And the 5 unfilled fields are marked incomplete
    assert!(
        data.contains("Remaining"),
        "incomplete fields must appear in Remaining; got:\n{data}"
    );
    assert!(
        data.contains("problemTitle"),
        "unfilled title in remaining; got:\n{data}"
    );
    assert!(
        data.contains("solutionOverview"),
        "unfilled overview in remaining; got:\n{data}"
    );
}

#[test]
fn foundation_status_on_finalized_foundation_reports_final_phase_with_no_remaining() {
    // @step Given a project root with a fully filled spec/foundation.json and no draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&valid_foundation()).unwrap(),
    );

    // @step When I dispatch foundation-status
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");
    let data = result.data.clone();

    // @step And the returned status reports phase 'final'
    assert!(
        data.contains("Foundation: FINAL"),
        "must report final phase; got:\n{data}"
    );

    // @step And the returned status reports 'Progress: 8/8 fields complete'
    assert!(
        data.contains("Progress: 8/8 fields complete"),
        "must report 8/8; got:\n{data}"
    );

    // @step And the returned status has no remaining fields
    assert!(
        !data.contains("Remaining (in any order"),
        "no Remaining section expected when complete; got:\n{data}"
    );
}

#[test]
fn foundation_status_in_json_mode_returns_machine_readable_envelope() {
    // @step Given a project root whose spec/foundation.json.draft has project.name filled and the rest placeholder
    let tmp = TempDir::new().expect("tempdir");
    let mut draft = placeholder_draft();
    draft["project"]["name"] = json!("fspec");
    write_draft_raw(tmp.path(), &serde_json::to_string_pretty(&draft).unwrap());

    // @step When I dispatch foundation-status with json=true
    let result = dispatch_command(req(tmp.path(), json!({ "json": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success; got {result:?}");

    // @step And the returned data parses as JSON with keys phase, progress, fields, remaining, and nextAction
    let data: Value = serde_json::from_str(&result.data)
        .unwrap_or_else(|e| panic!("data must be JSON: {e}; got:\n{}", result.data));
    for key in ["phase", "progress", "fields", "remaining", "nextAction"] {
        assert!(
            data.get(key).is_some(),
            "json envelope must have key {key}; got: {data}"
        );
    }
    assert_eq!(
        data["phase"].as_str(),
        Some("draft"),
        "phase must be draft: {data}"
    );

    // @step And the fields array has 8 entries each carrying path, alias, status, and preview
    let fields = data["fields"].as_array().expect("fields must be an array");
    assert_eq!(fields.len(), 8, "fields must have 8 rows: {data}");
    for f in fields {
        for key in ["path", "alias", "status", "preview"] {
            assert!(f.get(key).is_some(), "each field must carry {key}: {f}");
        }
    }

    // @step And the remaining array lists exactly the 7 incomplete fields
    let remaining = data["remaining"]
        .as_array()
        .expect("remaining must be an array");
    assert_eq!(
        remaining.len(),
        7,
        "remaining must list 7 incomplete fields: {data}"
    );
}
