#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/foundation-discovery-agent-guidance.feature
//
// Tests for: show-foundation draft auto-preference + --final, the finalize
// full remaining-fields report, the show-foundation-event-storm
// unknown-context error, and validate-foundation-schema --draft.
// Each scenario maps to exactly one #[test] with @step comments mirroring
// the Gherkin steps verbatim.
//
// RED PHASE: these behaviors do not exist yet, so the tests FAIL now.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(command: &str, project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_foundation_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), body).expect("write foundation.json");
}

fn write_draft_raw(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), body).expect("write draft");
}

fn minimal_foundation(name: &str) -> String {
    format!(
        r#"{{
  "version": "2.0.0",
  "project": {{"name":"{name}","vision":"V","projectType":"cli-tool"}},
  "problemSpace": {{"primaryProblem":{{"title":"P","description":"D","impact":"high"}}}},
  "solutionSpace": {{"overview":"O","capabilities":[{{"name":"C","description":"D"}}]}},
  "personas": [{{"name":"User","description":"d","goals":["g"]}}]
}}"#
    )
}

// ---------- show-foundation draft auto-preference ----------

#[test]
fn show_foundation_with_a_draft_present_shows_the_draft_by_default() {
    // @step Given a project root with both spec/foundation.json and spec/foundation.json.draft where the draft project.name='draft-name' and the final project.name='final-name'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(tmp.path(), &minimal_foundation("final-name"));
    write_draft_raw(tmp.path(), &minimal_foundation("draft-name"));

    // @step When I dispatch show-foundation with no section
    let result = dispatch_command(req("show-foundation", tmp.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success, got {result:?}");
    let data = result.data.clone();

    // @step And the returned output starts with the banner 'Showing DRAFT (foundation.json.draft)'
    assert!(
        data.starts_with("Showing DRAFT (foundation.json.draft)"),
        "banner must be first line; got:\n{data}"
    );

    // @step And the returned output contains a progress line 'progress:' with 'fields complete'
    assert!(
        data.contains("progress:") && data.contains("fields complete"),
        "progress line missing; got:\n{data}"
    );

    // @step And the returned output reflects the draft content 'draft-name'
    assert!(
        data.contains("draft-name"),
        "must show draft content; got:\n{data}"
    );
}

#[test]
fn show_foundation_final_flag_forces_the_finalized_file_when_a_draft_exists() {
    // @step Given a project root with both spec/foundation.json and spec/foundation.json.draft where the draft project.name='draft-name' and the final project.name='final-name'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(tmp.path(), &minimal_foundation("final-name"));
    write_draft_raw(tmp.path(), &minimal_foundation("draft-name"));

    // @step When I dispatch show-foundation with final=true and section='projectName'
    let result = dispatch_command(req(
        "show-foundation",
        tmp.path(),
        json!({ "final": true, "section": "projectName" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success, got {result:?}");

    // @step And the returned output is exactly 'final-name'
    assert_eq!(
        result.data, "final-name",
        "final flag must force the final file; got: {:?}",
        result.data
    );
}

#[test]
fn show_foundation_without_a_draft_is_byte_identical_to_today() {
    // @step Given a project root with spec/foundation.json project.name='fspec' and no draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(tmp.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with no section and format='text'
    let result = dispatch_command(req(
        "show-foundation",
        tmp.path(),
        json!({ "format": "text" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success, got {result:?}");
    let data = result.data.clone();

    // @step And the returned output contains the exact line '=== PROJECT ==='
    assert!(
        data.lines().any(|l| l == "=== PROJECT ==="),
        "default text render unchanged; got:\n{data}"
    );

    // @step And the returned output does NOT contain the banner 'Showing DRAFT'
    assert!(
        !data.contains("Showing DRAFT"),
        "no-draft path must not add a draft banner; got:\n{data}"
    );
}

// ---------- finalize full remaining-fields report ----------

#[test]
fn finalize_failure_lists_every_remaining_field_with_its_fix_command() {
    // @step Given a project root whose spec/foundation.json.draft still has project.vision, problemTitle, problemDefinition, solutionOverview, capabilities, and personas unfilled
    let tmp = TempDir::new().expect("tempdir");
    let draft = json!({
        "version": "2.0.0",
        "project": {
            "name": "fspec",
            "vision": "[QUESTION: What is the one-sentence vision?]",
            "projectType": "cli-tool"
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
    });
    write_draft_raw(tmp.path(), &serde_json::to_string_pretty(&draft).unwrap());

    // @step When I dispatch discover-foundation with finalize=true
    let result = dispatch_command(req(
        "discover-foundation",
        tmp.path(),
        json!({ "finalize": true }),
    ));

    // @step Then the dispatcher returns valid=false
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["valid"].as_bool(),
        Some(false),
        "valid must be false: {data}"
    );

    let errors = data["validationErrors"].as_str().unwrap_or_default();

    // @step And the validationErrors starts with 'Cannot finalize: draft still has unfilled placeholder fields'
    assert!(
        errors.starts_with("Cannot finalize: draft still has unfilled placeholder fields"),
        "prefix must survive: {errors}"
    );

    // @step And the validationErrors names each of the 6 remaining fields with its exact fix command
    for alias in [
        "projectVision",
        "problemTitle",
        "problemDefinition",
        "solutionOverview",
        "add-capability",
        "add-persona",
    ] {
        assert!(
            errors.contains(alias),
            "finalize error must name {alias} with its fix command:\n{errors}"
        );
    }

    // @step And the validationErrors ends with 'Then re-run: fspec discover-foundation --finalize'
    assert!(
        errors
            .trim_end()
            .ends_with("Then re-run: fspec discover-foundation --finalize"),
        "must end with the re-run instruction: {errors}"
    );
}

// ---------- event-storm unknown-context error ----------

fn final_with_two_contexts() -> Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "fspec", "vision": "v", "projectType": "cli-tool" },
        "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "high" } },
        "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] },
        "eventStorm": {
            "level": "big_picture",
            "items": [
                { "id": 1, "type": "bounded_context", "text": "Auth", "color": null, "deleted": false, "createdAt": "2026-09-01T00:00:00.000Z" },
                { "id": 2, "type": "bounded_context", "text": "Specification", "color": null, "deleted": false, "createdAt": "2026-09-01T00:00:00.000Z" },
                { "id": 3, "type": "aggregate", "text": "Session", "color": null, "deleted": false, "createdAt": "2026-09-01T00:00:00.000Z", "boundedContextId": 1 }
            ],
            "nextItemId": 4
        }
    })
}

#[test]
fn show_foundation_event_storm_with_an_unknown_context_errors_and_lists_available_contexts() {
    // @step Given a project root with a finalized spec/foundation.json whose event storm has bounded contexts 'Auth' and 'Specification'
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&final_with_two_contexts()).unwrap(),
    );

    // @step When I dispatch show-foundation-event-storm with context='Aut'
    let result = dispatch_command(req(
        "show-foundation-event-storm",
        tmp.path(),
        json!({ "context": "Aut" }),
    ));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "must fail on unknown context; got {result:?}"
    );

    let err = result.error.clone().unwrap_or_default();

    // @step And the error message contains 'Unknown context'
    assert!(
        err.contains("Unknown context"),
        "must name the failure kind: {err}"
    );

    // @step And the error message lists 'Auth' and 'Specification' as available contexts
    assert!(
        err.contains("Auth") && err.contains("Specification"),
        "must list available contexts: {err}"
    );
}

#[test]
fn show_foundation_event_storm_with_a_matching_context_is_unchanged() {
    // @step Given a project root with a finalized spec/foundation.json whose event storm has bounded context 'Auth' and one aggregate inside it
    let tmp = TempDir::new().expect("tempdir");
    write_foundation_raw(
        tmp.path(),
        &serde_json::to_string_pretty(&final_with_two_contexts()).unwrap(),
    );

    // @step When I dispatch show-foundation-event-storm with context='Auth'
    let result = dispatch_command(req(
        "show-foundation-event-storm",
        tmp.path(),
        json!({ "context": "Auth" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success, got {result:?}");

    let data: Value = serde_json::from_str(&result.data).expect("parse data json");

    // @step And the returned data contains the bounded context item 'Auth'
    let items = data["data"].as_array().expect("data array");
    assert!(
        items.iter().any(|i| i["text"].as_str() == Some("Auth")),
        "Auth BC item must be present: {data}"
    );

    // @step And the returned data contains the aggregate item
    assert!(
        items.iter().any(|i| i["text"].as_str() == Some("Session")),
        "aggregate inside Auth must be present: {data}"
    );
}

// ---------- validate-foundation-schema --draft ----------

#[test]
fn validate_foundation_schema_draft_validates_the_draft_file() {
    // @step Given a project root with a spec/foundation.json.draft that is empty in solutionSpace.capabilities and no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");
    let draft = json!({
        "version": "2.0.0",
        "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
        "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
        "solutionSpace": { "overview": "o", "capabilities": [] }
    });
    write_draft_raw(tmp.path(), &serde_json::to_string_pretty(&draft).unwrap());

    // @step When I dispatch validate-foundation-schema with draft=true
    let result = dispatch_command(req(
        "validate-foundation-schema",
        tmp.path(),
        json!({ "draft": true }),
    ));

    // @step Then the dispatcher returns success=true at the envelope level
    assert!(
        result.success,
        "envelope must stay success (recoverable failure inside data); got {result:?}"
    );

    // @step And the result reports success=false with the error 'Field solutionSpace.capabilities must have at least 1 items (found 0)'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["success"].as_bool(),
        Some(false),
        "must report failure: {data}"
    );
    assert_eq!(
        data["error"].as_str(),
        Some("Field solutionSpace.capabilities must have at least 1 items (found 0)"),
        "minItems special case must apply to the draft: {data}"
    );
}

#[test]
fn validate_foundation_schema_draft_on_a_valid_draft_reports_valid() {
    // @step Given a project root with a schema-valid spec/foundation.json.draft and no spec/foundation.json
    let tmp = TempDir::new().expect("tempdir");
    let draft = json!({
        "version": "2.0.0",
        "project": { "name": "T", "vision": "v", "projectType": "cli-tool" },
        "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
        "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
    });
    write_draft_raw(tmp.path(), &serde_json::to_string_pretty(&draft).unwrap());

    // @step When I dispatch validate-foundation-schema with draft=true
    let result = dispatch_command(req(
        "validate-foundation-schema",
        tmp.path(),
        json!({ "draft": true }),
    ));

    // @step Then the dispatcher returns success=true at the envelope level
    assert!(result.success, "expected success, got {result:?}");

    // @step And the result reports success=true with an output naming the draft as valid
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(true), "got: {data}");
    let output = data["output"].as_str().unwrap_or_default();
    assert!(
        output.contains("foundation.json.draft") && output.contains("valid"),
        "output must name the draft: {data}"
    );
}

#[test]
fn validate_foundation_schema_draft_with_no_draft_file_reports_a_friendly_error() {
    // @step Given an empty project root with no spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch validate-foundation-schema with draft=true
    let result = dispatch_command(req(
        "validate-foundation-schema",
        tmp.path(),
        json!({ "draft": true }),
    ));

    // @step Then the dispatcher returns success=true at the envelope level
    assert!(result.success, "expected success, got {result:?}");

    // @step And the result reports success=false with an error naming foundation.json.draft
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(data["success"].as_bool(), Some(false), "got: {data}");
    assert!(
        data["error"]
            .as_str()
            .unwrap_or_default()
            .contains("foundation.json.draft"),
        "error must name the draft file: {data}"
    );
}
