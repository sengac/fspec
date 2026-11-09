//! CLI surface for the `show-foundation-event-storm` subcommand on the
//! standalone fspec Rust binary — RPC-306.
//!
//! Features:
//!   - spec/features/show-foundation-event-storm-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired and the fspec_core port
//! is implemented (Phase C), these tests exercise the binary/dispatcher
//! and expect NotYetPorted / missing-subcommand failures. Once Phase C
//! lands the green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── Helpers ─────────

fn run_sfes(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-foundation-event-storm");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd
        .output()
        .expect("spawn fspec show-foundation-event-storm");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), raw).expect("write foundation.json");
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "show-foundation-event-storm".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

// ─── Sample foundation bodies ───

fn foundation_no_event_storm() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]}}"#
}

fn foundation_with_three_active_one_deleted() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"aggregate","text":"A1"},
        {"id":2,"type":"aggregate","text":"A2"},
        {"id":3,"type":"event","text":"E1"},
        {"id":4,"type":"aggregate","text":"A3","deleted":true}
    ]}}"#
}

fn foundation_two_aggs_one_bc_one_event() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"aggregate","text":"A1"},
        {"id":2,"type":"aggregate","text":"A2"},
        {"id":3,"type":"bounded_context","text":"Work Management"},
        {"id":4,"type":"event","text":"E1"}
    ]}}"#
}

fn foundation_bc_with_linked_items() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"bounded_context","text":"Work Management"},
        {"id":10,"type":"aggregate","text":"WU","boundedContextId":1},
        {"id":11,"type":"aggregate","text":"Epic","boundedContextId":1},
        {"id":12,"type":"event","text":"Created","boundedContextId":1},
        {"id":20,"type":"aggregate","text":"Other1","boundedContextId":2},
        {"id":21,"type":"aggregate","text":"Other2","boundedContextId":2}
    ]}}"#
}

fn foundation_bc_two_linked_aggregates() -> &'static str {
    r#"{"version":"2.0.0","project":{"name":"x","vision":"y","projectType":"other"},"problemSpace":{"primaryProblem":{"title":"a","description":"b","impact":"medium"}},"solutionSpace":{"overview":"c","capabilities":[]},"eventStorm":{"items":[
        {"id":1,"type":"bounded_context","text":"Work Management"},
        {"id":10,"type":"aggregate","text":"WU","boundedContextId":1},
        {"id":11,"type":"aggregate","text":"Epic","boundedContextId":1}
    ]}}"#
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 1: Clap exposes subcommand with help
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_clap_exposes_subcommand_with_flag_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-foundation-event-storm")
        .arg("--help")
        .output()
        .expect("spawn fspec show-foundation-event-storm --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'show-foundation-event-storm'
    assert!(
        stdout.contains("show-foundation-event-storm")
            || stdout.contains("SHOW-FOUNDATION-EVENT-STORM"),
        "help must mention subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--type'
    assert!(
        stdout.contains("--type"),
        "help must mention --type; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--context'
    assert!(
        stdout.contains("--context") || stdout.contains("context"),
        "help must mention --context; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 2: CLI no foundation exits 1 with error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_against_workspace_with_no_foundation_exits_1_with_error() {
    // @step Given an empty directory with no spec/ subdirectory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm` from that directory
    let (code, stdout, stderr) = run_sfes(ws.path(), &[]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'foundation.json'
    assert!(
        stderr.contains("foundation.json"),
        "stderr must mention foundation.json; got:\n{stderr}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 3: CLI prints empty array (no eventStorm)
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_prints_empty_array_when_no_event_storm() {
    // @step Given a temp workspace contains spec/foundation.json without an eventStorm field
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_no_event_storm());

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm` from that workspace
    let (code, stdout, stderr) = run_sfes(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as a JSON array with 0 elements
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let arr = parsed.as_array().expect("stdout is JSON array");
    assert_eq!(arr.len(), 0);
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 4: CLI prints all active items
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_prints_all_active_items_when_no_filters() {
    // @step Given a temp workspace contains spec/foundation.json with three active eventStorm items and one item where deleted=true
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_with_three_active_one_deleted());

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm` from that workspace
    let (code, stdout, stderr) = run_sfes(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as a JSON array with 3 elements
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let arr = parsed.as_array().expect("stdout is JSON array");
    assert_eq!(arr.len(), 3);
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 5: CLI --type narrows to matching items
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_type_filter_narrows_to_matching_items() {
    // @step Given a temp workspace contains spec/foundation.json with two aggregates, one bounded_context, and one event
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_two_aggs_one_bc_one_event());

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm --type aggregate` from that workspace
    let (code, stdout, stderr) = run_sfes(ws.path(), &["--type", "aggregate"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as a JSON array with 2 elements
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let arr = parsed.as_array().expect("stdout is JSON array");
    assert_eq!(arr.len(), 2);

    // @step And every JSON element has type='aggregate'
    for item in arr {
        assert_eq!(item["type"].as_str(), Some("aggregate"));
    }
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 6: CLI --context returns BC + linked
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_context_filter_returns_bc_plus_linked_items() {
    // @step Given a temp workspace contains spec/foundation.json with bounded_context id=1 text='Work Management' plus three items linked to it
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_bc_with_linked_items());

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm --context "Work Management"` from that workspace
    let (code, stdout, stderr) = run_sfes(ws.path(), &["--context", "Work Management"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as a JSON array with 4 elements
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let arr = parsed.as_array().expect("stdout is JSON array");
    assert_eq!(arr.len(), 4);
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 7: CLI --context unknown errors and
// lists available contexts (DISC-003 rule 10 supersedes the old
// "prints empty array" behavior)
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_context_unknown_errors_and_lists_available_contexts() {
    // @step Given a temp workspace contains spec/foundation.json with a bounded_context text='Work Management' and items linked to it
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_bc_with_linked_items());

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm --context Nonexistent` from that workspace
    let (code, stdout, stderr) = run_sfes(ws.path(), &["--context", "Nonexistent"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}");

    // @step And stderr contains "Unknown context 'Nonexistent'"
    assert!(
        stderr.contains("Unknown context 'Nonexistent'"),
        "stderr must name the bad context: {stderr}"
    );

    // @step And stderr lists 'Work Management' as an available bounded context
    assert!(
        stderr.contains("Work Management"),
        "stderr must list the available context: {stderr}"
    );
    let _ = stdout;
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 8: --help byte-for-byte identical
// ═════════════════════════════════════════════════════════════════════════

const TS_HELP_FIXTURE_SFES: &str = include_str!("fixtures/help/show-foundation-event-storm.txt");

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec show-foundation-event-storm --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-foundation-event-storm")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-foundation-event-storm --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-foundation-event-storm.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SFES);

    // @step And stdout starts with a blank line followed by 'SHOW-FOUNDATION-EVENT-STORM'
    assert!(stdout.starts_with("\nSHOW-FOUNDATION-EVENT-STORM\n"));
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 9: Default combined TUI mode preserved
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_combined_tui_mode_preserved() {
    // @step Given the fspec Rust binary has show-foundation-event-storm registered as a clap subcommand alongside daemon, client, status, and other ported subcommands

    // @step When I run `./rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists show-foundation-event-storm as an available subcommand
    assert!(
        help.contains("show-foundation-event-storm"),
        "fspec --help must list show-foundation-event-storm; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 10: CLI delegates to fspec_core
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a temp workspace contains spec/foundation.json with one bounded_context 'Work Management' and two linked aggregates
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), foundation_bc_two_linked_aggregates());

    // @step When I dispatch show-foundation-event-storm through fspec_core::dispatch::dispatch_command with context='Work Management' against that workspace
    let result = dispatch(ws.path(), r#"{"context":"Work Management"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let arr = parsed["data"].as_array().expect("data is array");

    // @step And I run `./rust/target/release/fspec show-foundation-event-storm --context "Work Management"` against the same workspace
    let (code, stdout, _stderr) = run_sfes(ws.path(), &["--context", "Work Management"]);
    assert_eq!(code, 0, "CLI must exit 0");
    let cli_parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let cli_arr = cli_parsed.as_array().expect("stdout is JSON array");

    // @step Then both invocations produce JSON arrays with 3 elements
    assert_eq!(arr.len(), 3, "dispatcher result must contain 3 items");
    assert_eq!(cli_arr.len(), 3, "CLI stdout must contain 3 items");

    // @step And the CLI bridge module rust/fspec/src/show_foundation_event_storm.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_foundation_event_storm.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/show_foundation_event_storm.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "boundedContextId",
        "eventStorm",
        "deleted",
        "No Event Storm data",
        "bounded_context",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
