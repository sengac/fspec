//! CLI surface for the `add-diagram` subcommand on the standalone
//! fspec Rust binary — RPC-178.
//!
//! Feature: spec/features/add-diagram-cli-subcommand.feature
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

fn run_add_diag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-diagram");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-diagram");
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
    let raw = fs::read_to_string(cwd.join("spec/foundation.json")).expect("read foundation.json");
    serde_json::from_str(&raw).expect("parse foundation.json")
}

fn empty_foundation() -> serde_json::Value {
    serde_json::json!({
        "version": "2.0.0",
        "project": {"name": "x", "vision": "v", "projectType": "cli-tool"},
        "architectureDiagrams": []
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes add-diagram with three positional args in --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_add_diagram_with_positional_args_in_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec add-diagram --help`
    let output = Command::new(fspec_bin())
        .arg("add-diagram")
        .arg("--help")
        .output()
        .expect("spawn add-diagram --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "add-diagram --help must exit 0; stderr={stderr}");

    // @step And stdout describes the add-diagram subcommand
    assert!(
        stdout.contains("add-diagram") || stdout.contains("ADD-DIAGRAM"),
        "help must describe add-diagram; got:\n{stdout}"
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

    // @step And stdout mentions the `<code>` argument
    assert!(
        stdout.contains("code"),
        "help must mention code; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds a new diagram and prints the success block on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_new_diagram_and_prints_success_block() {
    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I run `./rust/target/release/fspec add-diagram Architecture "Component" "graph TD\n  A-->B"`
    let (code, stdout, stderr) = run_add_diag(
        ws.path(),
        &["Architecture", "Component", "graph TD\n  A-->B"],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Added diagram "Component"'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Added diagram \"Component\"")),
        "missing Added line; got:\n{stdout}"
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

    // @step And spec/foundation.json architectureDiagrams contains exactly one entry titled 'Component'
    let data = read_foundation(ws.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 1);
    assert_eq!(diagrams[0]["title"].as_str(), Some("Component"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI replaces an existing diagram and prints the Updated success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_replaces_existing_diagram_and_prints_updated_block() {
    // @step Given spec/foundation.json contains a diagram titled 'System Overview' with mermaidCode='graph LR\n  Old-->X'
    let ws = tempfile::tempdir().expect("tempdir");
    let mut f = empty_foundation();
    f["architectureDiagrams"] = serde_json::json!([
        {"title": "System Overview", "mermaidCode": "graph LR\n  Old-->X"}
    ]);
    write_foundation(ws.path(), &f);

    // @step When I run `./rust/target/release/fspec add-diagram Architecture "System Overview" "graph LR\n  New-->X"`
    let (code, stdout, stderr) = run_add_diag(
        ws.path(),
        &["Architecture", "System Overview", "graph LR\n  New-->X"],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the line '✓ Updated diagram "System Overview"'
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("✓ Updated diagram \"System Overview\"")),
        "missing Updated line; got:\n{stdout}"
    );

    // @step And the diagram titled 'System Overview' now has mermaidCode='graph LR\n  New-->X'
    let data = read_foundation(ws.path());
    let d = data["architectureDiagrams"]
        .as_array()
        .expect("array")
        .iter()
        .find(|d| d["title"].as_str() == Some("System Overview"))
        .expect("must exist");
    assert_eq!(d["mermaidCode"].as_str(), Some("graph LR\n  New-->X"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an empty code argument with exit 1 and stderr Error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_empty_code_with_exit_1() {
    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I run `./rust/target/release/fspec add-diagram Architecture "X" ""`
    let (code, _stdout, stderr) = run_add_diag(ws.path(), &["Architecture", "X", ""]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Diagram code cannot be empty'
    assert!(
        stderr.contains("Diagram code cannot be empty"),
        "stderr must mention empty code; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects an invalid mermaid subgraph identifier
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_invalid_subgraph_identifier() {
    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I run `./rust/target/release/fspec add-diagram Architecture "Bad" "graph TB\n  subgraph Id!!!\n  end"`
    let (code, _stdout, stderr) = run_add_diag(
        ws.path(),
        &["Architecture", "Bad", "graph TB\n  subgraph Id!!!\n  end"],
    );

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring "Invalid subgraph identifier 'Id!!!'"
    assert!(
        stderr.contains("Invalid subgraph identifier 'Id!!!'"),
        "stderr must mention invalid subgraph; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/foundation.json exists with architectureDiagrams=[]
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &empty_foundation());

    // @step When I dispatch add-diagram via fspec_core::dispatch::dispatch_command with section='Architecture' title='Via dispatcher' code='graph TD\n  A-->B'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-diagram".to_string(),
        args_json:
            r#"{"section":"Architecture","title":"Via dispatcher","code":"graph TD\n  A-->B"}"#
                .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher writes spec/foundation.json
    assert!(ws.path().join("spec/foundation.json").exists());

    // @step And running `./rust/target/release/fspec add-diagram Architecture "Via CLI" "graph TD\n  C-->D"` afterwards exits 0
    let (code, stdout, stderr) =
        run_add_diag(ws.path(), &["Architecture", "Via CLI", "graph TD\n  C-->D"]);
    assert_eq!(
        code, 0,
        "CLI add must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/foundation.json architectureDiagrams contains two entries
    let data = read_foundation(ws.path());
    let diagrams = data["architectureDiagrams"].as_array().expect("array");
    assert_eq!(diagrams.len(), 2, "expected two entries, got {diagrams:?}");

    // @step And the CLI bridge module rust/fspec/src/add_diagram.rs contains NO inline mermaid validation, ensure_foundation_file, or JSON-mutation logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_diagram.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/add_diagram.rs must exist; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge readable");
    for forbidden in [
        "subgraph",
        "architectureDiagrams",
        "write_json_atomic",
        "ensure_foundation_file",
        "mermaidCode",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-diagram --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-diagram.txt");

#[test]
fn scenario_add_diagram_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec add-diagram --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-diagram")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-diagram --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "add-diagram --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/add-diagram.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}
