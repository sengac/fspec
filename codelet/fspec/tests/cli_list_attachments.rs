//! CLI surface for the `list-attachments` subcommand on the standalone fspec
//! Rust binary — RPC-241.
//!
//! Feature: spec/features/list-attachments-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because:
//!   - `codelet/fspec/src/main.rs` does not yet register a `list-attachments`
//!     clap subcommand (clap returns exit code 2 for "unrecognized
//!     subcommand").
//!   - `codelet/fspec-core/src/commands/list_attachments.rs` is still a
//!     NotYetPorted stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_list_attachments(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-attachments");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-attachments");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn write_attachment_file(project_root: &Path, rel_path: &str, bytes: usize) {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir attachment parent");
    }
    let content = vec![0u8; bytes];
    fs::write(&full, &content).expect("write attachment file");
}

/// Build a single-work-unit JSON document, optionally embedding an
/// `attachments` array via raw `serde_json::json!`.
fn work_units_with_attachments(id: &str, attachments: Option<&[&str]>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".to_string(), serde_json::json!(id));
    wu.insert(
        "title".to_string(),
        serde_json::json!(format!("title for {id}")),
    );
    wu.insert("status".to_string(), serde_json::json!("backlog"));
    wu.insert(
        "createdAt".to_string(),
        serde_json::json!("2026-06-01T00:00:00.000Z"),
    );
    wu.insert(
        "updatedAt".to_string(),
        serde_json::json!("2026-06-01T00:00:00.000Z"),
    );
    if let Some(att) = attachments {
        wu.insert(
            "attachments".to_string(),
            serde_json::Value::Array(att.iter().map(|s| serde_json::json!(s)).collect()),
        );
    }

    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), serde_json::Value::Object(wu));

    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": serde_json::Value::Object(wus),
        "states": {
            "backlog": [id], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-attachments --help is byte-for-byte identical to TS reference output
// ─────────────────────────────────────────────────────────────────────────

const TS_LA_HELP_FIXTURE: &str =
    include_str!("fixtures/help/list-attachments.txt");

#[test]
fn scenario_clap_exposes_list_attachments_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step And the TS reference binary `node dist/index.js list-attachments --help` produces a documented 69-line block (LIST-ATTACHMENTS header through NOTES section, including the TS-quirks: typicalWorkflow array comma-joined, relatedCommands entries already prefixed with 'fspec ', commonErrors Fix:undefined)

    // @step When I run `./codelet/target/release/fspec list-attachments --help` piped to non-TTY (no color codes)
    let output = Command::new(fspec_bin())
        .arg("list-attachments")
        .arg("--help")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec list-attachments --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-attachments --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the TS reference output
    assert_eq!(
        stdout, TS_LA_HELP_FIXTURE,
        "list-attachments --help output must be byte-for-byte identical to TS reference"
    );

    // @step Then stdout starts with a blank line followed by "LIST-ATTACHMENTS"
    assert!(
        stdout.starts_with("\nLIST-ATTACHMENTS\n"),
        "help must start with blank line then LIST-ATTACHMENTS header"
    );

    // @step Then stdout contains the section header "PREREQUISITES"
    assert!(stdout.contains("PREREQUISITES\n"), "help must contain PREREQUISITES section");

    // @step Then stdout contains the section header "USAGE" followed by "  fspec list-attachments <workUnitId>"
    assert!(
        stdout.contains("USAGE\n  fspec list-attachments <workUnitId>\n"),
        "help must contain USAGE\\n  fspec list-attachments <workUnitId>"
    );

    // @step Then stdout contains the section header "ARGUMENTS"
    assert!(stdout.contains("ARGUMENTS\n"), "help must contain ARGUMENTS section");

    // @step Then stdout contains the section header "OPTIONS" followed by "  No options available"
    assert!(
        stdout.contains("OPTIONS\n  No options available\n"),
        "help must contain OPTIONS\\n  No options available"
    );

    // @step Then stdout contains the section header "TYPICAL WORKFLOW"
    assert!(stdout.contains("TYPICAL WORKFLOW\n"), "help must contain TYPICAL WORKFLOW section");

    // @step Then stdout contains the section header "EXAMPLES"
    assert!(stdout.contains("EXAMPLES\n"), "help must contain EXAMPLES section");

    // @step Then stdout contains the section header "COMMON ERRORS" with the literal token "Fix: undefined" twice
    assert!(stdout.contains("COMMON ERRORS\n"), "help must contain COMMON ERRORS section");
    assert_eq!(
        stdout.matches("Fix: undefined").count(),
        2,
        "help must reproduce TS bug: Fix: undefined appears twice"
    );

    // @step Then stdout contains the section header "RELATED COMMANDS" with three entries each prefixed by "fspec fspec "
    assert!(stdout.contains("RELATED COMMANDS\n"), "help must contain RELATED COMMANDS section");
    assert_eq!(
        stdout.matches("fspec fspec ").count(),
        3,
        "help must reproduce TS bug: relatedCommands entries already prefixed with 'fspec ' yielding 'fspec fspec '"
    );

    // @step Then stdout contains the section header "NOTES"
    assert!(stdout.contains("NOTES\n"), "help must contain NOTES section");

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "list-attachments --help must NOT advertise --status; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--prefix'
    assert!(
        !stdout.contains("--prefix"),
        "list-attachments --help must NOT advertise --prefix; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--epic'
    assert!(
        !stdout.contains("--epic"),
        "list-attachments --help must NOT advertise --epic; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-attachments --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-attachments --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 2 when the required positional argument is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_missing_positional_exits_2() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec list-attachments` (no positional) from that directory
    let (code, _stdout, stderr) = run_list_attachments(ws.path(), &[]);

    // @step Then the command exits with code 2
    assert_eq!(
        code, 2,
        "fspec list-attachments (no positional) must exit 2 (clap usage error); got {code}, stderr={stderr}"
    );

    // @step Then stderr names the missing required argument
    assert!(
        stderr.contains("WORK_UNIT_ID")
            || stderr.contains("work_unit_id")
            || stderr.contains("required"),
        "stderr must name the missing required argument; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the empty-attachments sentinel and exits 0 when the work unit has no attachments
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_empty_sentinel_and_exits_0() {
    // @step Given spec/work-units.json contains AUTH-001 with no attachments field
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_attachments("AUTH-001", None));

    // @step When I run `./codelet/target/release/fspec list-attachments AUTH-001`
    let (code, stdout, stderr) = run_list_attachments(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-attachments AUTH-001 must exit 0 on empty-attachments; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring "No attachments found for work unit AUTH-001"
    assert!(
        stdout.contains("No attachments found for work unit AUTH-001"),
        "stdout must contain the empty-attachments sentinel; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders present and missing attachments with size and ✗ markers
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_renders_present_and_missing_attachments() {
    // @step Given spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/a.png","spec/attachments/AUTH-001/b.png"]
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with_attachments(
            "AUTH-001",
            Some(&[
                "spec/attachments/AUTH-001/a.png",
                "spec/attachments/AUTH-001/b.png",
            ]),
        ),
    );

    // @step Given the file spec/attachments/AUTH-001/a.png exists on disk with exactly 1234 bytes
    write_attachment_file(ws.path(), "spec/attachments/AUTH-001/a.png", 1234);

    // @step Given no file exists at spec/attachments/AUTH-001/b.png
    assert!(!ws.path().join("spec/attachments/AUTH-001/b.png").exists());

    // @step When I run `./codelet/target/release/fspec list-attachments AUTH-001`
    let (code, stdout, stderr) = run_list_attachments(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-attachments AUTH-001 must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring "Attachments for AUTH-001 (2):"
    assert!(
        stdout.contains("Attachments for AUTH-001 (2):"),
        "stdout must contain header 'Attachments for AUTH-001 (2):'; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line "  ✓ spec/attachments/AUTH-001/a.png"
    assert!(
        stdout
            .lines()
            .any(|l| l == "  ✓ spec/attachments/AUTH-001/a.png"),
        "stdout must contain exact present-marker line for a.png; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line "    Size: 1.21 KB"
    assert!(
        stdout.lines().any(|l| l == "    Size: 1.21 KB"),
        "stdout must contain exact line '    Size: 1.21 KB' (1234/1024 → 1.21); got:\n{stdout}"
    );

    // @step Then stdout contains a line starting with "    Modified: "
    assert!(
        stdout.lines().any(|l| l.starts_with("    Modified: ")),
        "stdout must contain a line starting with '    Modified: '; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line "  ✗ spec/attachments/AUTH-001/b.png"
    assert!(
        stdout
            .lines()
            .any(|l| l == "  ✗ spec/attachments/AUTH-001/b.png"),
        "stdout must contain exact ✗-marker line for b.png; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line "    File not found on filesystem"
    assert!(
        stdout
            .lines()
            .any(|l| l == "    File not found on filesystem"),
        "stdout must contain exact 'File not found on filesystem' line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when the requested work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_unknown_work_unit_exits_1_with_stderr() {
    // @step Given spec/work-units.json contains AUTH-001 only (no NONEXISTENT-001 entry)
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_attachments("AUTH-001", None));

    // @step When I run `./codelet/target/release/fspec list-attachments NONEXISTENT-001`
    let (code, stdout, stderr) = run_list_attachments(ws.path(), &["NONEXISTENT-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-attachments NONEXISTENT-001 must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the exact line "Error: Work unit 'NONEXISTENT-001' does not exist"
    assert!(
        stderr.lines().any(|l| l == "Error: Work unit 'NONEXISTENT-001' does not exist"),
        "stderr must contain exact line 'Error: Work unit 'NONEXISTENT-001' does not exist'; got:\n{stderr}"
    );

    // @step Then stderr does NOT contain the substring "Invalid args for fspec command"
    assert!(
        !stderr.contains("Invalid args for fspec command"),
        "stderr must NOT contain the InvalidArgs wrapper text (TS-parity); got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when work-units.json is malformed
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_malformed_work_units_json_exits_1() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not valid json");

    // @step When I run `./codelet/target/release/fspec list-attachments AUTH-001`
    let (code, stdout, stderr) = run_list_attachments(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec list-attachments must exit 1 on malformed work-units.json; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr must contain canonical parse-error substring; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_attachments() {
    // @step Given the fspec Rust binary has list-attachments registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-attachments as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "list-attachments",
    ] {
        assert!(
            help.contains(sub),
            "fspec --help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with attachments=["spec/attachments/AUTH-001/x.png"]
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with_attachments(
            "AUTH-001",
            Some(&["spec/attachments/AUTH-001/x.png"]),
        ),
    );

    // @step Given the file spec/attachments/AUTH-001/x.png exists on disk with exactly 1024 bytes
    write_attachment_file(ws.path(), "spec/attachments/AUTH-001/x.png", 1024);

    // @step When I dispatch list-attachments through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-attachments".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with attachments array of length 1
    let arr = dispatcher_data["attachments"]
        .as_array()
        .expect("attachments array on root");
    assert_eq!(arr.len(), 1, "attachments array length 1; got {arr:?}");

    // @step Then the CLI bridge module codelet/fspec/src/list_attachments.rs contains NO inline rendering, file-stat, or work-unit-lookup logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_attachments.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_attachments.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Attachments for",
        "No attachments found",
        "File not found on filesystem",
        "Size:",
        "Modified:",
        "ensure_work_units_file",
        "fs::metadata",
        "fs::read_to_string",
        "std::fs::metadata",
        "workUnits",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
