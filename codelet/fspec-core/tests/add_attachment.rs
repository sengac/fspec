// Feature: spec/features/add-attachment-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-attachment`
// (RPC-170). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Red phase: these tests MUST fail today because
// `codelet/fspec-core/src/commands/add_attachment.rs` is still a
// NotYetPorted stub and the dispatcher routes the command through
// `run_stub` rather than `run_ported`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::{Path, PathBuf};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ───────────────────────── helpers ─────────────────────────

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "add-attachment".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

fn work_units_path(project_root: &Path) -> PathBuf {
    project_root.join("spec").join("work-units.json")
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(project_root: &Path) -> Value {
    let raw = fs::read_to_string(work_units_path(project_root)).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

/// Build a single-work-unit JSON document seeded with optional `attachments`.
fn work_units_with(id: &str, status: &str, attachments: Option<&[&str]>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), json!(id));
    wu.insert("title".into(), json!(format!("title for {id}")));
    wu.insert("type".into(), json!("story"));
    wu.insert("status".into(), json!(status));
    wu.insert("createdAt".into(), json!("2026-06-01T00:00:00.000Z"));
    wu.insert("updatedAt".into(), json!("2026-06-01T00:00:00.000Z"));
    if let Some(att) = attachments {
        wu.insert(
            "attachments".into(),
            Value::Array(att.iter().map(|s| json!(s)).collect()),
        );
    }

    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), Value::Object(wu));

    let mut states = serde_json::Map::new();
    for st in &[
        "backlog",
        "specifying",
        "testing",
        "implementing",
        "validating",
        "done",
        "blocked",
    ] {
        let arr = if *st == status {
            vec![json!(id)]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), Value::Array(arr));
    }

    serde_json::to_string_pretty(&json!({
        "version": "0.7.1",
        "workUnits": Value::Object(wus),
        "states": Value::Object(states),
    }))
    .unwrap()
}

/// Write a deterministic non-empty file at `project_root/rel_path` and
/// return the absolute path.
fn write_file(project_root: &Path, rel_path: &str, bytes: &[u8]) -> PathBuf {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&full, bytes).expect("write source file");
    full
}

// ───────────────────────── scenarios ─────────────────────────

#[test]
fn adds_first_attachment_creates_dir_and_bumps_updated_at() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file ./diagram.png exists with non-empty bytes
    let bytes = b"PNGdata-non-empty";
    write_file(tmp.path(), "diagram.png", bytes);

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "./diagram.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output contains the substring '✓ Attachment added successfully'
    assert!(
        result.data.contains("✓ Attachment added successfully"),
        "data: {}",
        result.data
    );

    // @step And the rendered output contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    assert!(
        result
            .data
            .contains("  File: spec/attachments/AUTH-001/diagram.png"),
        "data: {}",
        result.data
    );

    // @step And spec/attachments/AUTH-001/diagram.png exists on disk with the same bytes as the source
    let dest = tmp.path().join("spec/attachments/AUTH-001/diagram.png");
    assert!(dest.exists(), "copied file must exist: {}", dest.display());
    let copied = fs::read(&dest).expect("read copied file");
    assert_eq!(copied, bytes, "copied bytes must match source");

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/diagram.png']
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(arr.len(), 1, "expected 1 attachment; got {arr:?}");
    assert_eq!(
        arr[0].as_str(),
        Some("spec/attachments/AUTH-001/diagram.png"),
        "attachments[0]"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z", "updatedAt must bump");
    assert!(
        updated.ends_with('Z') && updated.contains('T'),
        "updatedAt looks like ISO-8601: {updated}"
    );
}

#[test]
fn description_echoed_on_third_output_line_when_provided() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file ./diagram.png exists
    write_file(tmp.path(), "diagram.png", b"data");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' filePath='./diagram.png' description='Auth flow diagram v2'
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "filePath": "./diagram.png",
            "description": "Auth flow diagram v2",
        }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output contains the substring '  Description: Auth flow diagram v2'
    assert!(
        result.data.contains("  Description: Auth flow diagram v2"),
        "data: {}",
        result.data
    );
}

#[test]
fn description_omitted_from_output_when_not_provided() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file ./diagram.png exists
    write_file(tmp.path(), "diagram.png", b"data");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png' (no description)
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "./diagram.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output does NOT contain the substring 'Description:'
    assert!(
        !result.data.contains("Description:"),
        "must NOT contain 'Description:'; got: {}",
        result.data
    );
}

#[test]
fn missing_work_unit_surfaces_canonical_error_and_no_file_copy_occurs() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file ./diagram.png exists
    write_file(tmp.path(), "diagram.png", b"data");

    let before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I dispatch add-attachment with workUnitId='ZZZ-999' and filePath='./diagram.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "ZZZ-999", "filePath": "./diagram.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Work unit 'ZZZ-999' does not exist"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Work unit 'ZZZ-999' does not exist"),
        "error: {msg}"
    );

    // @step And spec/attachments/ZZZ-999/ does NOT exist on disk
    assert!(
        !tmp.path().join("spec/attachments/ZZZ-999").exists(),
        "spec/attachments/ZZZ-999 must NOT exist"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(before, after, "work-units.json must NOT be modified");
}

#[test]
fn missing_source_file_surfaces_canonical_error_using_original_caller_supplied_path() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And no file exists at ./does-not-exist.png
    assert!(!tmp.path().join("does-not-exist.png").exists());

    let before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./does-not-exist.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "./does-not-exist.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Source file './does-not-exist.png' does not exist"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Source file './does-not-exist.png' does not exist"),
        "error must echo ORIGINAL path './does-not-exist.png'; got: {msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(before, after, "work-units.json must NOT be modified");
}

#[test]
fn re_adding_same_file_surfaces_duplicate_attachment_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with attachments=['spec/attachments/AUTH-001/diagram.png']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/diagram.png"]),
        ),
    );

    // @step And the file spec/attachments/AUTH-001/diagram.png already exists on disk
    write_file(tmp.path(), "spec/attachments/AUTH-001/diagram.png", b"existing");

    // @step And a source file ./diagram.png exists
    write_file(tmp.path(), "diagram.png", b"new-source");

    let before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "./diagram.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Attachment 'diagram.png' already exists for work unit 'AUTH-001'"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Attachment 'diagram.png' already exists for work unit 'AUTH-001'"),
        "error: {msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(before, after, "work-units.json must NOT be modified");
}

#[test]
fn bug_055_dedup_unlinks_source_when_directly_in_spec_attachments_root() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file spec/attachments/foo.png exists (placed directly in the spec/attachments root)
    write_file(tmp.path(), "spec/attachments/foo.png", b"foo-bytes");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='spec/attachments/foo.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "spec/attachments/foo.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And spec/attachments/AUTH-001/foo.png exists on disk
    assert!(
        tmp.path()
            .join("spec/attachments/AUTH-001/foo.png")
            .exists(),
        "copied file must exist at per-WU directory"
    );

    // @step And spec/attachments/foo.png NO LONGER exists on disk (source was deleted after copy)
    assert!(
        !tmp.path().join("spec/attachments/foo.png").exists(),
        "BUG-055: source file at spec/attachments root must be unlinked"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/foo.png']
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(arr.len(), 1, "expected 1 attachment; got {arr:?}");
    assert_eq!(
        arr[0].as_str(),
        Some("spec/attachments/AUTH-001/foo.png"),
        "attachments[0]"
    );
}

#[test]
fn adding_third_attachment_preserves_existing_two_and_appends_in_array_order() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&[
                "spec/attachments/AUTH-001/a.png",
                "spec/attachments/AUTH-001/b.png",
            ]),
        ),
    );
    // Pre-populate the previously-existing files (not strictly required by the
    // command but reflects realistic state):
    write_file(tmp.path(), "spec/attachments/AUTH-001/a.png", b"a");
    write_file(tmp.path(), "spec/attachments/AUTH-001/b.png", b"b");

    // @step And a source file ./c.png exists
    write_file(tmp.path(), "c.png", b"cccc");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./c.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "./c.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png','spec/attachments/AUTH-001/c.png']
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    let paths: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(
        paths,
        vec![
            "spec/attachments/AUTH-001/a.png",
            "spec/attachments/AUTH-001/b.png",
            "spec/attachments/AUTH-001/c.png",
        ],
        "append-in-array-order parity"
    );
}

#[test]
fn auto_creates_work_units_json_when_missing_then_reports_missing_work_unit_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step And a source file ./diagram.png exists
    write_file(tmp.path(), "diagram.png", b"d");

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' and filePath='./diagram.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "./diagram.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Work unit 'AUTH-001' does not exist"),
        "error: {msg}"
    );

    // @step And spec/work-units.json now exists on disk with the canonical empty initial structure
    let p = work_units_path(tmp.path());
    assert!(p.exists(), "spec/work-units.json must be auto-created");
    let v = read_work_units(tmp.path());
    assert!(
        v.get("workUnits").map(|w| w.is_object()).unwrap_or(false),
        "expected workUnits object: {v}"
    );
}
