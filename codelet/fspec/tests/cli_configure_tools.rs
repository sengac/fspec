//! CLI surface for the `configure-tools` subcommand on the standalone fspec
//! Rust binary — RPC-208.
//!
//! Feature: spec/features/configure-tools-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_cmd(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("configure-tools");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec configure-tools");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn config_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join("spec/fspec-config.json")
}

fn read_config(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(config_path(project_root)).expect("read fspec-config.json");
    serde_json::from_str(&raw).expect("parse fspec-config.json")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/configure-tools.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_configure_tools_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec configure-tools --help`
    let output = Command::new(fspec_bin())
        .arg("configure-tools")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn configure-tools --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "configure-tools --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/configure-tools.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI saves the test command and prints the confirmation line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_saves_test_command_and_prints_confirmation() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!config_path(ws.path()).exists());

    // @step When I run `fspec configure-tools --test-command "cargo test"` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--test-command", "cargo test"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Tool configuration saved to spec/fspec-config.json'
    assert!(
        stdout.contains("✓ Tool configuration saved to spec/fspec-config.json"),
        "stdout must contain canonical confirmation line; got:\n{stdout}"
    );

    // @step And spec/fspec-config.json shows tools.test.command='cargo test'
    let v = read_config(ws.path());
    assert_eq!(v["tools"]["test"]["command"].as_str(), Some("cargo test"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI forwards multi-value quality commands into the persisted array
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_forwards_multi_value_quality_commands() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec configure-tools --test-command "npm test" --quality-commands "eslint ." "prettier --check ."` in that tempdir
    let (code, _stdout, stderr) = run_cmd(
        ws.path(),
        &[
            "--test-command",
            "npm test",
            "--quality-commands",
            "eslint .",
            "prettier --check .",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And spec/fspec-config.json shows tools.qualityCheck.commands=['eslint .','prettier --check .']
    let v = read_config(ws.path());
    let cmds = v["tools"]["qualityCheck"]["commands"]
        .as_array()
        .expect("commands array");
    let cmds: Vec<&str> = cmds.iter().filter_map(serde_json::Value::as_str).collect();
    assert_eq!(cmds, vec!["eslint .", "prettier --check ."]);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --reconfigure does not write the config file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reconfigure_does_not_write_config() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!config_path(ws.path()).exists());

    // @step When I run `fspec configure-tools --reconfigure` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--reconfigure"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout is empty
    // TS-parity (configure-tools.ts:225-227): the Commander.js action only
    // emits the saved-confirmation line when NOT in --reconfigure mode, and
    // DISCARDS the configureTools() return value. The reconfigure guidance
    // guidance is therefore never printed via the CLI front door — it surfaces
    // only through the LLM-facing dispatcher. So CLI stdout must be empty.
    assert!(
        stdout.trim().is_empty(),
        "CLI --reconfigure must print nothing (TS parity); got:\n{stdout}"
    );

    // @step And spec/fspec-config.json does not exist on disk
    assert!(
        !config_path(ws.path()).exists(),
        "reconfigure must NOT write the config file"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function() {
    // @step Given a project root tempdir with no spec/fspec-config.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I dispatch configure-tools via fspec_core::dispatch::dispatch_command with testCommand='via-dispatcher'
    let req = codelet_fspec_core::DispatchRequest {
        command: "configure-tools".to_string(),
        args_json: r#"{"testCommand":"via-dispatcher"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then spec/fspec-config.json shows tools.test.command='via-dispatcher'
    let v = read_config(ws.path());
    assert_eq!(v["tools"]["test"]["command"].as_str(), Some("via-dispatcher"));

    // @step And running `fspec configure-tools --quality-commands "via-cli"` afterwards exits 0
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--quality-commands", "via-cli"]);
    assert_eq!(code, 0, "CLI configure must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/fspec-config.json still shows tools.test.command='via-dispatcher' and tools.qualityCheck.commands=['via-cli']
    let v = read_config(ws.path());
    assert_eq!(v["tools"]["test"]["command"].as_str(), Some("via-dispatcher"));
    let cmds = v["tools"]["qualityCheck"]["commands"]
        .as_array()
        .expect("commands array");
    let cmds: Vec<&str> = cmds.iter().filter_map(serde_json::Value::as_str).collect();
    assert_eq!(cmds, vec!["via-cli"]);

    // @step And the CLI bridge module codelet/fspec/src/configure_tools.rs contains NO inline config-merge or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/configure_tools.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/configure_tools.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "qualityCheck",
        "write_json_atomic",
        "fspec-config.json",
        "RECONFIGURE TOOLS",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
