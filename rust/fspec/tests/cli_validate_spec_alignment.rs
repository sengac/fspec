//! CLI surface for the `validate-spec-alignment` subcommand on the standalone
//! fspec Rust binary — RPC-323.
//!
//! Feature: spec/features/validate-spec-alignment-cli-subcommand.feature
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

fn run_validate(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("validate-spec-alignment");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec validate-spec-alignment");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, ids: &[&str]) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let mut wus = serde_json::Map::new();
    for id in ids {
        wus.insert(
            (*id).to_string(),
            serde_json::json!({ "id": id, "title": format!("title {id}"), "status": "backlog" }),
        );
    }
    let payload = serde_json::json!({ "workUnits": Value::Object(wus), "states": {} });
    fs::write(
        spec.join("work-units.json"),
        serde_json::to_string_pretty(&payload).unwrap(),
    )
    .expect("write work-units.json");
}

fn write_feature(cwd: &Path, name: &str, content: &str) {
    let dir = cwd.join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir spec/features");
    fs::write(dir.join(name), content).expect("write feature");
}

const FEATURE_TAGGED_AUTH_001: &str =
    "Feature: Auth\n\n  @AUTH-001\n  Scenario: logs in\n    Given x\n";

// ───────── scenarios ─────────

#[test]
fn scenario_clap_exposes_validate_spec_alignment_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec validate-spec-alignment --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("validate-spec-alignment")
        .arg("--help")
        .output()
        .expect("spawn validate-spec-alignment --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "validate-spec-alignment --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'validate-spec-alignment'
    assert!(
        stdout.contains("validate-spec-alignment") || stdout.contains("VALIDATE-SPEC-ALIGNMENT"),
        "help must describe the validate-spec-alignment subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_exits_0_and_prints_success_when_the_work_unit_has_tagged_scenarios() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 and a feature file with '@AUTH-001' before a 'Scenario:' line
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &["AUTH-001"]);
    write_feature(ws.path(), "auth.feature", FEATURE_TAGGED_AUTH_001);

    // @step When I run `./rust/target/release/fspec validate-spec-alignment AUTH-001` from that workspace
    let (code, stdout, stderr) = run_validate(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring '✓'
    assert!(stdout.contains('✓'), "got:\n{stdout}");
}

#[test]
fn scenario_cli_exits_1_and_prints_the_warning_when_no_scenarios_are_tagged() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 and no scenario tagged '@AUTH-001'
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &["AUTH-001"]);
    write_feature(
        ws.path(),
        "other.feature",
        "Feature: Other\n\n  @OTHER-001\n  Scenario: x\n    Given x\n",
    );

    // @step When I run `./rust/target/release/fspec validate-spec-alignment AUTH-001` from that workspace
    let (code, stdout, stderr) = run_validate(ws.path(), &["AUTH-001"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}");

    // @step And stderr contains the substring 'No scenarios for AUTH-001'
    assert!(
        stderr.contains("No scenarios for AUTH-001"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn scenario_cli_exits_1_when_the_work_unit_does_not_exist() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 but not MISSING-999
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &["AUTH-001"]);

    // @step When I run `./rust/target/release/fspec validate-spec-alignment MISSING-999` from that workspace
    let (code, stdout, stderr) = run_validate(ws.path(), &["MISSING-999"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr:\n{stderr}");

    // @step And stderr contains the substring 'Work unit MISSING-999 not found'
    assert!(
        stderr.contains("Work unit MISSING-999 not found"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace whose spec/work-units.json contains AUTH-001 and a feature file with '@AUTH-001' before a 'Scenario:' line
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &["AUTH-001"]);
    write_feature(ws.path(), "auth.feature", FEATURE_TAGGED_AUTH_001);

    // @step When I dispatch validate-spec-alignment through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "validate-spec-alignment".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_json: Value =
        serde_json::from_str(&result.data).expect("dispatcher data must be JSON");

    // @step And I run `./rust/target/release/fspec validate-spec-alignment AUTH-001` against the same workspace
    let (code, stdout, stderr) = run_validate(ws.path(), &["AUTH-001"]);

    // @step Then both invocations agree the work unit is valid
    assert_eq!(
        dispatcher_json["valid"],
        serde_json::json!(true),
        "dispatcher must report valid=true; got {}",
        result.data
    );
    assert_eq!(
        code, 0,
        "CLI must exit 0 (valid); got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And the CLI bridge module rust/fspec/src/validate_spec_alignment.rs contains NO inline scan logic — its only computation is JSON arg marshalling and stdout/stderr printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/validate_spec_alignment.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/validate_spec_alignment.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    // Strip comment lines before scanning: the standard `//! Feature:
    // spec/features/<name>.feature` doc header legitimately mentions
    // `.feature`, which would false-positive the no-inline-scan guard
    // (Batch 10 lesson: substring assertions on bridge source must strip
    // comments first). We only care about executable code.
    let bridge_code: String = bridge_src
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "No scenarios for",
        "Scenario:",
        "scenariosFound",
        ".feature",
        "glob",
    ] {
        assert!(
            !bridge_code.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_code}"
        );
    }
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/validate-spec-alignment.txt");

#[test]
fn scenario_validate_spec_alignment_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec validate-spec-alignment --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("validate-spec-alignment")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate-spec-alignment --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "validate-spec-alignment --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/validate-spec-alignment.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'VALIDATE-SPEC-ALIGNMENT'
    assert!(stdout.starts_with("\nVALIDATE-SPEC-ALIGNMENT\n"));
}
