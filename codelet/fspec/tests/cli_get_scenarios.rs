//! CLI surface for the `get-scenarios` subcommand on the standalone fspec
//! Rust binary — RPC-237.
//!
//! Feature: spec/features/get-scenarios-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand + help intercept are wired (Phase C),
//! these tests fail — the binary rejects the unknown subcommand and the bridge
//! module does not yet exist. Once wired, the green-phase assertions hold.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── helpers ─────────

fn run_get(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("get-scenarios");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec get-scenarios");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write feature");
}

const LOGIN_AUTH_TWO_SCENARIOS: &str = "@auth\nFeature: Login\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    When I submit credentials\n    Then I see the dashboard\n\n  Scenario: Login with invalid password\n    Given I am on the login page\n    When I submit a bad password\n    Then I see an error\n";

// ───────── scenarios ─────────

#[test]
fn scenario_clap_exposes_get_scenarios_with_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec get-scenarios --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("get-scenarios")
        .arg("--help")
        .output()
        .expect("spawn fspec get-scenarios --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec get-scenarios --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'get-scenarios'
    assert!(
        stdout.contains("get-scenarios") || stdout.contains("GET-SCENARIOS"),
        "help must describe the get-scenarios subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_format_json_prints_a_json_array_of_scenario_objects() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with two scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", LOGIN_AUTH_TWO_SCENARIOS);

    // @step When I run `./codelet/target/release/fspec get-scenarios --tag @auth --format json` from that workspace
    let (code, stdout, stderr) = run_get(ws.path(), &["--tag", "@auth", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec get-scenarios --tag @auth --format json must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout parses as a JSON array
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout must be valid JSON: {e}\nstdout was:\n{stdout}"));
    let arr = parsed.as_array().expect("stdout must be a JSON array");

    // @step Then each array element has the keys feature, name, and line
    assert!(!arr.is_empty(), "expected non-empty array; got:\n{stdout}");
    for el in arr {
        assert!(el.get("feature").is_some(), "missing feature key: {el}");
        assert!(el.get("name").is_some(), "missing name key: {el}");
        assert!(el.get("line").is_some(), "missing line key: {el}");
    }
}

#[test]
fn scenario_cli_default_text_output_prints_count_message_and_groups_by_feature() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with two scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", LOGIN_AUTH_TWO_SCENARIOS);

    // @step When I run `./codelet/target/release/fspec get-scenarios --tag @auth` from that workspace
    let (code, stdout, stderr) = run_get(ws.path(), &["--tag", "@auth"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec get-scenarios --tag @auth must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Found 2 scenarios matching tags: @auth'
    assert!(
        stdout.contains("Found 2 scenarios matching tags: @auth"),
        "got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'spec/features/login.feature'
    assert!(stdout.contains("spec/features/login.feature"), "got:\n{stdout}");
}

#[test]
fn scenario_cli_against_workspace_with_no_spec_features_exits_1() {
    // @step Given an empty directory with no spec/ subdirectory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec get-scenarios` from that directory
    let (code, stdout, stderr) = run_get(ws.path(), &[]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec get-scenarios must exit 1 with no spec/features; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain 'Error:'; got:\n{stderr}");

    // @step Then stderr contains the substring 'spec/features directory not found'
    assert!(
        stderr.contains("spec/features directory not found"),
        "stderr must contain canonical substring; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with two scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", LOGIN_AUTH_TWO_SCENARIOS);

    // @step When I dispatch get-scenarios through fspec_core::dispatch::dispatch_command with tags=['@auth'] and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "get-scenarios".to_string(),
        args_json: r#"{"tags":["@auth"],"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then the DispatchResult succeeds and its data is a JSON array matching the CLI's --format json stdout
    let (code, stdout, stderr) = run_get(ws.path(), &["--tag", "@auth", "--format", "json"]);
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}");
    let cli_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("CLI stdout is JSON array");
    // The dispatcher returns the full envelope; the CLI prints only the
    // scenarios array. The CLI array must equal the dispatcher envelope's
    // scenarios field.
    let envelope: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(
        cli_json, envelope["scenarios"],
        "CLI array must match dispatcher envelope scenarios; cli=\n{stdout}\ndispatcher=\n{}",
        result.data
    );

    // @step Then the CLI bridge module codelet/fspec/src/get_scenarios.rs contains NO inline parsing, filtering, or rendering logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/get_scenarios.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/get_scenarios.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "glob_feature_files",
        "parse_feature_lenient",
        "No scenarios found",
        "totalCount",
        "matching tags",
        "Gherkin",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_GS: &str = include_str!("fixtures/help/get-scenarios.txt");

#[test]
fn scenario_get_scenarios_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec get-scenarios --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("get-scenarios")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn get-scenarios --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "get-scenarios --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/get-scenarios.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_GS);

    // @step Then stdout starts with a blank line followed by 'GET-SCENARIOS'
    assert!(stdout.starts_with("\nGET-SCENARIOS\n"));
}
