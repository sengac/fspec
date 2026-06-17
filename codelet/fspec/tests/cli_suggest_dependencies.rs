//! CLI surface for the `suggest-dependencies` subcommand on the standalone
//! fspec Rust binary — RPC-309.
//!
//! Feature: spec/features/suggest-dependencies-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand + help intercept + bridge module are
//! wired (Phase C), these tests fail — the binary rejects the unknown
//! subcommand and the bridge module does not yet exist.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::Value;

// ───────── helpers ─────────

fn run_suggest(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("suggest-dependencies");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec suggest-dependencies");
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

/// AUTH-001 and AUTH-002 with no relationships (sequential candidate).
fn work_units_auth_sequential() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "Work one", "type": "story", "status": "backlog", "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "Work two", "type": "story", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#
    .to_string()
}

// ───────── scenarios ─────────

#[test]
fn scenario_clap_exposes_suggest_dependencies_with_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec suggest-dependencies --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("suggest-dependencies")
        .arg("--help")
        .output()
        .expect("spawn suggest-dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "suggest-dependencies --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'suggest-dependencies'
    assert!(
        stdout.contains("suggest-dependencies") || stdout.contains("SUGGEST-DEPENDENCIES"),
        "help must describe the suggest-dependencies subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_without_options_prints_empty_sentinel_against_empty_workspace() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec suggest-dependencies` from that directory
    let (code, stdout, stderr) = run_suggest(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring 'No dependency suggestions found.'
    assert!(
        stdout.contains("No dependency suggestions found."),
        "got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_with_output_json_prints_empty_suggestions_array() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec suggest-dependencies --output json` from that directory
    let (code, stdout, stderr) = run_suggest(ws.path(), &["--output", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout parses as JSON whose root object has suggestions=[]
    let parsed: Value = serde_json::from_str(stdout.trim()).expect("stdout must be JSON");
    assert_eq!(
        parsed["suggestions"],
        serde_json::json!([]),
        "got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_text_output_lists_a_sequential_suggestion() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 and AUTH-002 with no relationships
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_auth_sequential());

    // @step When I run `./codelet/target/release/fspec suggest-dependencies` from that workspace
    let (code, stdout, stderr) = run_suggest(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring 'Found 1 dependency suggestion(s):'
    assert!(
        stdout.contains("Found 1 dependency suggestion(s):"),
        "got:\n{stdout}"
    );

    // @step And stdout contains the substring 'AUTH-002'
    assert!(stdout.contains("AUTH-002"), "got:\n{stdout}");

    // @step And stdout contains the substring 'Confidence: MEDIUM'
    assert!(stdout.contains("Confidence: MEDIUM"), "got:\n{stdout}");
}

#[test]
fn scenario_cli_exits_1_and_writes_to_stderr_when_work_units_json_is_malformed() {
    // @step Given spec/work-units.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), "{ not json");

    // @step When I run `./codelet/target/release/fspec suggest-dependencies --output json` from that directory
    let (code, stdout, stderr) = run_suggest(ws.path(), &["--output", "json"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr:\n{stderr}");

    // @step And stderr contains the substring 'Failed to parse work-units.json'
    assert!(
        stderr.contains("Failed to parse work-units.json"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 and AUTH-002 with no relationships
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_auth_sequential());

    // @step When I dispatch suggest-dependencies through fspec_core::dispatch::dispatch_command with output='json' against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "suggest-dependencies".to_string(),
        args_json: r#"{"output":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    let dispatcher_json: Value =
        serde_json::from_str(&result.data).expect("dispatcher data must be JSON");

    // @step And I run `./codelet/target/release/fspec suggest-dependencies --output json` against the same workspace
    let (code, stdout, stderr) = run_suggest(ws.path(), &["--output", "json"]);
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}");
    let cli_json: Value = serde_json::from_str(stdout.trim()).expect("CLI stdout must be JSON");

    // @step Then both invocations produce JSON with a suggestion from='AUTH-002' to='AUTH-001'
    for (label, value) in [("dispatcher", &dispatcher_json), ("cli", &cli_json)] {
        let suggestions = value["suggestions"]
            .as_array()
            .unwrap_or_else(|| panic!("{label} suggestions must be an array"));
        assert!(
            suggestions
                .iter()
                .any(|s| s["from"] == serde_json::json!("AUTH-002")
                    && s["to"] == serde_json::json!("AUTH-001")),
            "{label} must contain AUTH-002->AUTH-001; got {value}"
        );
    }

    // @step And the CLI bridge module codelet/fspec/src/suggest_dependencies.rs contains NO inline suggestion logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/suggest_dependencies.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/suggest_dependencies.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "sequential IDs",
        "test work depends on build work",
        "infrastructure work",
        "specificMatches",
        "blockedBy",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/suggest-dependencies.txt");

#[test]
fn scenario_suggest_dependencies_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec suggest-dependencies --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("suggest-dependencies")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn suggest-dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "suggest-dependencies --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/suggest-dependencies.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'SUGGEST-DEPENDENCIES'
    assert!(stdout.starts_with("\nSUGGEST-DEPENDENCIES\n"));
}
