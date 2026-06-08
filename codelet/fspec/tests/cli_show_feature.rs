//! CLI surface for the `show-feature` subcommand on the standalone fspec
//! Rust binary — RPC-304.
//!
//! Feature: spec/features/show-feature-cli-subcommand.feature
//!
//! Green phase: these tests exercise the wired-up clap subcommand
//! (`Mode::ShowFeature` in `codelet/fspec/src/main.rs`) and the ported
//! `codelet/fspec-core/src/commands/show_feature.rs` implementation.
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── Helpers ─────────

fn run_show_feature(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-feature");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-feature");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

const LOGIN_NO_TAGS: &str = "Feature: Login\n\n  Scenario: Login with valid credentials\n    Given I am on the login page\n    Then I see the dashboard\n";

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes show-feature as a subcommand and prints flag-aware help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_show_feature_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec show-feature --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-feature")
        .arg("--help")
        .output()
        .expect("spawn fspec show-feature --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-feature --help must exit 0; stderr={stderr}"
    );

    // @step And stdout contains the substring 'show-feature'
    assert!(
        stdout.contains("show-feature") || stdout.contains("SHOW-FEATURE"),
        "help must mention show-feature; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Feature file path'
    assert!(
        stdout.contains("Feature file path"),
        "help must describe the feature-file argument; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-feature against a workspace with no spec/features prints feature-not-found and exits 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_show_feature_against_empty_workspace_exits_1_with_not_found() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec show-feature missing` from that directory
    let (code, stdout, stderr) = run_show_feature(ws.path(), &["missing"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec show-feature must exit 1 when feature not found; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Feature file not found: missing'
    assert!(
        stderr.contains("Feature file not found: missing"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders feature contents and Work Units None for a tag-free feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_feature_contents_and_work_units_none() {
    // @step Given a temp workspace contains spec/features/login.feature with valid gherkin and no @PREFIX-NNN tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_NO_TAGS);

    // @step When I run `./codelet/target/release/fspec show-feature login` from that workspace
    let (code, stdout, stderr) = run_show_feature(ws.path(), &["login"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-feature must exit 0 on tag-free feature; stderr={stderr}"
    );

    // @step And stdout contains the file body of spec/features/login.feature
    assert!(
        stdout.contains("Feature: Login"),
        "stdout must contain feature body; got:\n{stdout}"
    );
    assert!(
        stdout.contains("Scenario: Login with valid credentials"),
        "stdout must contain scenario line; got:\n{stdout}"
    );

    // @step And stdout contains the exact line 'Work Units: None'
    assert!(
        stdout.lines().any(|l| l == "Work Units: None"),
        "stdout must contain exact line 'Work Units: None'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders Work Unit progress block when the feature carries a work-unit tag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_work_unit_progress_block() {
    // @step Given a temp workspace contains spec/features/auth.feature tagged '@AUTH-001' at the feature level with scenario 'A' on line 4
    let ws = tempfile::tempdir().expect("tempdir");
    let body = "@AUTH-001\nFeature: Auth\n\nScenario: A\n  Given step a\n";
    write_file(ws.path(), "spec/features/auth.feature", body);

    // @step And spec/work-units.json contains AUTH-001 with title 'Login' and status 'implementing'
    let wus = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Login", "status": "implementing", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": [],
    "implementing": ["AUTH-001"], "validating": [], "done": [], "blocked": []
  }
}"#;
    write_file(ws.path(), "spec/work-units.json", wus);

    // @step When I run `./codelet/target/release/fspec show-feature auth` from that workspace
    let (code, stdout, stderr) = run_show_feature(ws.path(), &["auth"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-feature must exit 0 on tagged feature; stderr={stderr}"
    );

    // @step And stdout contains the exact line '  AUTH-001 (feature-level) - Login'
    assert!(
        stdout.lines().any(|l| l == "  AUTH-001 (feature-level) - Login"),
        "stdout must contain exact AUTH-001 header line; got:\n{stdout}"
    );

    // @step And stdout contains the exact line '    auth.feature:4 - A'
    assert!(
        stdout.lines().any(|l| l == "    auth.feature:4 - A"),
        "stdout must contain exact scenario line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-feature --help is byte-for-byte identical to TS formatCommandHelp reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_SF: &str = include_str!("fixtures/help/show-feature.txt");

#[test]
fn scenario_show_feature_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec show-feature --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-feature")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-feature --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "show-feature --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-feature.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SF);

    // @step And stdout starts with a blank line followed by 'SHOW-FEATURE'
    assert!(stdout.starts_with("\nSHOW-FEATURE\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_show_feature() {
    // @step Given the fspec Rust binary has show-feature registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and list-features

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);

    // @step Then the command exits 0
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step And the help output lists daemon, client, status, list-work-units, list-prefixes, and show-feature as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "show-feature",
    ] {
        assert!(
            help.contains(sub),
            "fspec --help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
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
    // @step Given a temp workspace contains spec/features/auth.feature tagged '@AUTH-001' at the feature level with scenario 'A' on line 4 and spec/work-units.json contains AUTH-001 with title 'Login' and status 'done'
    let ws = tempfile::tempdir().expect("tempdir");
    let body = "@AUTH-001\nFeature: Auth\n\nScenario: A\n  Given step a\n";
    write_file(ws.path(), "spec/features/auth.feature", body);
    let wus = r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Login", "status": "done", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": [], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": ["AUTH-001"], "blocked": []
  }
}"#;
    write_file(ws.path(), "spec/work-units.json", wus);

    // @step When I dispatch show-feature through fspec_core::dispatch::dispatch_command with feature='auth' and format='text' against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-feature".to_string(),
        args_json: r#"{"feature":"auth","format":"text"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step And I run `./codelet/target/release/fspec show-feature auth` against the same workspace
    let (code, stdout, stderr) = run_show_feature(ws.path(), &["auth"]);
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}");

    // @step Then both invocations produce the exact line '  AUTH-001 (feature-level) - Login'
    assert!(
        result
            .data
            .lines()
            .any(|l| l == "  AUTH-001 (feature-level) - Login"),
        "dispatcher data must contain canonical line; got:\n{}",
        result.data
    );
    assert!(
        stdout
            .lines()
            .any(|l| l == "  AUTH-001 (feature-level) - Login"),
        "CLI stdout must contain canonical line; got:\n{stdout}"
    );

    // @step And the CLI bridge module codelet/fspec/src/show_feature.rs contains NO inline gherkin parsing, work-unit aggregation, or text rendering — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_feature.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/show_feature.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Work Units:",
        "Work Units: None",
        "feature-level",
        "scenario-level",
        "Invalid Gherkin syntax",
        "Feature file not found",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
