//! CLI surface for the `import-example-map` subcommand on the standalone
//! fspec Rust binary — RPC-238.
//!
//! Feature: spec/features/import-example-map-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and the success-path assertions fail (clap reports an
//! unrecognized subcommand). Once the subcommand is wired, the green-phase
//! assertions take over.

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
    cmd.arg("import-example-map");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec import-example-map");
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

fn write_import_file(cwd: &Path, name: &str, raw: &str) {
    fs::write(cwd.join(name), raw).expect("write import file");
}

fn specifying_store(id: &str) -> String {
    format!(
        r#"{{
  "version": "0.7.1",
  "workUnits": {{
    "{id}": {{ "id": "{id}", "title": "t", "status": "specifying", "createdAt": "x", "updatedAt": "x" }}
  }},
  "states": {{
    "backlog": [], "specifying": ["{id}"], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }}
}}"#
    )
}

const IMPORT_FILE: &str = r#"{ "rules": ["r1", "r2"], "examples": ["e1"], "questions": [], "assumptions": [] }"#;

// ─────────────────────────────────────────────────────────────────────────
// Scenarios
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_import_example_map_appends_data_and_prints_success() {
    // @step Given a workspace whose spec/work-units.json has AUTH-001 in specifying state and an import file
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &specifying_store("AUTH-001"));
    write_import_file(ws.path(), "emap.json", IMPORT_FILE);

    // @step When I run `fspec import-example-map AUTH-001 emap.json`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["AUTH-001", "emap.json"]);

    // @step Then the command exits with code 0
    assert_eq!(code, 0, "expected exit 0; stdout={stdout}, stderr={stderr}");

    // @step And stdout contains "✓ Imported"
    assert!(
        stdout.contains("✓ Imported"),
        "stdout must contain success message; got:\n{stdout}"
    );

    // @step And spec/work-units.json now contains the imported items under AUTH-001
    let raw = fs::read_to_string(ws.path().join("spec/work-units.json")).expect("read work-units");
    let data: serde_json::Value = serde_json::from_str(&raw).expect("work-units is JSON");
    assert_eq!(
        data["workUnits"]["AUTH-001"]["rules"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        data["workUnits"]["AUTH-001"]["examples"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

#[test]
fn scenario_cli_import_example_map_fails_for_unknown_work_unit() {
    // @step Given a workspace whose spec/work-units.json does not contain NOPE-999 and an import file
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &specifying_store("AUTH-001"));
    write_import_file(ws.path(), "emap.json", IMPORT_FILE);

    // @step When I run `fspec import-example-map NOPE-999 emap.json`
    let (code, stdout, stderr) = run_cmd(ws.path(), &["NOPE-999", "emap.json"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "expected exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains "✗ Failed to import example map: Work unit 'NOPE-999' does not exist"
    assert!(
        stderr.contains("✗ Failed to import example map: Work unit 'NOPE-999' does not exist"),
        "stderr must contain the canonical failure message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_import_example_map_requires_file_argument() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec import-example-map AUTH-001`
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with a non-zero code
    assert_ne!(code, 0, "missing argument must produce a non-zero exit");

    // @step And stderr reports a missing required argument
    let lc = stderr.to_lowercase();
    assert!(
        lc.contains("required") || lc.contains("missing") || lc.contains("usage"),
        "stderr must report a missing required argument; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_import_example_map_help_matches_fixture() {
    // @step Given an empty workspace
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec import-example-map --help`
    let output = Command::new(fspec_bin())
        .arg("import-example-map")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .current_dir(ws.path())
        .output()
        .expect("spawn fspec import-example-map --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then stdout matches the captured import-example-map help fixture
    assert_eq!(code, 0, "help must exit 0; stderr={stderr}");
    let fixture = include_str!("fixtures/help/import-example-map.txt");
    assert_eq!(stdout, fixture);
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json has AUTH-001 in specifying state and an import file
    let ws_cli = tempfile::tempdir().expect("tempdir cli");
    let ws_disp = tempfile::tempdir().expect("tempdir disp");
    write_work_units(ws_cli.path(), &specifying_store("AUTH-001"));
    write_work_units(ws_disp.path(), &specifying_store("AUTH-001"));
    write_import_file(ws_cli.path(), "emap.json", IMPORT_FILE);
    write_import_file(ws_disp.path(), "emap.json", IMPORT_FILE);

    // @step When I import AUTH-001 via the CLI and via the dispatcher into separate stores
    let (cli_code, _o, _e) = run_cmd(ws_cli.path(), &["AUTH-001", "emap.json"]);
    assert_eq!(cli_code, 0, "CLI import must succeed");

    let req = codelet_fspec_core::DispatchRequest {
        command: "import-example-map".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","file":"emap.json"}"#.to_string(),
        project_root: ws_disp.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then both stores have identical AUTH-001 example map data
    let cli_data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(ws_cli.path().join("spec/work-units.json")).unwrap(),
    )
    .unwrap();
    let disp_data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(ws_disp.path().join("spec/work-units.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        cli_data["workUnits"]["AUTH-001"]["rules"],
        disp_data["workUnits"]["AUTH-001"]["rules"]
    );
    assert_eq!(
        cli_data["workUnits"]["AUTH-001"]["examples"],
        disp_data["workUnits"]["AUTH-001"]["examples"]
    );
}
