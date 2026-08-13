//! CLI surface for the `set-user-story` subcommand on the standalone
//! fspec Rust binary — RPC-298.
//!
//! Feature: spec/features/set-user-story-cli-subcommand.feature
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

fn run_set_user_story(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("set-user-story");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec set-user-story");
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

fn read_work_units(project_root: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(project_root.join("spec").join("work-units.json"))
        .expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

fn seed_unit(id: &str, status: &str) -> String {
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
        let arr: Vec<serde_json::Value> = if *st == status {
            vec![serde_json::Value::String(id.to_string())]
        } else {
            vec![]
        };
        states.insert((*st).to_string(), serde_json::Value::Array(arr));
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": {
            id: {
                "id": id,
                "title": "title",
                "type": "story",
                "status": status,
                "createdAt": "2026-06-01T00:00:00.000Z",
                "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": serde_json::Value::Object(states),
    }))
    .unwrap()
}

const TS_HELP_FIXTURE_SUS: &str = include_str!("fixtures/help/set-user-story.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes set-user-story with positional and required flags in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_set_user_story_with_positional_and_required_flags() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec set-user-story --help`
    let output = Command::new(fspec_bin())
        .arg("set-user-story")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec set-user-story --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec set-user-story --help must exit 0; stderr={stderr}"
    );

    // @step And stdout describes the set-user-story subcommand
    assert!(
        stdout.contains("set-user-story")
            || stdout.contains("SET-USER-STORY")
            || stdout.contains("user story"),
        "help must describe the set-user-story subcommand; got:\n{stdout}"
    );

    // @step And stdout mentions the `--role` flag
    assert!(
        stdout.contains("--role"),
        "help must mention --role; got:\n{stdout}"
    );

    // @step And stdout mentions the `--action` flag
    assert!(
        stdout.contains("--action"),
        "help must mention --action; got:\n{stdout}"
    );

    // @step And stdout mentions the `--benefit` flag
    assert!(
        stdout.contains("--benefit"),
        "help must mention --benefit; got:\n{stdout}"
    );

    // @step And stdout does NOT advertise a `--workspace` global flag
    assert!(
        !stdout.contains("--workspace"),
        "set-user-story --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI writes the user story and prints the success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_writes_the_user_story_and_prints_the_success_block() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001' with no userStory
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I run `./rust/target/release/fspec set-user-story AUTH-001 --role developer --action ship --benefit happiness`
    let (code, stdout, stderr) = run_set_user_story(
        ws.path(),
        &[
            "AUTH-001",
            "--role",
            "developer",
            "--action",
            "ship",
            "--benefit",
            "happiness",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the line '✓ User story set for AUTH-001'
    assert!(
        stdout.lines().any(|l| l == "✓ User story set for AUTH-001"),
        "stdout must contain '✓ User story set for AUTH-001'; got:\n{stdout}"
    );

    // @step And stdout contains the line '  As a developer'
    assert!(
        stdout.lines().any(|l| l == "  As a developer"),
        "stdout must contain '  As a developer'; got:\n{stdout}"
    );

    // @step And stdout contains the line '  I want to ship'
    assert!(
        stdout.lines().any(|l| l == "  I want to ship"),
        "stdout must contain '  I want to ship'; got:\n{stdout}"
    );

    // @step And stdout contains the line '  So that happiness'
    assert!(
        stdout.lines().any(|l| l == "  So that happiness"),
        "stdout must contain '  So that happiness'; got:\n{stdout}"
    );

    // @step And spec/work-units.json work unit 'AUTH-001' has userStory.role='developer'
    let v = read_work_units(ws.path());
    let us = &v["workUnits"]["AUTH-001"]["userStory"];
    assert_eq!(us["role"].as_str(), Some("developer"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an unknown work unit with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_unknown_work_unit_with_exit_1_and_error_prefix() {
    // @step Given spec/work-units.json contains no work unit 'MISSING-001'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I run `./rust/target/release/fspec set-user-story MISSING-001 --role x --action y --benefit z`
    let (code, _stdout, stderr) = run_set_user_story(
        ws.path(),
        &[
            "MISSING-001",
            "--role",
            "x",
            "--action",
            "y",
            "--benefit",
            "z",
        ],
    );

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Work unit 'MISSING-001' does not exist"
    assert!(
        stderr.contains("Work unit 'MISSING-001' does not exist"),
        "stderr must contain canonical missing message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/work-units.json contains work unit 'AUTH-001'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &seed_unit("AUTH-001", "specifying"));

    // @step When I dispatch set-user-story via fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' role='dev' action='go' benefit='win'
    let req = codelet_fspec_core::DispatchRequest {
        command: "set-user-story".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","role":"dev","action":"go","benefit":"win"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher writes spec/work-units.json
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let v = read_work_units(ws.path());
    let us = &v["workUnits"]["AUTH-001"]["userStory"];
    assert_eq!(us["role"].as_str(), Some("dev"));
    assert_eq!(us["action"].as_str(), Some("go"));
    assert_eq!(us["benefit"].as_str(), Some("win"));

    // @step And the CLI bridge module rust/fspec/src/set_user_story.rs contains NO inline userStory build, file-write, or success-line rendering — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/set_user_story.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/set_user_story.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "userStory\":",
        "does not exist",
        "✓ User story set for",
        "  As a ",
        "  I want to ",
        "  So that ",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: set-user-story --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_set_user_story_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec set-user-story --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("set-user-story")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn set-user-story --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "set-user-story --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/set-user-story.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SUS);

    // @step And stdout starts with a blank line followed by 'SET-USER-STORY'
    assert!(stdout.starts_with("\nSET-USER-STORY\n"));
}
