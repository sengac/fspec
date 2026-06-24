//! CLI surface for the `remove-attachment` subcommand on the standalone fspec
//! Rust binary — RPC-268.
//!
//! Feature: spec/features/remove-attachment-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because:
//!   - `codelet/fspec/src/main.rs` does not yet register a `remove-attachment`
//!     clap subcommand (clap returns exit code 2 for "unrecognized
//!     subcommand").
//!   - `codelet/fspec-core/src/commands/remove_attachment.rs` is still a
//!     NotYetPorted stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────────────────────── helpers ─────────────────────────

fn run_remove_attachment(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-attachment");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-attachment");
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

const TS_HELP_FIXTURE_RA: &str = include_str!("fixtures/help/remove-attachment.txt");

// ───────────────────────── scenarios ─────────────────────────

#[test]
fn scenario_remove_attachment_help_matches_ts_fixture_byte_for_byte() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `fspec remove-attachment --help` piped to non-TTY (no color codes)
    let output = Command::new(fspec_bin())
        .arg("remove-attachment")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-attachment --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "fspec remove-attachment --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-attachment.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RA);

    // @step And stdout starts with a blank line followed by 'REMOVE-ATTACHMENT'
    assert!(
        stdout.starts_with("\nREMOVE-ATTACHMENT\n"),
        "help must start with blank line then REMOVE-ATTACHMENT header; got:\n{stdout}"
    );

    // @step And stdout contains the section header 'USAGE' followed by '  fspec remove-attachment <workUnitId> <fileName> [options]'
    assert!(
        stdout.contains("USAGE\n  fspec remove-attachment <workUnitId> <fileName> [options]\n"),
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
fn scenario_cli_successfully_removes_attachment_and_prints_canonical_success_block() {
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

    // @step And the file spec/attachments/AUTH-001/diagram.png exists on disk
    write_file(ws.path(), "spec/attachments/AUTH-001/diagram.png", b"png");

    // @step When I run `fspec remove-attachment AUTH-001 diagram.png` in that tempdir
    let (code, stdout, stderr) = run_remove_attachment(ws.path(), &["AUTH-001", "diagram.png"]);

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "fspec remove-attachment must exit 0; stdout={stdout}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Attachment removed from work unit and file deleted'
    assert!(
        stdout.contains("✓ Attachment removed from work unit and file deleted"),
        "stdout: {stdout}"
    );

    // @step And stdout contains the substring '  File: spec/attachments/AUTH-001/diagram.png'
    assert!(
        stdout.contains("  File: spec/attachments/AUTH-001/diagram.png"),
        "stdout: {stdout}"
    );

    // @step And spec/attachments/AUTH-001/diagram.png NO LONGER exists on disk
    assert!(
        !ws.path()
            .join("spec/attachments/AUTH-001/diagram.png")
            .exists(),
        "file must be unlinked"
    );

    // @step And spec/work-units.json on disk shows AUTH-001.attachments=[]
    let raw = fs::read_to_string(ws.path().join("spec/work-units.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let arr = v["workUnits"]["AUTH-001"]["attachments"]
        .as_array()
        .expect("attachments array");
    assert!(arr.is_empty(), "attachments must be empty; got {arr:?}");
}

#[test]
fn scenario_cli_passes_keep_file_through_to_core_preserving_file_on_disk() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/keep.pdf']
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/keep.pdf"]),
        ),
    );

    // @step And the file spec/attachments/AUTH-001/keep.pdf exists on disk
    write_file(
        ws.path(),
        "spec/attachments/AUTH-001/keep.pdf",
        b"pdf-bytes",
    );

    // @step When I run `fspec remove-attachment AUTH-001 keep.pdf --keep-file` in that tempdir
    let (code, stdout, stderr) =
        run_remove_attachment(ws.path(), &["AUTH-001", "keep.pdf", "--keep-file"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit code; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains the substring '✓ Attachment removed from work unit (file kept)'
    assert!(
        stdout.contains("✓ Attachment removed from work unit (file kept)"),
        "stdout: {stdout}"
    );

    // @step And spec/attachments/AUTH-001/keep.pdf STILL exists on disk
    assert!(
        ws.path()
            .join("spec/attachments/AUTH-001/keep.pdf")
            .exists(),
        "--keep-file must preserve the file on disk"
    );
}

#[test]
fn scenario_cli_surfaces_already_missing_warning_with_exit_0_when_file_is_gone() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/ghost.png']
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&["spec/attachments/AUTH-001/ghost.png"]),
        ),
    );

    // @step And NO file exists at spec/attachments/AUTH-001/ghost.png
    assert!(!ws
        .path()
        .join("spec/attachments/AUTH-001/ghost.png")
        .exists());

    // @step When I run `fspec remove-attachment AUTH-001 ghost.png` in that tempdir
    let (code, stdout, stderr) = run_remove_attachment(ws.path(), &["AUTH-001", "ghost.png"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "exit code; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains the substring '⚠ Attachment removed from work unit (file was already missing)'
    assert!(
        stdout.contains("⚠ Attachment removed from work unit (file was already missing)"),
        "stdout: {stdout}"
    );
}

#[test]
fn scenario_cli_exits_2_when_work_unit_id_positional_missing() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec remove-attachment` (no positionals) from that directory
    let (code, _stdout, stderr) = run_remove_attachment(ws.path(), &[]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec remove-attachment (no positionals) must exit 1 (Commander parity); stderr={stderr}"
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
fn scenario_cli_exits_2_when_file_name_positional_missing() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec remove-attachment AUTH-001` (missing second positional) from that directory
    let (code, _stdout, stderr) = run_remove_attachment(ws.path(), &["AUTH-001"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec remove-attachment AUTH-001 (no second positional) must exit 1 (Commander parity); stderr={stderr}"
    );

    // @step And stderr names the missing required argument
    assert!(
        stderr.contains("FILE_NAME")
            || stderr.contains("file_name")
            || stderr.contains("fileName")
            || stderr.contains("required"),
        "stderr must name the missing required argument; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_with_stderr_prefix_when_work_unit_does_not_exist() {
    // @step Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step When I run `fspec remove-attachment ZZZ-999 diagram.png` in that tempdir
    let (code, stdout, stderr) = run_remove_attachment(ws.path(), &["ZZZ-999", "diagram.png"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec remove-attachment ZZZ-999 must exit 1; stdout={stdout}, stderr={stderr}"
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
fn scenario_cli_exits_1_when_work_unit_has_no_attachments() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no attachments field
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with("AUTH-001", "specifying", None));

    // @step When I run `fspec remove-attachment AUTH-001 whatever.png` in that tempdir
    let (code, stdout, stderr) = run_remove_attachment(ws.path(), &["AUTH-001", "whatever.png"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec remove-attachment must exit 1 on no-attachments; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring "Error: Work unit 'AUTH-001' has no attachments to remove"
    assert!(
        stderr.contains("Error: Work unit 'AUTH-001' has no attachments to remove"),
        "stderr must contain canonical no-attachments error; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_when_filename_does_not_match_any_attachment() {
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

    // @step When I run `fspec remove-attachment AUTH-001 missing.png` in that tempdir
    let (code, stdout, stderr) = run_remove_attachment(ws.path(), &["AUTH-001", "missing.png"]);

    // @step Then the exit code is 1
    assert_eq!(
        code, 1,
        "fspec remove-attachment must exit 1 on unknown filename; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring "Error: Attachment 'missing.png' not found for work unit 'AUTH-001'"
    assert!(
        stderr.contains("Error: Attachment 'missing.png' not found for work unit 'AUTH-001'"),
        "stderr must contain canonical not-found error; got:\n{stderr}"
    );
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_remove_attachment() {
    // @step Given the fspec Rust binary has remove-attachment registered as a clap subcommand

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(code, 0, "fspec --help must exit 0; stderr={stderr}");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists remove-attachment as an available subcommand
    assert!(
        help.contains("remove-attachment"),
        "fspec --help must list 'remove-attachment' subcommand; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying and attachments=['spec/attachments/AUTH-001/a.png','spec/attachments/AUTH-001/b.png']
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with(
            "AUTH-001",
            "specifying",
            Some(&[
                "spec/attachments/AUTH-001/a.png",
                "spec/attachments/AUTH-001/b.png",
            ]),
        ),
    );

    // @step And the corresponding files exist on disk
    write_file(ws.path(), "spec/attachments/AUTH-001/a.png", b"a");
    write_file(ws.path(), "spec/attachments/AUTH-001/b.png", b"b");

    // @step When I dispatch remove-attachment via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' fileName='a.png'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-attachment".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","fileName":"a.png"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success
    assert!(result.success, "dispatcher must succeed; got {result:?}");

    // @step And the CLI bridge module codelet/fspec/src/remove_attachment.rs contains NO splice, file unlink, or atomic write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_attachment.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_attachment.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    let stripped = common::strip_comments(&bridge_src);
    for forbidden in [
        "remove_file",
        "fs::remove_file",
        "unlink",
        "splice",
        "ensure_work_units_file",
        "write_json_atomic",
        "Work unit '",
    ] {
        assert!(
            !stripped.contains(forbidden),
            "CLI bridge must NOT contain domain logic substring {forbidden:?}; got:\n{stripped}"
        );
    }
}
