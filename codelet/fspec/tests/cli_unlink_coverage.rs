//! CLI surface for the `unlink-coverage` subcommand on the standalone
//! fspec Rust binary — RPC-311.
//!
//! Features:
//!   - spec/features/unlink-coverage-rust-port.feature
//!   - spec/features/unlink-coverage-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired and the fspec_core port
//! is implemented (Phase C), these tests exercise the binary/dispatcher
//! and expect missing-subcommand / NotYetPorted failures. Once Phase C
//! lands the green-phase assertions take over.
//!
//! Coverage-sidecar fixtures are seeded inline, pretty-printed and visually
//! inspected (no duplicate keys).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── Helpers ─────────

fn run_uc(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("unlink-coverage");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec unlink-coverage");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "unlink-coverage".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// Sidecar where scenario "Login" has one test mapping.
fn write_login_sidecar(cwd: &Path) {
    let body = r#"{
  "scenarios": [
    {
      "name": "Login",
      "testMappings": [
        {
          "file": "src/auth.test.ts",
          "lines": "1-10",
          "implMappings": [
            { "file": "src/old.ts", "lines": [1, 2, 3] }
          ]
        }
      ]
    }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["src/auth.test.ts"],
    "implFiles": ["src/old.ts"],
    "totalLinesCovered": 13
  }
}"#;
    write_file(cwd, "spec/features/user-login.feature.coverage", body);
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Clap exposes unlink-coverage as a subcommand and prints flag help
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn clap_exposes_unlink_coverage_as_a_subcommand_and_prints_flag_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec unlink-coverage --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("unlink-coverage")
        .arg("--help")
        .output()
        .expect("spawn fspec unlink-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'unlink-coverage'
    assert!(
        stdout.contains("unlink-coverage") || stdout.contains("UNLINK-COVERAGE"),
        "help must mention subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--scenario'
    assert!(
        stdout.contains("--scenario"),
        "help must mention --scenario; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI --all empties the scenario mappings and prints the success message
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_all_empties_the_scenario_mappings_and_prints_the_success_message() {
    // @step Given a temp workspace has a coverage sidecar where scenario "Login" has one test mapping
    let ws = tempfile::tempdir().expect("tempdir");
    write_login_sidecar(ws.path());

    // @step When I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login --all` from that workspace
    let (code, stdout, stderr) =
        run_uc(ws.path(), &["user-login", "--scenario", "Login", "--all"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Removed all coverage mappings for scenario "Login"'
    assert!(
        stdout.contains("Removed all coverage mappings for scenario \"Login\""),
        "stdout must contain success message; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI without --all or --test-file exits 1
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_without_all_or_test_file_exits_1() {
    // @step Given a temp workspace has a coverage sidecar with scenario "Login"
    let ws = tempfile::tempdir().expect("tempdir");
    write_login_sidecar(ws.path());

    // @step When I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login` from that workspace
    let (code, _stdout, stderr) = run_uc(ws.path(), &["user-login", "--scenario", "Login"]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero without --all or --test-file");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain 'Error:'; got:\n{stderr}");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI exits 1 when the coverage file is missing
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_exits_1_when_the_coverage_file_is_missing() {
    // @step Given an empty directory with no coverage sidecar is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login --all` from that directory
    let (code, _stdout, stderr) =
        run_uc(ws.path(), &["user-login", "--scenario", "Login", "--all"]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero when coverage file missing");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain 'Error:'; got:\n{stderr}");
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: unlink-coverage --help is byte-for-byte identical to the TS reference
// ═════════════════════════════════════════════════════════════════════════

const TS_HELP_FIXTURE_UC: &str = include_str!("fixtures/help/unlink-coverage.txt");

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec unlink-coverage --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("unlink-coverage")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn unlink-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/unlink-coverage.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_UC);

    // @step And stdout starts with a blank line followed by 'UNLINK-COVERAGE'
    assert!(stdout.starts_with("\nUNLINK-COVERAGE\n"));
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn default_combined_tui_mode_is_preserved_when_no_subcommand_is_provided() {
    // @step Given the fspec Rust binary has unlink-coverage registered as a clap subcommand alongside other ported subcommands

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists unlink-coverage as an available subcommand
    assert!(
        help.contains("unlink-coverage"),
        "fspec --help must list unlink-coverage; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_delegates_to_the_same_fspec_core_function_used_by_the_dispatcher() {
    // @step Given a temp workspace has a coverage sidecar where scenario "Login" has one test mapping
    let ws_dispatch = tempfile::tempdir().expect("tempdir");
    write_login_sidecar(ws_dispatch.path());
    let ws_cli = tempfile::tempdir().expect("tempdir");
    write_login_sidecar(ws_cli.path());

    // @step When I dispatch unlink-coverage through fspec_core::dispatch::dispatch_command for feature "user-login" with scenario='Login' and all=true against that workspace
    let result = dispatch(
        ws_dispatch.path(),
        r#"{"featureName":"user-login","scenario":"Login","all":true}"#,
    );

    // @step And I run `./codelet/target/release/fspec unlink-coverage user-login --scenario Login --all` against an identical workspace
    let (code, _stdout, _stderr) =
        run_uc(ws_cli.path(), &["user-login", "--scenario", "Login", "--all"]);

    // @step Then both invocations report success
    assert!(result.success, "dispatcher must report success; got {result:?}");
    assert_eq!(code, 0, "CLI must exit 0");

    // @step And the CLI bridge module codelet/fspec/src/unlink_coverage.rs contains NO inline mutation or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/unlink_coverage.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/unlink_coverage.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Removed all coverage mappings",
        "update_stats",
        "write_json_atomic",
        "testMappings",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
