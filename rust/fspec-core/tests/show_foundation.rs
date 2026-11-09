//! Feature: spec/features/show-foundation-rust-port.feature
//!
//! Dispatcher-level tests for `show-foundation` (RPC-305), moved out of
//! rust/fspec/tests/cli_show_foundation.rs so each test file maps one feature (COV-075).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn write_foundation(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), raw).expect("write foundation.json");
}

fn write_foundation_draft(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), raw).expect("write foundation.json.draft");
}

fn dispatch_sf(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-foundation".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    };
    codelet_fspec_core::dispatch_command(req)
}

/// Minimal v2.0.0 foundation with project.name set.
fn minimal_foundation(name: &str) -> String {
    format!(
        r#"{{
  "schemaVersion": "2.0.0",
  "project": {{"name":"{name}","vision":"V","projectType":"cli-tool"}},
  "problemSpace": {{"primaryProblem":{{"title":"P","description":"D","impact":"I"}}}},
  "solutionSpace": {{"overview":"O","capabilities":[{{"name":"C","description":"D"}}]}},
  "personas": [{{"name":"User","description":"d"}}]
}}"#
    )
}

// =========================================================================
// Scenarios from show-foundation-rust-port.feature (dispatcher path)
// =========================================================================

#[test]
fn scenario_returns_text_render_with_project_and_other_sections() {
    // @step Given spec/foundation.json contains project.name='fspec', project.vision='V', project.projectType='cli-tool', a primary problem with title/description/impact, a solution overview with one capability, and one persona
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with no section and format='text'
    let result = dispatch_sf(ws.path(), r#"{"format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data contains the exact line '=== PROJECT ==='
    assert!(
        result.data.lines().any(|l| l == "=== PROJECT ==="),
        "data must contain '=== PROJECT ==='; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Name: fspec'
    assert!(
        result.data.lines().any(|l| l == "Name: fspec"),
        "data must contain 'Name: fspec'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Vision: V'
    assert!(
        result.data.lines().any(|l| l == "Vision: V"),
        "data must contain 'Vision: V'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Type: cli-tool'
    assert!(
        result.data.lines().any(|l| l == "Type: cli-tool"),
        "data must contain 'Type: cli-tool'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '=== PROBLEM SPACE ==='
    assert!(
        result.data.lines().any(|l| l == "=== PROBLEM SPACE ==="),
        "data must contain '=== PROBLEM SPACE ==='; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '=== SOLUTION SPACE ==='
    assert!(
        result.data.lines().any(|l| l == "=== SOLUTION SPACE ==="),
        "data must contain '=== SOLUTION SPACE ==='; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '=== PERSONAS ==='
    assert!(
        result.data.lines().any(|l| l == "=== PERSONAS ==="),
        "data must contain '=== PERSONAS ==='; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_returns_entire_foundation_as_pretty_json_when_format_json() {
    // @step Given spec/foundation.json contains a complete v2.0.0 foundation
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with no section and format='json'
    let result = dispatch_sf(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root has a 'project' field
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert!(
        parsed.get("project").is_some(),
        "data root must have 'project' field; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.contains("\n  "),
        "data must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_resolves_project_name_via_field_map_text_format() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'fspec'
    assert_eq!(
        result.data, "fspec",
        "data must equal 'fspec'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_resolves_project_name_via_field_map_json_format() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and format='json'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly '"fspec"'
    assert_eq!(
        result.data, "\"fspec\"",
        "data must equal '\"fspec\"'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_section_pointing_to_object_emits_pretty_json_in_text_format() {
    // @step Given spec/foundation.json contains project.name='fspec' and project.projectType='cli-tool'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='project' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"project","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root has 'name' and 'projectType'
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert!(
        parsed.get("name").is_some(),
        "data must have 'name' field; got:\n{}",
        result.data
    );
    assert!(
        parsed.get("projectType").is_some(),
        "data must have 'projectType' field; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.contains("\n  "),
        "data must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_missing_section_returns_field_not_found_error() {
    // @step Given spec/foundation.json contains a complete foundation
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='nonexistent' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"nonexistent","format":"text"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly "Field 'nonexistent' not found"
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Field 'nonexistent' not found"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_dotted_path_bypasses_field_map_for_unmapped_sections() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='project.name' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"project.name","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'fspec'
    assert_eq!(
        result.data, "fspec",
        "data must equal 'fspec'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_draft_true_with_no_draft_file_returns_missing_draft_error() {
    // @step Given spec/foundation.json.draft does NOT exist in the project root
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch show-foundation with draft=true
    let result = dispatch_sf(ws.path(), r#"{"draft":true}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one.'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result.error.as_deref().unwrap_or("").contains("No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one."),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_draft_true_reads_draft_file_instead_of_foundation() {
    // @step Given spec/foundation.json contains project.name='final-name'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("final-name"));

    // @step Given spec/foundation.json.draft contains project.name='draft-name'
    write_foundation_draft(ws.path(), &minimal_foundation("draft-name"));

    // @step When I dispatch show-foundation with section='projectName' and draft=true and format='text'
    let result = dispatch_sf(
        ws.path(),
        r#"{"section":"projectName","draft":true,"format":"text"}"#,
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'draft-name'
    assert_eq!(
        result.data, "draft-name",
        "data must equal 'draft-name'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_empty_workspace_auto_creates_foundation_json() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch show-foundation with section='projectName' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'Project Name'
    assert_eq!(
        result.data, "Project Name",
        "data must equal 'Project Name' (canonical default); got: {:?}",
        result.data
    );

    // @step Then spec/foundation.json exists after the call (auto-created by ensure_foundation_file)
    assert!(
        ws.path().join("spec/foundation.json").exists(),
        "spec/foundation.json must be auto-created"
    );
}

#[test]
fn scenario_escalates_malformed_foundation_json_dispatcher() {
    // @step Given spec/foundation.json exists but contains the malformed bytes '{ not json'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), "{ not json");

    // @step When I dispatch show-foundation against that project root
    let result = dispatch_sf(ws.path(), r#"{}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse foundation.json'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Failed to parse foundation.json"))
            .unwrap_or(false),
        "error must mention parse failure; got {result:?}"
    );
}

#[test]
fn scenario_output_writes_formatted_content_to_disk_via_dispatcher() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and format='text' and output='out/name.txt'
    let result = dispatch_sf(
        ws.path(),
        r#"{"section":"projectName","format":"text","output":"out/name.txt"}"#,
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the file <project_root>/out/name.txt exists with the exact bytes 'fspec'
    let written =
        fs::read_to_string(ws.path().join("out/name.txt")).expect("out/name.txt must exist");
    assert_eq!(
        written, "fspec",
        "file must contain exactly 'fspec'; got: {written:?}"
    );
}

#[test]
fn scenario_default_format_is_text_when_format_flag_omitted() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and no format flag
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'fspec'
    assert_eq!(
        result.data, "fspec",
        "data must equal 'fspec' (default format text); got: {:?}",
        result.data
    );
}
