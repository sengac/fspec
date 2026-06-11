//! CLI surface for the `delete-diagram` subcommand on the standalone
//! fspec Rust binary — RPC-216.
//!
//! Feature: spec/features/delete-diagram-cli-subcommand.feature
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

fn run_del_diag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-diagram");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-diagram");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(cwd: &Path, value: &serde_json::Value) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        serde_json::to_string_pretty(value).expect("ser"),
    )
    .expect("write foundation.json");
}

fn read_foundation(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec/foundation.json"))
        .expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn foundation_with_diagrams(titles: &[&str]) -> serde_json::Value {
    let diagrams: Vec<serde_json::Value> = titles
        .iter()
        .map(|t| serde_json::json!({"title": *t, "mermaidCode": format!("graph TD\n  {t}-->X")}))
        .collect();
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "fspec", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": diagrams
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes delete-diagram with two positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_delete_diagram_with_positional_args_in_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec delete-diagram --help`
    let output = Command::new(fspec_bin())
        .arg("delete-diagram")
        .arg("--help")
        .output()
        .expect("spawn delete-diagram --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "delete-diagram --help must exit 0; stderr={stderr}");

    // @step And stdout describes the delete-diagram subcommand
    assert!(
        stdout.contains("delete-diagram") || stdout.contains("DELETE-DIAGRAM"),
        "help must describe delete-diagram; got:\n{stdout}"
    );

    // @step And stdout mentions the `<section>` argument
    assert!(
        stdout.contains("section"),
        "help must mention section; got:\n{stdout}"
    );

    // @step And stdout mentions the `<title>` argument
    assert!(
        stdout.contains("title"),
        "help must mention title; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI removes a diagram by title and prints the success block on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_removes_diagram_by_title_and_prints_success_block() {
    // @step Given spec/foundation.json contains a diagram titled 'Component Flow'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_diagrams(&["Component Flow"]));

    // @step When I run `./codelet/target/release/fspec delete-diagram Architecture "Component Flow"`
    let (code, stdout, stderr) =
        run_del_diag(ws.path(), &["Architecture", "Component Flow"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line "✓ Deleted diagram 'Component Flow' from section 'Architecture'"
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Deleted diagram 'Component Flow' from section 'Architecture'")),
        "missing Deleted line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Updated: spec/foundation.json'
    assert!(
        stdout.contains("  Updated: spec/foundation.json"),
        "missing Updated status line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Regenerated: spec/FOUNDATION.md'
    assert!(
        stdout.contains("  Regenerated: spec/FOUNDATION.md"),
        "missing Regenerated status line; got:\n{stdout}"
    );

    // @step And spec/foundation.json architectureDiagrams is empty
    let data = read_foundation(ws.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 0);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when foundation.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_foundation_missing() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec delete-diagram Architecture "X"`
    let (code, _stdout, stderr) = run_del_diag(ws.path(), &["Architecture", "X"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'foundation.json not found: spec/foundation.json'
    assert!(
        stderr.contains("foundation.json not found: spec/foundation.json"),
        "stderr must mention missing foundation; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when title is not found
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_title_not_found() {
    // @step Given spec/foundation.json contains a diagram titled 'Existing'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_diagrams(&["Existing"]));

    // @step When I run `./codelet/target/release/fspec delete-diagram Architecture "Missing"`
    let (code, _stdout, stderr) =
        run_del_diag(ws.path(), &["Architecture", "Missing"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Diagram 'Missing' not found in section 'Architecture'"
    assert!(
        stderr.contains("Diagram 'Missing' not found in section 'Architecture'"),
        "stderr must mention missing diagram; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/foundation.json contains diagrams titled 'A' and 'B'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &foundation_with_diagrams(&["A", "B"]));

    // @step When I dispatch delete-diagram via fspec_core::dispatch::dispatch_command with section='Architecture' title='A'
    let req = codelet_fspec_core::DispatchRequest {
        command: "delete-diagram".to_string(),
        args_json: r#"{"section":"Architecture","title":"A"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then the dispatcher writes spec/foundation.json
    assert!(ws.path().join("spec/foundation.json").exists());

    // @step And running `./codelet/target/release/fspec delete-diagram Architecture "B"` afterwards exits 0
    let (code, stdout, stderr) = run_del_diag(ws.path(), &["Architecture", "B"]);
    assert_eq!(code, 0, "CLI delete must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/foundation.json architectureDiagrams is empty
    let data = read_foundation(ws.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 0);

    // @step And the CLI bridge module codelet/fspec/src/delete_diagram.rs contains NO inline file IO, JSON-parse, or splice logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/delete_diagram.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/delete_diagram.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "architectureDiagrams",
        "write_json_atomic",
        "ensure_foundation_file",
        "splice",
        "remove(",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: delete-diagram --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/delete-diagram.txt");

#[test]
fn scenario_delete_diagram_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec delete-diagram --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("delete-diagram")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-diagram --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "delete-diagram --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/delete-diagram.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
