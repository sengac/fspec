// Feature: spec/features/add-attachment-rust-port.feature
//
// Validates the acceptance criteria for the Rust port of `add-attachment`
// (RPC-170, hardened by BUG-151). Each scenario maps to exactly one #[test]
// function with @step comments mirroring the Gherkin steps verbatim. The
// port is complete: the dispatcher routes `add-attachment` to
// `codelet/fspec-core/src/commands/add_attachment.rs::run`.

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
    write_file(
        tmp.path(),
        "spec/attachments/AUTH-001/diagram.png",
        b"existing",
    );

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
        v.get("workUnits").map(Value::is_object).unwrap_or(false),
        "expected workUnits object: {v}"
    );
}

#[test]
fn validates_mmd_attachment_and_rejects_invalid_mermaid_before_copy() {
    // Scenario: Validates a .mmd attachment and rejects invalid Mermaid before copy

    // @step Given a work unit AUTH-001 and a source file diagram.mmd containing invalid Mermaid
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));
    write_file(
        tmp.path(),
        "diagram.mmd",
        b"flowchart TD\n  A[Start --> B[Done",
    );

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' filePath='diagram.mmd'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "diagram.mmd"}),
    ));

    // @step Then the dispatcher returns an error containing 'Invalid Mermaid'
    assert!(!result.success, "expected failure; got {result:?}");
    let msg = result.error.as_ref().expect("error must be set");
    assert!(msg.contains("Invalid Mermaid"), "got: {msg}");

    // @step And no file is copied into spec/attachments/AUTH-001 and the work unit is unchanged
    assert!(
        !tmp.path().join("spec/attachments/AUTH-001").exists(),
        "no attachments dir should be created on validation failure"
    );
    let data = read_work_units(tmp.path());
    assert!(
        data["workUnits"]["AUTH-001"].get("attachments").is_none(),
        "work unit must remain unchanged"
    );
}

#[test]
fn validates_md_mermaid_fences_and_accepts_fence_free_markdown() {
    // Scenario: Validates mermaid fences inside a .md attachment and accepts fence-free markdown

    // @step Given a work unit AUTH-001 and a notes.md containing one valid and one invalid mermaid fence
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("AUTH-001", "specifying", None));
    write_file(
        tmp.path(),
        "notes.md",
        b"# Notes\n\n```mermaid\ngraph TD\n  A-->B\n```\n\n```mermaid\nflowchart TD\n  A[Start --> B\n```\n",
    );

    // @step When I dispatch add-attachment with workUnitId='AUTH-001' filePath='notes.md'
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "notes.md"}),
    ));

    // @step Then the dispatcher returns an error naming the failing code block
    assert!(!result.success, "expected failure; got {result:?}");
    let msg = result.error.as_ref().expect("error must be set");
    assert!(
        msg.contains("Mermaid code block 2 is invalid"),
        "got: {msg}"
    );

    // @step And a plain.md containing no mermaid fences is accepted and copied unchanged
    write_file(
        tmp.path(),
        "plain.md",
        b"# Just markdown\n\nNo diagrams here.\n",
    );
    let ok = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "AUTH-001", "filePath": "plain.md"}),
    ));
    assert!(
        ok.success,
        "fence-free markdown must be accepted; got {ok:?}"
    );
    let copied = tmp.path().join("spec/attachments/AUTH-001/plain.md");
    assert!(copied.exists(), "plain.md should be copied");
    assert_eq!(
        fs::read(&copied).expect("read copied"),
        b"# Just markdown\n\nNo diagrams here.\n",
        "copied markdown must be byte-identical"
    );
}

// ───────────────────────── BUG-151 scenarios ─────────────────────────
//
// Feature: spec/features/add-attachment-rust-port.feature (BUG-151
// scenarios, mirrored from the TS-side work-unit-attachments.feature)
//
// BUG-151: add-attachment truncates the source file to 0 bytes when it
// already lives in spec/attachments/<ID>/. std::fs::copy opens the
// destination with truncation, so copying a file onto itself destroys it.
// The fix: canonicalized source==destination guard (register-only path)
// and the duplicate-registration check moved BEFORE any filesystem
// mutation.

#[test]
fn bug_151_self_copy_registers_without_truncating_the_file() {
    // @step Given I have a work unit "TEST-001"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("TEST-001", "specifying", None));

    // @step And a file "spec/attachments/TEST-001/notes.md" with content "important research"
    let notes = write_file(
        tmp.path(),
        "spec/attachments/TEST-001/notes.md",
        b"important research",
    );

    // @step When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "TEST-001", "filePath": "spec/attachments/TEST-001/notes.md"}),
    ));

    // @step Then the command should succeed
    assert!(result.success, "expected success; got {result:?}");

    // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    let content = fs::read(&notes).expect("read notes.md after add-attachment");
    assert_eq!(
        content,
        b"important research",
        "BUG-151: self-copy must NOT truncate the file (got {} bytes)",
        content.len()
    );

    // @step And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["TEST-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(
        arr.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        vec!["spec/attachments/TEST-001/notes.md"],
    );
}

#[test]
fn bug_151_duplicate_registration_rejected_without_touching_the_file() {
    // @step Given I have a work unit "TEST-001" with attachment "spec/attachments/TEST-001/notes.md" containing "important research"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "TEST-001",
            "specifying",
            Some(&["spec/attachments/TEST-001/notes.md"]),
        ),
    );
    let notes = write_file(
        tmp.path(),
        "spec/attachments/TEST-001/notes.md",
        b"important research",
    );
    let json_before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001" again
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "TEST-001", "filePath": "spec/attachments/TEST-001/notes.md"}),
    ));

    // @step Then the command should fail with an "already exists" error
    assert!(!result.success, "expected error; got {result:?}");
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Attachment 'notes.md' already exists for work unit 'TEST-001'"),
        "error: {msg}"
    );

    // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    let content = fs::read(&notes).expect("read notes.md after duplicate add");
    assert_eq!(
        content,
        b"important research",
        "BUG-151: duplicate registration must NOT touch the file (got {} bytes)",
        content.len()
    );

    // work-units.json byte-equal to its pre-call state
    let json_after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(json_before, json_after, "work-units.json must NOT change");
}

#[test]
fn bug_151_attachments_root_source_still_moved_into_work_unit_directory() {
    // @step Given I have a work unit "TEST-001"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("TEST-001", "specifying", None));

    // @step And a file "spec/attachments/analysis.md" with content "root analysis"
    write_file(tmp.path(), "spec/attachments/analysis.md", b"root analysis");

    // @step When I add the attachment "spec/attachments/analysis.md" to work unit "TEST-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "TEST-001", "filePath": "spec/attachments/analysis.md"}),
    ));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then the file should exist at "spec/attachments/TEST-001/analysis.md" with content "root analysis"
    let moved = tmp.path().join("spec/attachments/TEST-001/analysis.md");
    assert_eq!(
        fs::read(&moved).expect("read moved file"),
        b"root analysis",
        "BUG-055: root file must be copied into the per-WU directory"
    );

    // @step And the file "spec/attachments/analysis.md" should no longer exist
    assert!(
        !tmp.path().join("spec/attachments/analysis.md").exists(),
        "BUG-055: root copy must be removed"
    );

    // @step And the work unit should track "spec/attachments/TEST-001/analysis.md" as an attachment
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["TEST-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(
        arr.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        vec!["spec/attachments/TEST-001/analysis.md"],
    );
}

#[cfg(unix)]
#[test]
fn bug_151_symlink_alias_of_destination_does_not_truncate_it() {
    // @step Given I have a work unit "TEST-001"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("TEST-001", "specifying", None));

    // @step And a file "spec/attachments/TEST-001/notes.md" with content "important research"
    let notes = write_file(
        tmp.path(),
        "spec/attachments/TEST-001/notes.md",
        b"important research",
    );

    // @step And a symlink outside the attachments directory pointing at "spec/attachments/TEST-001/notes.md"
    let alias_dir = tmp.path().join("alias");
    fs::create_dir_all(&alias_dir).expect("mkdir alias");
    let alias = alias_dir.join("notes.md");
    std::os::unix::fs::symlink(&notes, &alias).expect("create symlink");

    // @step When I add the attachment via the symlink path to work unit "TEST-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "TEST-001", "filePath": "alias/notes.md"}),
    ));
    assert!(result.success, "expected success; got {result:?}");

    // @step Then the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    let content = fs::read(&notes).expect("read notes.md after symlink add");
    assert_eq!(
        content,
        b"important research",
        "BUG-151: symlink alias must NOT truncate the destination (got {} bytes)",
        content.len()
    );

    // @step And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["TEST-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(
        arr.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        vec!["spec/attachments/TEST-001/notes.md"],
    );
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn bug_151_read_only_self_source_registers_without_attempting_a_copy() {
    // @step Given I have a work unit "TEST-001"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(tmp.path(), &work_units_with("TEST-001", "specifying", None));

    // @step And a read-only file "spec/attachments/TEST-001/notes.md" with content "important research"
    let notes = write_file(
        tmp.path(),
        "spec/attachments/TEST-001/notes.md",
        b"important research",
    );
    let mut perms = fs::metadata(&notes).expect("metadata").permissions();
    perms.set_readonly(true);
    fs::set_permissions(&notes, perms).expect("set read-only");
    assert!(
        fs::metadata(&notes)
            .expect("metadata")
            .permissions()
            .readonly(),
        "precondition: notes.md must be read-only"
    );

    // @step When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "TEST-001", "filePath": "spec/attachments/TEST-001/notes.md"}),
    ));

    // @step Then the command should succeed
    // Meaningful guard proof: an attempted std::fs::copy onto the read-only
    // self would fail with a permission error (dest opened for write), so
    // success here proves the register-only path skipped the copy entirely.
    assert!(
        result.success,
        "register-only path must not attempt a copy onto the read-only self; got {result:?}"
    );

    // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    let content = fs::read(&notes).expect("read notes.md after add-attachment");
    assert_eq!(
        content,
        b"important research",
        "BUG-151: read-only self-source must NOT be truncated (got {} bytes)",
        content.len()
    );
    assert!(
        fs::metadata(&notes)
            .expect("metadata")
            .permissions()
            .readonly(),
        "file permissions must be untouched (still read-only)"
    );

    // @step And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
    let on_disk = read_work_units(tmp.path());
    let arr = on_disk["workUnits"]["TEST-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(
        arr.iter().filter_map(Value::as_str).collect::<Vec<_>>(),
        vec!["spec/attachments/TEST-001/notes.md"],
    );

    // Restore writability so TempDir cleanup succeeds on all platforms.
    let mut perms = fs::metadata(&notes).expect("metadata").permissions();
    perms.set_readonly(false);
    fs::set_permissions(&notes, perms).expect("restore writable");
}

#[test]
fn bug_151_duplicate_from_different_source_does_not_overwrite_registered_attachment() {
    // @step Given I have a work unit "TEST-001" with attachment "spec/attachments/TEST-001/notes.md" containing "important research"
    let tmp = TempDir::new().expect("tempdir");
    write_work_units(
        tmp.path(),
        &work_units_with(
            "TEST-001",
            "specifying",
            Some(&["spec/attachments/TEST-001/notes.md"]),
        ),
    );
    let notes = write_file(
        tmp.path(),
        "spec/attachments/TEST-001/notes.md",
        b"important research",
    );
    let json_before = fs::read(work_units_path(tmp.path())).expect("read pre");

    // @step And a file "other/notes.md" with content "different content"
    write_file(tmp.path(), "other/notes.md", b"different content");

    // @step When I add the attachment "other/notes.md" to work unit "TEST-001"
    let result = dispatch_command(req(
        tmp.path(),
        json!({"workUnitId": "TEST-001", "filePath": "other/notes.md"}),
    ));

    // @step Then the command should fail with an "already exists" error
    assert!(!result.success, "expected error; got {result:?}");
    let msg = result.error.as_ref().expect("error");
    assert!(
        msg.contains("Attachment 'notes.md' already exists for work unit 'TEST-001'"),
        "error: {msg}"
    );

    // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
    // (i.e. the registered attachment was NOT overwritten by "different content")
    let content = fs::read(&notes).expect("read notes.md after duplicate add");
    assert_eq!(
        content,
        b"important research",
        "BUG-151: duplicate guard must fire BEFORE the copy — the registered \
         attachment must NOT be overwritten by the different source"
    );

    // @step And the work unit's registered attachments must be unchanged on disk
    let json_after = fs::read(work_units_path(tmp.path())).expect("read post");
    assert_eq!(json_before, json_after, "work-units.json must NOT change");
}
