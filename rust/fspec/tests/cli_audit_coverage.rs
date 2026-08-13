//! CLI surface for the `audit-coverage` subcommand on the standalone fspec
//! Rust binary — RPC-197.
//!
//! Feature: spec/features/audit-coverage-cli-subcommand.feature
//!
//! PHASE B (TESTING): the clap subcommand + CLI bridge are not yet wired,
//! so these tests are RED until PHASE C. Each scenario maps 1:1 to a
//! Gherkin scenario; @step comments mirror the step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_audit(cwd: &Path, feature: &str) -> (i32, String, String) {
    let output = Command::new(fspec_bin())
        .arg("audit-coverage")
        .arg(feature)
        .current_dir(cwd)
        .output()
        .expect("spawn fspec audit-coverage");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_coverage(root: &Path, feature: &str, scenarios_json: &str) {
    write_file(
        root,
        &format!("spec/features/{feature}.feature.coverage"),
        &format!("{{\n  \"scenarios\": {scenarios_json}\n}}"),
    );
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/audit-coverage.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes audit-coverage as a subcommand requiring a feature-name and printing byte-parity help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_audit_coverage_with_byte_parity_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec audit-coverage --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("audit-coverage")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec audit-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "audit-coverage --help must exit 0; stderr={stderr}"
    );

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/audit-coverage.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout starts with a blank line followed by 'AUDIT-COVERAGE'
    assert!(stdout.starts_with("\nAUDIT-COVERAGE\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports all files present and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_all_files_present_and_exits_0() {
    // @step Given a project root whose spec/features/user-login.feature.coverage references three files that all exist
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/a.test.ts", "// a\n");
    write_file(ws.path(), "src/a.ts", "// a impl\n");
    write_file(ws.path(), "tests/b.test.ts", "// b\n");
    write_coverage(
        ws.path(),
        "user-login",
        r#"[
    { "name": "A", "testMappings": [
      { "file": "tests/a.test.ts", "lines": "1-5", "implMappings": [ { "file": "src/a.ts", "lines": "1-3" } ] },
      { "file": "tests/b.test.ts", "lines": "1-5", "implMappings": [] }
    ] }
  ]"#,
    );

    // @step When I run `./rust/target/release/fspec audit-coverage user-login` from that directory
    let (code, stdout, stderr) = run_audit(ws.path(), "user-login");

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}, stdout={stdout}");

    // @step Then stdout contains the substring '✅ All files found (3/3)'
    assert!(
        stdout.contains("✅ All files found (3/3)"),
        "stdout={stdout}"
    );

    // @step Then stdout contains the substring 'All mappings valid'
    assert!(stdout.contains("All mappings valid"), "stdout={stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a missing test file and exits 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_missing_test_file_and_exits_1() {
    // @step Given a project root whose spec/features/user-login.feature.coverage maps to a test file that does not exist
    let ws = tempfile::tempdir().expect("tempdir");
    write_coverage(
        ws.path(),
        "user-login",
        r#"[
    { "name": "A", "testMappings": [
      { "file": "src/__tests__/deleted.test.ts", "lines": "1-5", "implMappings": [] }
    ] }
  ]"#,
    );

    // @step When I run `./rust/target/release/fspec audit-coverage user-login` from that directory
    let (code, stdout, stderr) = run_audit(ws.path(), "user-login");

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}, stdout={stdout}");

    // @step Then stdout contains the substring '❌ Test file not found:'
    assert!(
        stdout.contains("❌ Test file not found:"),
        "stdout={stdout}"
    );

    // @step Then stdout contains the substring 'Recommendation: Remove this mapping or restore the deleted file'
    assert!(
        stdout.contains("Recommendation: Remove this mapping or restore the deleted file"),
        "stdout={stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a missing coverage file and exits 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_missing_coverage_file_and_exits_1() {
    // @step Given a project root with no spec/features/user-login.feature.coverage file
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./rust/target/release/fspec audit-coverage user-login` from that directory
    let (code, stdout, stderr) = run_audit(ws.path(), "user-login");

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stderr={stderr}, stdout={stdout}");

    // @step Then stdout contains the substring '✗ Coverage file not found:'
    assert!(
        stdout.contains("✗ Coverage file not found:"),
        "stdout={stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/features/user-login.feature.coverage references files that all exist
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/a.test.ts", "// a\n");
    write_file(ws.path(), "src/a.ts", "// a impl\n");
    write_file(ws.path(), "tests/b.test.ts", "// b\n");
    write_coverage(
        ws.path(),
        "user-login",
        r#"[
    { "name": "A", "testMappings": [
      { "file": "tests/a.test.ts", "lines": "1-5", "implMappings": [ { "file": "src/a.ts", "lines": "1-3" } ] },
      { "file": "tests/b.test.ts", "lines": "1-5", "implMappings": [] }
    ] }
  ]"#,
    );

    // @step When I dispatch audit-coverage through fspec_core::dispatch::dispatch_command for feature 'user-login'
    let req = codelet_fspec_core::DispatchRequest {
        command: "audit-coverage".to_string(),
        args_json: r#"{"featureName":"user-login"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data carries the same output and exitCode the CLI prints against the same on-disk state
    let (code, stdout, _stderr) = run_audit(ws.path(), "user-login");
    assert_eq!(code, dispatcher_data["exitCode"].as_i64().unwrap() as i32);
    assert!(stdout.contains("✅ All files found (3/3)"));
    assert!(dispatcher_data["output"]
        .as_str()
        .unwrap()
        .contains("✅ All files found (3/3)"));

    // @step Then the CLI bridge module rust/fspec/src/audit_coverage.rs contains NO inline file-existence or rendering logic — its only computation is JSON arg marshalling and envelope decoding
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/audit_coverage.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/audit_coverage.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "All files found",
        "Coverage file not found",
        "Test file not found",
        "Implementation file not found",
        "Recommendation: Remove this mapping",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic)"
        );
    }
}
