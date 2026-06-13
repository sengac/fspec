// Feature: spec/features/list-attachments-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `list-attachments`
// (RPC-241). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Red phase: these tests MUST fail today because
// `codelet/fspec-core/src/commands/list_attachments.rs` is still a
// NotYetPorted stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "list-attachments".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

/// Build a work-units.json with a single work unit, optionally embedding an
/// `attachments` array via raw `serde_json::json!` so we don't depend on the
/// typed `WorkUnit.attachments` field (added in Phase C).
fn work_units_with_attachments(id: &str, attachments: Option<&[&str]>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".to_string(), json!(id));
    wu.insert("title".to_string(), json!(format!("title for {id}")));
    wu.insert("status".to_string(), json!("backlog"));
    wu.insert("createdAt".to_string(), json!("2026-06-01T00:00:00.000Z"));
    wu.insert("updatedAt".to_string(), json!("2026-06-01T00:00:00.000Z"));
    if let Some(att) = attachments {
        wu.insert(
            "attachments".to_string(),
            Value::Array(att.iter().map(|s| json!(s)).collect()),
        );
    }

    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), Value::Object(wu));

    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": {
            "backlog": [id], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

fn write_attachment_file(project_root: &Path, rel_path: &str, bytes: usize) -> PathBuf {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir attachment parent");
    }
    let content = vec![0u8; bytes];
    fs::write(&full, &content).expect("write attachment file");
    full
}

fn parse_data(data: &str) -> Value {
    serde_json::from_str(data)
        .unwrap_or_else(|e| panic!("dispatch data not valid JSON: {e}\n{data}"))
}

// ---------- scenarios ----------

#[test]
fn returns_structured_error_when_work_unit_does_not_exist() {
    // Scenario: Returns a structured error when the requested work unit does not exist

    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch list-attachments with workUnitId='AUTH-001' against that project root
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false for unknown work unit; got {result:?}"
    );

    // @step Then the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Work unit 'AUTH-001' does not exist"),
        "error message missing canonical substring: {msg}"
    );

    // @step Then spec/work-units.json was auto-created with an empty workUnits map
    let wu_path = tmp.path().join("spec/work-units.json");
    assert!(
        wu_path.exists(),
        "spec/work-units.json must be auto-created by ensure_work_units_file"
    );
    let raw = fs::read_to_string(&wu_path).expect("read work-units.json");
    let parsed: Value = serde_json::from_str(&raw).expect("auto-created file is valid JSON");
    let wus = parsed["workUnits"].as_object().expect("workUnits map");
    assert!(
        wus.is_empty(),
        "auto-created workUnits must be empty; got {wus:?}"
    );
}

#[test]
fn returns_empty_sentinel_when_attachments_field_is_missing() {
    // Scenario: Returns the empty-attachments sentinel when the attachments field is missing

    // @step Given spec/work-units.json contains AUTH-001 with NO attachments field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with_attachments("AUTH-001", None));

    // @step When I dispatch list-attachments with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data contains exactly the line "No attachments found for work unit AUTH-001"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "No attachments found for work unit AUTH-001"),
        "missing empty-sentinel line; got:\n{}",
        result.data
    );
}

#[test]
fn returns_empty_sentinel_when_attachments_array_is_empty() {
    // Scenario: Returns the empty-attachments sentinel when the attachments array is empty

    // @step Given spec/work-units.json contains AUTH-001 with attachments=[]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with_attachments("AUTH-001", Some(&[])),
    );

    // @step When I dispatch list-attachments with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data contains exactly the line "No attachments found for work unit AUTH-001"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "No attachments found for work unit AUTH-001"),
        "missing empty-sentinel line; got:\n{}",
        result.data
    );
}

#[test]
fn renders_present_attachment_with_size_and_modified_prefix() {
    // Scenario: Renders a present attachment with size and modified-line prefix

    // @step Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/diagram.png"]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with_attachments("AUTH-001", Some(&["spec/attachments/AUTH-001/diagram.png"])),
    );

    // @step Given the file spec/attachments/AUTH-001/diagram.png exists on disk with exactly 2048 bytes
    write_attachment_file(tmp.path(), "spec/attachments/AUTH-001/diagram.png", 2048);

    // @step When I dispatch list-attachments with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data contains the substring "Attachments for AUTH-001 (1):"
    assert!(
        result.data.contains("Attachments for AUTH-001 (1):"),
        "missing header substring; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "  ✓ spec/attachments/AUTH-001/diagram.png"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  ✓ spec/attachments/AUTH-001/diagram.png"),
        "missing exact present-marker line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "    Size: 2.00 KB"
    assert!(
        result.data.lines().any(|l| l == "    Size: 2.00 KB"),
        "missing exact Size line '    Size: 2.00 KB'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains a line starting with "    Modified: "
    assert!(
        result.data.lines().any(|l| l.starts_with("    Modified: ")),
        "missing a line starting with '    Modified: '; got:\n{}",
        result.data
    );
}

#[test]
fn renders_missing_attachment_with_x_marker_and_not_found_line() {
    // Scenario: Renders a missing attachment with the ✗ marker and the canonical not-found message

    // @step Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/missing.png"]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with_attachments("AUTH-001", Some(&["spec/attachments/AUTH-001/missing.png"])),
    );

    // @step Given no file exists at spec/attachments/AUTH-001/missing.png
    assert!(!tmp
        .path()
        .join("spec/attachments/AUTH-001/missing.png")
        .exists());

    // @step When I dispatch list-attachments with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data contains the exact line "  ✗ spec/attachments/AUTH-001/missing.png"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  ✗ spec/attachments/AUTH-001/missing.png"),
        "missing exact ✗-marker line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "    File not found on filesystem"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    File not found on filesystem"),
        "missing exact 'File not found on filesystem' line; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data does NOT contain the substring "Size:" for this entry
    assert!(
        !result.data.contains("Size:"),
        "must NOT print a Size: line for a missing attachment; got:\n{}",
        result.data
    );
}

#[test]
fn preserves_insertion_order_with_mixed_present_and_missing_markers() {
    // Scenario: Preserves attachment-array insertion order and mixes present/missing markers

    // @step Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/a.png","spec/attachments/AUTH-001/b.png"]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with_attachments(
            "AUTH-001",
            Some(&[
                "spec/attachments/AUTH-001/a.png",
                "spec/attachments/AUTH-001/b.png",
            ]),
        ),
    );

    // @step Given the file spec/attachments/AUTH-001/a.png exists on disk with exactly 1234 bytes
    write_attachment_file(tmp.path(), "spec/attachments/AUTH-001/a.png", 1234);

    // @step Given no file exists at spec/attachments/AUTH-001/b.png
    assert!(!tmp.path().join("spec/attachments/AUTH-001/b.png").exists());

    // @step When I dispatch list-attachments with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data contains the substring "Attachments for AUTH-001 (2):"
    assert!(
        result.data.contains("Attachments for AUTH-001 (2):"),
        "missing header substring 'Attachments for AUTH-001 (2):'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "  ✓ spec/attachments/AUTH-001/a.png"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  ✓ spec/attachments/AUTH-001/a.png"),
        "missing present-marker line for a.png; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "    Size: 1.21 KB"
    assert!(
        result.data.lines().any(|l| l == "    Size: 1.21 KB"),
        "missing exact Size line '    Size: 1.21 KB' (1234/1024 = 1.205... → 1.21); got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "  ✗ spec/attachments/AUTH-001/b.png"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  ✗ spec/attachments/AUTH-001/b.png"),
        "missing ✗-marker line for b.png; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line "    File not found on filesystem"
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "    File not found on filesystem"),
        "missing 'File not found on filesystem' line for b.png; got:\n{}",
        result.data
    );

    // @step Then the substring 'a.png' appears before 'b.png' in the DispatchResult.data
    let a = result.data.find("a.png").expect("a.png present in output");
    let b = result.data.find("b.png").expect("b.png present in output");
    assert!(
        a < b,
        "insertion order must be preserved (a.png before b.png); got a={a} b={b}\n{}",
        result.data
    );
}

#[test]
fn rejects_empty_args_object_with_invalid_args_error() {
    // Scenario: Rejects an empty arguments object with a structured InvalidArgs error

    // @step Given an empty project root directory
    let tmp = TempDir::new().expect("tempdir");

    // @step When I dispatch list-attachments with the empty JSON args object {}
    let result = dispatch_command(req(tmp.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false on empty args; got {result:?}"
    );

    // @step Then the error message names the missing field workUnitId
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("workUnitId"),
        "error message must name missing field 'workUnitId'; got: {msg}"
    );
}

#[test]
fn escalates_malformed_work_units_json_as_structured_parse_error() {
    // Scenario: Escalates malformed work-units.json as a structured parse error

    // @step Given spec/work-units.json exists but contains invalid JSON syntax
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), "{ not valid json");

    // @step When I dispatch list-attachments with workUnitId='AUTH-001'
    let result = dispatch_command(req(tmp.path(), json!({ "workUnitId": "AUTH-001" })));

    // @step Then the dispatcher returns success=false
    assert!(
        !result.success,
        "expected success=false on malformed work-units.json; got {result:?}"
    );

    // @step Then the error message contains the substring 'Failed to parse work-units.json'
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Failed to parse work-units.json"),
        "error message missing canonical parse-error substring; got: {msg}"
    );
}

#[test]
fn json_format_emits_two_space_indent_with_canonical_field_set() {
    // Scenario: JSON format emits two-space indented payload with the canonical field set

    // @step Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/x.png"]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with_attachments("AUTH-001", Some(&["spec/attachments/AUTH-001/x.png"])),
    );

    // @step Given the file spec/attachments/AUTH-001/x.png exists on disk with exactly 1024 bytes
    write_attachment_file(tmp.path(), "spec/attachments/AUTH-001/x.png", 1024);

    // @step When I dispatch list-attachments with workUnitId='AUTH-001' and format='json'
    let result = dispatch_command(req(
        tmp.path(),
        json!({ "workUnitId": "AUTH-001", "format": "json" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root object has workUnitId='AUTH-001'
    let data = parse_data(&result.data);
    assert_eq!(
        data["workUnitId"].as_str(),
        Some("AUTH-001"),
        "root.workUnitId must be 'AUTH-001'; got: {}",
        result.data
    );

    // @step Then the JSON root object has an attachments array of length 1
    let arr = data["attachments"]
        .as_array()
        .expect("attachments array on root");
    assert_eq!(arr.len(), 1, "attachments array length; got {arr:?}");

    // @step Then the first attachments entry has path='spec/attachments/AUTH-001/x.png', exists=true, and sizeKb='1.00'
    let entry = &arr[0];
    assert_eq!(
        entry["path"].as_str(),
        Some("spec/attachments/AUTH-001/x.png")
    );
    assert_eq!(entry["exists"].as_bool(), Some(true));
    assert_eq!(entry["sizeKb"].as_str(), Some("1.00"));

    // @step Then the first attachments entry has a non-empty modified string
    let modified = entry["modified"]
        .as_str()
        .expect("modified string on present entry");
    assert!(
        !modified.is_empty(),
        "modified must be non-empty; got: {modified}"
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result
            .data
            .lines()
            .any(|l| l.starts_with("  \"workUnitId\"") || l.starts_with("  \"attachments\"")),
        "expected 2-space-indented root fields; got:\n{}",
        result.data
    );
    assert!(
        result.data.lines().any(|l| l == "    {"),
        "expected 4-space-indented `{{` opening the attachments entry; got:\n{}",
        result.data
    );
}

#[test]
fn shared_infrastructure_and_ported_wiring_are_in_place() {
    // Scenario: Shared infrastructure and ported wiring are in place

    // @step Given the codelet/fspec-core crate is built
    // (precondition: this test only runs if the crate builds successfully)

    // @step When I inspect codelet/fspec-core/src/
    let crate_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step Then commands/list_attachments.rs does NOT return FspecCoreError::NotYetPorted
    let list_src = fs::read_to_string(crate_src.join("commands/list_attachments.rs"))
        .expect("commands/list_attachments.rs readable");
    assert!(
        !list_src.contains("FspecCoreError::NotYetPorted"),
        "commands/list_attachments.rs must no longer be a NotYetPorted stub"
    );

    // @step Then commands/list_attachments.rs delegates to ensure_work_units_file
    assert!(
        list_src.contains("ensure_work_units_file"),
        "commands/list_attachments.rs must delegate to ensure_work_units_file; got:\n{list_src}"
    );

    // @step Then commands/list_attachments.rs reads the attachments field via the WorkUnit extra map
    //
    // Architecture revision (Phase C): the supervisor opted to keep the
    // port isolated to commands/list_attachments.rs rather than promoting
    // `attachments` to a typed field on WorkUnit. The command reads the
    // field from `work_unit.extra` (the round-tripped flatten map) so no
    // shared-types change is required.
    assert!(
        list_src.contains("extra") && list_src.contains("\"attachments\""),
        "commands/list_attachments.rs must read the `attachments` key from the WorkUnit `extra` map; got:\n{list_src}"
    );

    // @step Then canonical.rs lists "list-attachments" in PORTED_COMMANDS
    let canonical_src =
        fs::read_to_string(crate_src.join("canonical.rs")).expect("canonical.rs readable");
    assert!(
        canonical_src.contains("\"list-attachments\""),
        "canonical.rs PORTED_COMMANDS must include \"list-attachments\"; got:\n{canonical_src}"
    );
}
