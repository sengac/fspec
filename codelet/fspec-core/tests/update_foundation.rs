#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/update-foundation-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `update-foundation`
// (RPC-312). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Supervisor rulings honoured (orchestration-state.md):
//   - 2-arg run(args_json, project_root); draft path = spec/foundation.json.draft
//   - D1 (discover_foundation chaining) and D2 (schema gate) are deferred
//     divergences — NOT tested here.

use std::fs;
use std::path::Path;

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "update-foundation".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// A complete generic-schema v2.0.0 foundation document.
fn valid_foundation() -> Value {
    json!({
        "version": "2.0.0",
        "project": {
            "name": "Original Name",
            "vision": "Original Vision",
            "projectType": "cli-tool"
        },
        "problemSpace": {
            "primaryProblem": {
                "title": "Primary Problem",
                "description": "Problem description",
                "impact": "high"
            }
        },
        "solutionSpace": {
            "overview": "Solution overview",
            "capabilities": [
                { "name": "Core Capability", "description": "What the system does" }
            ]
        }
    })
}

fn write_foundation(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser foundation"),
    )
    .expect("write foundation.json");
}

fn write_draft(project_root: &Path, value: &Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json.draft"),
        serde_json::to_string_pretty(value).expect("ser draft"),
    )
    .expect("write foundation.json.draft");
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read json");
    serde_json::from_str(&raw).expect("parse json")
}

// ---------- scenarios ----------

#[test]
fn updating_project_name_on_final_sets_nested_field_and_regenerates_md() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &valid_foundation());

    // @step When I dispatch update-foundation with section='projectName' and content='Acme Tool'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "projectName", "content": "Acme Tool"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Updated "projectName" section in FOUNDATION.md'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Updated \"projectName\" section in FOUNDATION.md")
    );

    // @step And spec/foundation.json on disk shows project.name='Acme Tool'
    let v = read_json(&tmp.path().join("spec/foundation.json"));
    assert_eq!(v["project"]["name"].as_str(), Some("Acme Tool"));

    // @step And spec/FOUNDATION.md exists on disk
    assert!(
        tmp.path().join("spec/FOUNDATION.md").exists(),
        "FOUNDATION.md must be regenerated on the final path"
    );
}

#[test]
fn updating_problem_impact_with_invalid_enum_fails_fast_and_leaves_file_unchanged() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &valid_foundation());
    let pre = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch update-foundation with section='problemImpact' and content='urgent'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "problemImpact", "content": "urgent"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Invalid value for problemImpact: "urgent". Valid values: high, medium, low.'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains(
            "Invalid value for problemImpact: \"urgent\". Valid values: high, medium, low."
        ),
        "expected enum error; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(
        pre, post,
        "foundation.json must NOT change on validation failure"
    );
}

#[test]
fn updating_project_type_too_long_fails_fast() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &valid_foundation());
    let pre = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch update-foundation with section='projectType' and content='this-project-type-descriptor-is-far-too-long'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "projectType", "content": "this-project-type-descriptor-is-far-too-long"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Invalid projectType: too long (must be 1-30 characters, got 44).'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Invalid projectType: too long (must be 1-30 characters, got 44)."),
        "expected length error; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn when_draft_exists_the_draft_is_the_write_target_with_no_md_regen() {
    // @step Given a project root tempdir with an existing spec/foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_draft(tmp.path(), &valid_foundation());

    // @step When I dispatch update-foundation with section='projectVision' and content='Ship faster'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "projectVision", "content": "Ship faster"}),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the returned message is 'Updated "projectVision" in foundation.json.draft'
    let data: Value = serde_json::from_str(&result.data).expect("parse data json");
    assert_eq!(
        data["message"].as_str(),
        Some("Updated \"projectVision\" in foundation.json.draft")
    );

    // @step And spec/foundation.json.draft on disk shows project.vision='Ship faster'
    let v = read_json(&tmp.path().join("spec/foundation.json.draft"));
    assert_eq!(v["project"]["vision"].as_str(), Some("Ship faster"));
}

#[test]
fn unknown_section_is_rejected_and_file_left_unchanged() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &valid_foundation());
    let pre = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch update-foundation with section='bogusSection' and content='whatever'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "bogusSection", "content": "whatever"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Unknown section: "bogusSection"'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Unknown section: \"bogusSection\""),
        "expected unknown-section error; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn empty_section_name_is_rejected_before_any_file_io() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &valid_foundation());
    let pre = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch update-foundation with section='' and content='whatever'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "", "content": "whatever"}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Section name cannot be empty'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Section name cannot be empty"),
        "expected empty-section error; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(pre, post);
}

#[test]
fn empty_content_for_normal_section_is_rejected_with_generic_error() {
    // @step Given a project root tempdir with an existing spec/foundation.json and no foundation.json.draft
    let tmp = TempDir::new().expect("tempdir");
    write_foundation(tmp.path(), &valid_foundation());
    let pre = fs::read(tmp.path().join("spec/foundation.json")).unwrap();

    // @step When I dispatch update-foundation with section='projectName' and content=''
    let result = dispatch_command(req(
        tmp.path(),
        json!({"section": "projectName", "content": ""}),
    ));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected failure; got {result:?}");

    // @step And the error message contains the substring 'Section content cannot be empty'
    let err = result.error.unwrap_or_default();
    assert!(
        err.contains("Section content cannot be empty"),
        "expected empty-content error; got: {err}"
    );

    // @step And spec/foundation.json on disk is byte-equal to its pre-call contents
    let post = fs::read(tmp.path().join("spec/foundation.json")).unwrap();
    assert_eq!(pre, post);
}
