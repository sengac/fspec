//! CLI surface for the `show-deleted` subcommand on the standalone fspec
//! Rust binary — RPC-301.
//!
//! Feature: spec/features/show-deleted-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_show_deleted(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-deleted");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-deleted");
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

fn auth_001_workunits(
    rules: &str,
    examples: &str,
    questions: &str,
    architecture_notes: &str,
) -> String {
    let mut fields = String::from(
        r#""id":"AUTH-001","title":"t","status":"backlog","createdAt":"x","updatedAt":"x""#,
    );
    if !rules.is_empty() {
        fields.push_str(&format!(r#","rules":{rules}"#));
    }
    if !examples.is_empty() {
        fields.push_str(&format!(r#","examples":{examples}"#));
    }
    if !questions.is_empty() {
        fields.push_str(&format!(r#","questions":{questions}"#));
    }
    if !architecture_notes.is_empty() {
        fields.push_str(&format!(r#","architectureNotes":{architecture_notes}"#));
    }
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "AUTH-001": {{ {fields} }}
  }},
  "states": {{
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes show-deleted as a subcommand with positional workUnitId and no flags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_show_deleted_with_positional_work_unit_id_and_no_flags() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec show-deleted --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-deleted")
        .arg("--help")
        .output()
        .expect("spawn fspec show-deleted --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-deleted --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the show-deleted subcommand
    assert!(
        stdout.contains("show-deleted") || stdout.to_lowercase().contains("deleted"),
        "help must describe the show-deleted subcommand; got:\n{stdout}"
    );

    // @step Then stdout mentions the workUnitId positional argument
    assert!(
        stdout.contains("workUnitId")
            || stdout.contains("WORKUNITID")
            || stdout.contains("work_unit_id")
            || stdout.contains("WORK_UNIT_ID"),
        "help must mention the workUnitId positional argument; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "show-deleted --help must NOT advertise --status; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "show-deleted --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "show-deleted --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints sentinel when the work unit exists but has no deleted items
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_sentinel_when_no_deleted_items() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step Given spec/work-units.json contains AUTH-001 with no rules, examples, questions, or architectureNotes
    write_work_units(ws.path(), &auth_001_workunits("", "", "", ""));

    // @step When I run `./codelet/target/release/fspec show-deleted AUTH-001` from that directory
    let (code, stdout, stderr) = run_show_deleted(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-deleted must exit 0 when no items deleted; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'No deleted items found'
    assert!(
        stdout.contains("No deleted items found"),
        "stdout must contain 'No deleted items found'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders header and item lines for the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_header_and_item_lines() {
    // @step Given spec/work-units.json contains AUTH-001 with one deleted rule (id=0, text='Old rule', deletedAt='2025-01-31T12:00:00.000Z') and one deleted example (id=1, text='Obsolete example', deletedAt='2025-02-01T08:00:00.000Z')
    let ws = tempfile::tempdir().expect("tempdir");
    let rules = r#"[
        {"id":0,"text":"Old rule","deleted":true,"createdAt":"x","deletedAt":"2025-01-31T12:00:00.000Z"}
    ]"#;
    let examples = r#"[
        {"id":1,"text":"Obsolete example","deleted":true,"createdAt":"x","deletedAt":"2025-02-01T08:00:00.000Z"}
    ]"#;
    write_work_units(ws.path(), &auth_001_workunits(rules, examples, "", ""));

    // @step When I run `./codelet/target/release/fspec show-deleted AUTH-001`
    let (code, stdout, stderr) = run_show_deleted(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-deleted must exit 0 on the populated case; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Deleted items in AUTH-001 (2 total):'
    assert!(
        stdout.contains("Deleted items in AUTH-001 (2 total):"),
        "stdout must contain header line; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  [0] Old rule (deleted: 2025-01-31T12:00:00.000Z)'
    assert!(
        stdout
            .lines()
            .any(|l| l == "  [0] Old rule (deleted: 2025-01-31T12:00:00.000Z)"),
        "stdout must contain exact rule line; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  [1] Obsolete example (deleted: 2025-02-01T08:00:00.000Z)'
    assert!(
        stdout
            .lines()
            .any(|l| l == "  [1] Obsolete example (deleted: 2025-02-01T08:00:00.000Z)"),
        "stdout must contain exact example line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and writes to stderr when the work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_unknown_work_unit_exits_1_with_stderr() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec show-deleted UNKNOWN-999`
    let (code, stdout, stderr) = run_show_deleted(ws.path(), &["UNKNOWN-999"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec show-deleted must exit 1 for unknown work unit; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring '✗ Failed to show deleted items:' (TS-parity prefix)
    assert!(
        stderr.contains("✗ Failed to show deleted items:"),
        "stderr must contain '✗ Failed to show deleted items:' prefix; got:\n{stderr}"
    );

    // @step Then stderr contains the substring "Work unit 'UNKNOWN-999' does not exist"
    assert!(
        stderr.contains("Work unit 'UNKNOWN-999' does not exist"),
        "stderr must contain canonical missing-work-unit message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI auto-creates work-units.json before checking for the requested work unit
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_auto_creates_work_units_json_before_check() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec show-deleted AUTH-001`
    let (code, stdout, stderr) = run_show_deleted(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec show-deleted must exit 1 when the work unit does not exist; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then spec/work-units.json was created in the directory
    assert!(
        ws.path().join("spec/work-units.json").exists(),
        "show-deleted MUST auto-create spec/work-units.json (load-or-init parity with TS)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_show_deleted() {
    // @step Given the fspec Rust binary has show-deleted registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes

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

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, and show-deleted as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "show-deleted",
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
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with one deleted rule (id=5, text='Shared', deletedAt='2025-03-01T00:00:00.000Z')
    let ws = tempfile::tempdir().expect("tempdir");
    let rules = r#"[
        {"id":5,"text":"Shared","deleted":true,"createdAt":"x","deletedAt":"2025-03-01T00:00:00.000Z"}
    ]"#;
    write_work_units(ws.path(), &auth_001_workunits(rules, "", "", ""));

    // @step When I dispatch show-deleted through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-deleted".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data shows totalDeleted=1 with text='Shared'
    assert_eq!(dispatcher_data["totalDeleted"].as_u64(), Some(1));
    let items = dispatcher_data["deletedItems"]
        .as_array()
        .expect("deletedItems array");
    assert_eq!(items[0]["text"].as_str(), Some("Shared"));

    // @step Then the CLI text output `fspec show-deleted AUTH-001` shows the exact line '  [5] Shared (deleted: 2025-03-01T00:00:00.000Z)' against the same on-disk state
    let (code, stdout, _stderr) = run_show_deleted(ws.path(), &["AUTH-001"]);
    assert_eq!(code, 0);
    assert!(
        stdout
            .lines()
            .any(|l| l == "  [5] Shared (deleted: 2025-03-01T00:00:00.000Z)"),
        "CLI text output must reflect the same item line as the dispatcher; got:\n{stdout}"
    );

    // @step Then the CLI bridge module codelet/fspec/src/show_deleted.rs contains NO inline deleted-item collection, filter, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_deleted.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/show_deleted.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "deletedItems",
        "totalDeleted",
        "No deleted items found",
        "Deleted items in",
        "architectureNotes",
        "questions",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-deleted --help is byte-for-byte identical to TS (RPC-301)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_SD: &str = include_str!("fixtures/help/show-deleted.txt");

#[test]
fn scenario_show_deleted_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec show-deleted --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-deleted")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-deleted --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "show-deleted --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-deleted.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SD);

    // @step And stdout starts with a blank line followed by 'SHOW-DELETED'
    assert!(stdout.starts_with("\nSHOW-DELETED\n"));
}
