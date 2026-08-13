// Feature: spec/features/remove-attachment-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `remove-attachment`
// (RPC-268). Each scenario maps to exactly one #[test] function with @step
// comments mirroring the Gherkin steps verbatim.
//
// Red phase: these tests MUST fail today because
// `rust/fspec-core/src/commands/remove_attachment.rs` is still a
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
        command: "remove-attachment".to_string(),
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

fn write_file(project_root: &Path, rel_path: &str, bytes: &[u8]) -> PathBuf {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&full, bytes).expect("write file");
    full
}

// ───────────────────────── scenarios ─────────────────────────

#[test]
fn removing_only_attachment_empties_array_and_deletes_file_from_disk() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/diagram.png"]),
        ),
    );

    // @step And the file spec/attachments/AUTH-001/diagram.png exists on disk with 1024 bytes
    write_file(
        tmp.path(),
        "spec/attachments/AUTH-001/diagram.png",
        &vec![0u8; 1024],
    );

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='diagram.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "diagram.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output contains the substring '✓ Attachment removed from work unit and file deleted'
    assert!(
        result
            .data
            .contains("✓ Attachment removed from work unit and file deleted"),
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

    // @step And spec/attachments/AUTH-001/diagram.png NO LONGER exists on disk
    assert!(
        !tmp.path()
            .join("spec/attachments/AUTH-001/diagram.png")
            .exists(),
        "file must be unlinked"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=[]
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert!(arr.is_empty(), "attachments must be empty; got {arr:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp
    let updated = on_disk["workUnits"]["AUTH-001"]["updatedAt"]
        .as_str()
        .expect("updatedAt");
    assert_ne!(updated, "2026-06-01T00:00:00.000Z");
    assert!(
        updated.ends_with('Z') && updated.contains('T'),
        "ISO-8601: {updated}"
    );
}

#[test]
fn missing_work_unit_surfaces_canonical_error() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    let before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I dispatch remove-attachment with workUnitId='ZZZ-999' and fileName='diagram.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "ZZZ-999", "fileName": "diagram.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Work unit 'ZZZ-999' does not exist"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Work unit 'ZZZ-999' does not exist"),
        "error: {msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(before, after, "work-units.json must NOT be modified");
}

#[test]
fn work_unit_with_no_attachments_field_surfaces_no_attachments_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no attachments field
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));

    let before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='whatever.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "whatever.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' has no attachments to remove"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Work unit 'AUTH-001' has no attachments to remove"),
        "error: {msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(before, after, "work-units.json must NOT be modified");
}

#[test]
fn work_unit_with_empty_attachments_array_surfaces_no_attachments_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with attachments=[]
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with("AUTH-001", "specifying", Some(&[])),
    );

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='whatever.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "whatever.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Work unit 'AUTH-001' has no attachments to remove"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Work unit 'AUTH-001' has no attachments to remove"),
        "error: {msg}"
    );
}

#[test]
fn unknown_filename_surfaces_not_found_error() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/diagram.png"]),
        ),
    );

    let before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='nonexistent.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "nonexistent.png"}),
    ));

    // @step Then the dispatcher returns an error
    assert!(!result.success, "expected error; got {result:?}");

    // @step And the error message contains the substring "Attachment 'nonexistent.png' not found for work unit 'AUTH-001'"
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Attachment 'nonexistent.png' not found for work unit 'AUTH-001'"),
        "error: {msg}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(before, after, "work-units.json must NOT be modified");
}

#[test]
fn missing_on_disk_file_degrades_gracefully_to_already_missing_warning() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/ghost.png']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/ghost.png"]),
        ),
    );

    // @step And NO file exists at spec/attachments/AUTH-001/ghost.png on disk
    assert!(!tmp
        .path()
        .join("spec/attachments/AUTH-001/ghost.png")
        .exists());

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='ghost.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "ghost.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output contains the substring '⚠ Attachment removed from work unit (file was already missing)'
    assert!(
        result
            .data
            .contains("⚠ Attachment removed from work unit (file was already missing)"),
        "data: {}",
        result.data
    );

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=[]
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert!(arr.is_empty(), "attachments must be empty; got {arr:?}");
}

#[test]
fn keep_file_preserves_file_on_disk_but_removes_array_entry() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/keep.pdf']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/keep.pdf"]),
        ),
    );

    // @step And the file spec/attachments/AUTH-001/keep.pdf exists on disk
    write_file(
        tmp.path(),
        "spec/attachments/AUTH-001/keep.pdf",
        b"pdf-bytes",
    );

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' fileName='keep.pdf' keepFile=true
    let result = dispatch_command(req(
        tmp.path(),
        json!({
            "workUnitId": "AUTH-001",
            "fileName": "keep.pdf",
            "keepFile": true,
        }),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And the rendered output contains the substring '✓ Attachment removed from work unit (file kept)'
    assert!(
        result
            .data
            .contains("✓ Attachment removed from work unit (file kept)"),
        "data: {}",
        result.data
    );

    // @step And spec/attachments/AUTH-001/keep.pdf STILL exists on disk
    assert!(
        tmp.path()
            .join("spec/attachments/AUTH-001/keep.pdf")
            .exists(),
        "--keep-file must preserve the file on disk"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=[]
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert!(arr.is_empty(), "attachments must be empty; got {arr:?}");
}

#[test]
fn removing_middle_of_three_attachments_preserves_order_of_remaining_two() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png','spec/attachments/AUTH-001/c.png']
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&[
                "spec/attachments/AUTH-001/a.png",
                "spec/attachments/AUTH-001/b.png",
                "spec/attachments/AUTH-001/c.png",
            ]),
        ),
    );

    // @step And the files for all three attachments exist on disk
    write_file(tmp.path(), "spec/attachments/AUTH-001/a.png", b"a");
    write_file(tmp.path(), "spec/attachments/AUTH-001/b.png", b"b");
    write_file(tmp.path(), "spec/attachments/AUTH-001/c.png", b"c");

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='b.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "b.png"}),
    ));

    // @step Then the dispatcher returns success
    assert!(result.success, "expected success; got {result:?}");

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/c.png']
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    let paths: Vec<&str> = arr.iter().map(|v| v.as_str().unwrap_or("")).collect();
    assert_eq!(
        paths,
        vec![
            "spec/attachments/AUTH-001/a.png",
            "spec/attachments/AUTH-001/c.png",
        ],
        "remove-middle preserves order of remaining two"
    );

    // @step And spec/attachments/AUTH-001/a.png and spec/attachments/AUTH-001/c.png both STILL exist on disk
    assert!(tmp.path().join("spec/attachments/AUTH-001/a.png").exists());
    assert!(tmp.path().join("spec/attachments/AUTH-001/c.png").exists());

    // @step And spec/attachments/AUTH-001/b.png NO LONGER exists on disk
    assert!(!tmp.path().join("spec/attachments/AUTH-001/b.png").exists());
}

#[test]
fn auto_creates_work_units_json_when_missing_then_reports_missing_work_unit_error() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let tmp = TempDir::new().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I dispatch remove-attachment with workUnitId='AUTH-001' and fileName='whatever.png'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "fileName": "whatever.png"}),
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
        v.get("workUnits").map(Value::is_object).unwrap_or(false),
        "expected workUnits object: {v}"
    );
}
