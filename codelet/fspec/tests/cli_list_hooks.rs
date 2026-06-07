//! CLI surface for the `list-hooks` subcommand on the standalone fspec
//! Rust binary — RPC-247.
//!
//! Feature: spec/features/list-hooks-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because:
//!   - `codelet/fspec/src/main.rs` does not yet register a `list-hooks`
//!     clap subcommand (clap returns exit code 2 for "unrecognized
//!     subcommand").
//!   - `codelet/fspec-core/src/commands/list_hooks.rs` is still a
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

fn run_list_hooks(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-hooks");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-hooks");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_hooks(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-hooks.json"), raw).expect("write fspec-hooks.json");
}

fn canonical_hooks_json() -> String {
    r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "l.sh" }
    ],
    "post-implementing": [
      { "name": "test", "command": "t.sh" },
      { "name": "notify", "command": "n.sh" }
    ]
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-hooks --help is byte-for-byte identical to TS reference output
// ─────────────────────────────────────────────────────────────────────────

/// Captured byte-exact TS reference output of
/// `node dist/index.js list-hooks --help` when piped to non-TTY (no colour).
/// Regenerate via:
///   `cd /tmp && node /Users/rquast/projects/fspec/dist/index.js list-hooks --help \
///    > codelet/fspec/tests/fixtures/help/list-hooks.txt`
const TS_HELP_FIXTURE: &str =
    include_str!("fixtures/help/list-hooks.txt");

#[test]
fn scenario_clap_exposes_list_hooks_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step And the TS reference binary `node dist/index.js list-hooks --help` produces a documented 51-line block starting with a blank line, then "LIST-HOOKS", "List all configured lifecycle hooks", then WHEN TO USE / USAGE / OPTIONS / TYPICAL WORKFLOW / EXAMPLES / COMMON ERRORS / RELATED COMMANDS / NOTES sections
    // (Fixture captured at codelet/fspec/tests/fixtures/help/list-hooks.txt — 51 lines, 1237 bytes.)

    // @step When I run `./codelet/target/release/fspec list-hooks --help` piped to non-TTY (no color codes)
    let output = Command::new(fspec_bin())
        .arg("list-hooks")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-hooks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-hooks --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the TS reference output
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "list-hooks --help output must be byte-for-byte identical to TS reference"
    );

    // @step Then stdout starts with a blank line followed by "LIST-HOOKS"
    assert!(
        stdout.starts_with("\nLIST-HOOKS\n"),
        "help must start with blank line then LIST-HOOKS header; got first 32 bytes:\n{:?}",
        &stdout.chars().take(32).collect::<String>()
    );

    // @step Then stdout contains the section header "WHEN TO USE"
    assert!(stdout.contains("WHEN TO USE\n"), "help must contain WHEN TO USE section");

    // @step Then stdout contains the section header "USAGE" followed by "  fspec list-hooks"
    assert!(
        stdout.contains("USAGE\n  fspec list-hooks\n"),
        "help must contain USAGE\\n  fspec list-hooks"
    );

    // @step Then stdout contains the section header "OPTIONS" followed by "  No options available"
    assert!(
        stdout.contains("OPTIONS\n  No options available\n"),
        "help must contain OPTIONS\\n  No options available"
    );

    // @step Then stdout contains the section header "TYPICAL WORKFLOW"
    assert!(stdout.contains("TYPICAL WORKFLOW\n"), "help must contain TYPICAL WORKFLOW section");

    // @step Then stdout contains the section header "EXAMPLES" with both documented examples
    assert!(stdout.contains("EXAMPLES\n"), "help must contain EXAMPLES section");
    assert!(stdout.contains("1. List all configured hooks"), "help must list example 1");
    assert!(stdout.contains("2. When no hooks are configured"), "help must list example 2");

    // @step Then stdout contains the section header "COMMON ERRORS"
    assert!(stdout.contains("COMMON ERRORS\n"), "help must contain COMMON ERRORS section");

    // @step Then stdout contains the section header "RELATED COMMANDS" listing validate-hooks, add-hook, remove-hook
    assert!(stdout.contains("RELATED COMMANDS\n"), "help must contain RELATED COMMANDS section");
    assert!(stdout.contains("fspec validate-hooks"), "help must list validate-hooks");
    assert!(stdout.contains("fspec add-hook"), "help must list add-hook");
    assert!(stdout.contains("fspec remove-hook"), "help must list remove-hook");

    // @step Then stdout contains the section header "NOTES" listing the four documented notes
    assert!(stdout.contains("NOTES\n"), "help must contain NOTES section");
    assert!(stdout.contains("Reads from spec/fspec-hooks.json"), "help must list note 1");
    assert!(stdout.contains("Shows event names and hook names only"), "help must list note 2");
    assert!(stdout.contains("Use validate-hooks to check if hook scripts exist"), "help must list note 3");
    assert!(stdout.contains("Hooks are organized by event"), "help must list note 4");

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-hooks --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--event'
    assert!(
        !stdout.contains("--event"),
        "list-hooks --help must NOT advertise --event; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-hooks --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory writes zero bytes to stdout and does not auto-create files
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_directory_writes_zero_bytes_and_no_files() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec list-hooks` from that directory
    let (code, stdout, stderr) = run_list_hooks(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-hooks must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then spec/fspec-hooks.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/fspec-hooks.json").exists(),
        "list-hooks must NOT auto-create spec/fspec-hooks.json"
    );

    // @step Then stdout is exactly zero bytes (byte-parity with TS Commander.js action that discards listHooks result)
    assert_eq!(
        stdout.len(),
        0,
        "stdout must be exactly zero bytes (TS Commander.js action discards listHooks result); got {} bytes:\n{stdout}",
        stdout.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI writes zero bytes to stdout regardless of populated hooks (TS Commander.js action discards result)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_writes_zero_bytes_regardless_of_populated_hooks() {
    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with hooks ['lint'] and event 'post-implementing' with hooks ['test', 'notify'] in that order
    let ws = tempfile::tempdir().expect("tempdir");
    write_hooks(ws.path(), &canonical_hooks_json());

    // @step When I run `./codelet/target/release/fspec list-hooks`
    let (code, stdout, stderr) = run_list_hooks(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-hooks must exit 0 on populated config; got {code}, stderr={stderr}"
    );

    // @step Then stdout is exactly zero bytes (byte-parity with TS Commander.js action that discards listHooks result)
    assert_eq!(
        stdout.len(),
        0,
        "stdout must be exactly zero bytes (TS Commander.js action discards listHooks result); got {} bytes:\n{stdout}",
        stdout.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 0 with zero stdout bytes when spec/fspec-hooks.json contains invalid JSON
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_invalid_json_exits_0_with_zero_stdout() {
    // @step Given spec/fspec-hooks.json exists in the working directory but contains invalid JSON syntax
    let ws = tempfile::tempdir().expect("tempdir");
    write_hooks(ws.path(), "{ not json");

    // @step When I run `./codelet/target/release/fspec list-hooks`
    let (code, stdout, stderr) = run_list_hooks(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-hooks must exit 0 even when fspec-hooks.json is malformed (TS bare catch swallows parse errors); got {code}, stderr={stderr}"
    );

    // @step Then stderr does NOT contain the substring 'Error:'
    assert!(
        !stderr.contains("Error:"),
        "stderr must NOT contain 'Error:' on swallowed parse failure; got:\n{stderr}"
    );

    // @step Then stdout is exactly zero bytes (byte-parity with TS Commander.js action that discards listHooks result on the swallowed-error path)
    assert_eq!(
        stdout.len(),
        0,
        "stdout must be exactly zero bytes on swallowed-error path (TS discards result); got {} bytes:\n{stdout}",
        stdout.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_hooks() {
    // @step Given the fspec Rust binary has list-hooks registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    // (asserted by the help-listing check below)

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-hooks as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "list-hooks",
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
    // @step Given a project root whose spec/fspec-hooks.json contains event 'post-implementing' with hooks ['lint','test']
    let ws = tempfile::tempdir().expect("tempdir");
    write_hooks(
        ws.path(),
        r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "l.sh" },
      { "name": "test", "command": "t.sh" }
    ]
  }
}"#,
    );

    // @step When I dispatch list-hooks through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-hooks".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let events = dispatcher_data["events"].as_array().expect("events array");

    // @step Then the dispatcher's DispatchResult.data parses to an events array containing one entry with event='post-implementing' and hooks=['lint','test']
    assert_eq!(events.len(), 1, "expected 1 event; got {events:?}");
    assert_eq!(events[0]["event"].as_str(), Some("post-implementing"));
    let hooks = events[0]["hooks"].as_array().expect("hooks array");
    assert_eq!(hooks.len(), 2);
    assert_eq!(hooks[0].as_str(), Some("lint"));
    assert_eq!(hooks[1].as_str(), Some("test"));

    // @step Then the CLI bridge module codelet/fspec/src/list_hooks.rs contains NO inline event-aggregation or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_hooks.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_hooks.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // Forbid any inline aggregation/rendering markers — the bridge must only
    // marshall args and delegate. (Same anti-duplication guard used by
    // RPC-248 in the list_prefixes bridge.)
    for forbidden in [
        "Configured Hooks:",
        "No hooks are configured",
        "event_name",
        "Object::entries",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
