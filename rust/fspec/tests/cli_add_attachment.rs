//! CLI surface for the `add-attachment` subcommand on the standalone fspec
//! Rust binary — RPC-170.
//!
//! Feature: spec/features/add-attachment-cli-subcommand.feature
//!
//! The port is complete: `rust/fspec/src/main.rs` registers the
//! `add-attachment` clap subcommand, which bridges into
//! `rust/fspec-core/src/commands/add_attachment.rs::run`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────────────────────── helpers ─────────────────────────

fn run_add_attachment(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-attachment");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-attachment");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn work_units_with(id: &str, status: &str, attachments: Option<&[&str]>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".into(), serde_json::json!(id));
    wu.insert("title".into(), serde_json::json!(format!("title for {id}")));
    wu.insert("type".into(), serde_json::json!("story"));
    wu.insert("status".into(), serde_json::json!(status));
    wu.insert(
        "createdAt".into(),
        serde_json::json!("2026-06-01T00:00:00.000Z"),
    );
    wu.insert(
        "updatedAt".into(),
        serde_json::json!("2026-06-01T00:00:00.000Z"),
    );
    if let Some(att) = attachments {
        wu.insert(
            "attachments".into(),
            serde_json::Value::Array(att.iter().map(|s| serde_json::json!(s)).collect()),
        );
    }

    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), serde_json::Value::Object(wu));

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
            vec![serde_json::json!(id)]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), serde_json::Value::Array(arr));
    }

    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": serde_json::Value::Object(wus),
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

fn write_file(project_root: &Path, rel_path: &str, bytes: &[u8]) {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&full, bytes).expect("write file");
}

const TS_HELP_FIXTURE_AA: &str = include_str!("fixtures/help/add-attachment.txt");

// ───────────────────────── scenarios ─────────────────────────

#[test]
fn scenario_add_attachment_help_matches_ts_fixture_byte_for_byte() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `fspec add-attachment --help` piped to non-TTY (no color codes)
    let output = Command::new(fspec_bin())
        .arg("add-attachment")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-attachment --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "fspec add-attachment --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-attachment.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AA);

    // @step And stdout starts with a blank line followed by 'ADD-ATTACHMENT'
    assert!(
        stdout.starts_with("\nADD-ATTACHMENT\n"),
        "help must start with blank line then ADD-ATTACHMENT header; got:\n{stdout}"
    );

    // @step And stdout contains the section header 'USAGE' followed by '  fspec add-attachment <workUnitId> <filePath> [options]'
    assert!(
        stdout.contains("USAGE\n  fspec add-attachment <workUnitId> <filePath> [options]\n"),
        "help must contain USAGE then canonical signature; got:\n{stdout}"
    );

    // @step And stdout contains the section header 'ARGUMENTS'
    assert!(
        stdout.contains("ARGUMENTS\n"),
        "help must contain ARGUMENTS"
    );

    // @step And stdout contains the section header 'OPTIONS'
    assert!(stdout.contains("OPTIONS\n"), "help must contain OPTIONS");

    // @step And stdout contains the substring 'Fix: undefined'
    assert!(
        stdout.contains("Fix: undefined"),
        "help must reproduce TS bug 'Fix: undefined'; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_successfully_adds_attachment_and_prints_canonical_success_block() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file diagram.png exists in the tempdir
    write_file(ws.path(), "diagram.png", b"PNG-bytes");

    // @step When I run `fspec add-attachment AUTH-001 diagram.png` in that tempdir
    let (code, stdout, stderr) = run_add_attachment(ws.path(), &["AUTH-001", "diagram.png"]);

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "fspec add-attachment must exit 0; stdout={stdout}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Attachment added successfully'
    assert!(
        stdout.contains("✓ Attachment added successfully"),
        "stdout: {stdout}"
    );

    // @step And stdout contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    assert!(
        stdout.contains("  File: spec/attachments/AUTH-001/diagram.png"),
        "stdout: {stdout}"
    );

    // @step And spec/attachments/AUTH-001/diagram.png exists on disk
    assert!(
        ws.path()
            .join("spec/attachments/AUTH-001/diagram.png")
            .exists(),
        "copied file must exist"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=['spec/attachments/AUTH-001/diagram.png']
    let raw = fs::read_to_string(ws.path().join("spec/work-units.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = v["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0].as_str(),
        Some("spec/attachments/AUTH-001/diagram.png")
    );
}

#[test]
fn scenario_cli_passes_description_through_to_rendered_output() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file diagram.png exists in the tempdir
    write_file(ws.path(), "diagram.png", b"data");

    // @step When I run `fspec add-attachment AUTH-001 diagram.png --description "Auth flow v2"` in that tempdir
    let (code, stdout, stderr) = run_add_attachment(
        ws.path(),
        &["AUTH-001", "diagram.png", "--description", "Auth flow v2"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit code; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains the substring '  Description: Auth flow v2'
    assert!(
        stdout.contains("  Description: Auth flow v2"),
        "stdout: {stdout}"
    );
}

#[test]
fn scenario_cli_exits_2_when_work_unit_id_positional_missing() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec add-attachment` (no positionals) from that directory
    let (code, _stdout, stderr) = run_add_attachment(ws.path(), &[]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec add-attachment (no positionals) must exit 1 (Commander usage error parity); stderr={stderr}"
    );

    // @step And stderr names the missing required argument
    assert!(
        stderr.contains("WORK_UNIT_ID")
            || stderr.contains("work_unit_id")
            || stderr.contains("workUnitId")
            || stderr.contains("required"),
        "stderr must name the missing required argument; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_2_when_file_path_positional_missing() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec add-attachment AUTH-001` (missing second positional) from that directory
    let (code, _stdout, stderr) = run_add_attachment(ws.path(), &["AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec add-attachment AUTH-001 (no second positional) must exit 1 (Commander parity); stderr={stderr}"
    );

    // @step And stderr names the missing required argument
    assert!(
        stderr.contains("FILE_PATH")
            || stderr.contains("file_path")
            || stderr.contains("filePath")
            || stderr.contains("required"),
        "stderr must name the missing required argument; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_with_stderr_prefix_when_work_unit_does_not_exist() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file diagram.png exists
    write_file(ws.path(), "diagram.png", b"data");

    // @step When I run `fspec add-attachment ZZZ-999 diagram.png` in that tempdir
    let (code, stdout, stderr) = run_add_attachment(ws.path(), &["ZZZ-999", "diagram.png"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec add-attachment ZZZ-999 must exit 1; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the exact line "Error: Work unit 'ZZZ-999' does not exist"
    assert!(
        stderr
            .lines()
            .any(|l| l == "Error: Work unit 'ZZZ-999' does not exist"),
        "stderr must contain canonical 'Error:' line; got:\n{stderr}"
    );

    // @step And stderr does NOT contain the substring 'Invalid args for fspec command'
    assert!(
        !stderr.contains("Invalid args for fspec command"),
        "stderr must NOT contain the InvalidArgs wrapper text; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_when_source_file_does_not_exist() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And no file exists at ./missing.png
    assert!(!ws.path().join("missing.png").exists());

    // @step When I run `fspec add-attachment AUTH-001 ./missing.png` in that tempdir
    let (code, stdout, stderr) = run_add_attachment(ws.path(), &["AUTH-001", "./missing.png"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec add-attachment must exit 1 on missing source; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring "Error: Source file './missing.png' does not exist"
    assert!(
        stderr.contains("Error: Source file './missing.png' does not exist"),
        "stderr must contain canonical missing-source line; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_on_duplicate_attachment_with_byte_equality_on_json_file() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/diagram.png']
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/diagram.png"]),
        ),
    );

    // @step And the file spec/attachments/AUTH-001/diagram.png already exists
    write_file(
        ws.path(),
        "spec/attachments/AUTH-001/diagram.png",
        b"existing",
    );

    // @step And a source file diagram.png exists at the tempdir root
    write_file(ws.path(), "diagram.png", b"new-source");

    let wu_path = ws.path().join("spec/work-units.json");
    let before = fs::read(&wu_path).expect("read pre");

    // @step When I run `fspec add-attachment AUTH-001 diagram.png` in that tempdir
    let (code, stdout, stderr) = run_add_attachment(ws.path(), &["AUTH-001", "diagram.png"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec add-attachment on duplicate must exit 1; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring "Error: Attachment 'diagram.png' already exists for work unit 'AUTH-001'"
    assert!(
        stderr.contains("Error: Attachment 'diagram.png' already exists for work unit 'AUTH-001'"),
        "stderr must contain canonical duplicate error; got:\n{stderr}"
    );

    // @step And spec/work-units.json on disk is byte-equal to its pre-call contents
    let after = fs::read(&wu_path).expect("read post");
    assert_eq!(before, after, "spec/work-units.json must NOT be modified");
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_add_attachment() {
    // @step Given the fspec Rust binary has add-attachment registered as a clap subcommand

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(code, 0, "fspec --help must exit 0; stderr={stderr}");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists add-attachment as an available subcommand
    assert!(
        help.contains("add-attachment"),
        "fspec --help must list 'add-attachment' subcommand; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step And a source file diagram.png exists
    write_file(ws.path(), "diagram.png", b"data");

    // @step When I dispatch add-attachment via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' filePath='diagram.png'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-attachment".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","filePath":"diagram.png"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success
    assert!(result.success, "dispatcher must succeed; got {result:?}");

    // @step And the CLI bridge module rust/fspec/src/add_attachment.rs contains NO file copy, work-unit lookup, or atomic write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_attachment.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/add_attachment.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    let stripped = common::strip_comments(&bridge_src);
    for forbidden in [
        "copy_file",
        "fs::copy",
        "ensure_work_units_file",
        "write_json_atomic",
        "attachments_dir",
        "Work unit '",
    ] {
        assert!(
            !stripped.contains(forbidden),
            "CLI bridge must NOT contain domain logic substring {forbidden:?}; got:\n{stripped}"
        );
    }
}
