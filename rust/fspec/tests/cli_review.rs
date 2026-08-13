//! CLI surface for the `review` subcommand on the standalone fspec Rust binary
//! — RPC-295.
//!
//! Feature: spec/features/review-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE (Phase B): the `review` clap subcommand is not wired until Phase C
//! and the core impl is still the 1-arg NotYetPorted stub, so these tests are
//! EXPECTED to fail until then.
//!
//! Per the supervisor ruling, `review` follows the delete-scenarios
//! SPECIAL-CASE: bare clap-generated --help (NO rich byte-parity fixture).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

use serde_json::json;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir parent");
    }
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {rel}: {e}"));
}

/// Scaffold a project root with a reviewable AUTH-001 work unit: a
/// work-units.json store, a linked feature tagged @AUTH-001, a clean test
/// file and a 100%-coverage file.
fn scaffold_reviewable(root: &Path) {
    let doc = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-06-01T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001", "title": "Login", "type": "story", "status": "validating",
                "rules": [ { "id": 0, "text": "a rule", "deleted": false } ],
                "stateHistory": [ { "state": "specifying", "timestamp": "2026-06-01T00:00:00.000Z" } ],
                "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [], "testing": [],
            "implementing": [], "validating": ["AUTH-001"], "done": [], "blocked": []
        }
    });
    write_file(root, "spec/work-units.json", &doc.to_string());
    write_file(
        root,
        "spec/features/user-login.feature",
        "@AUTH-001\nFeature: User Login\n\n  Scenario: Login\n    Given a user\n    When they log in\n    Then they see the dashboard\n",
    );
    write_file(root, "tests/login.test.ts", "const x: string = 'ok';\n");
    write_file(
        root,
        "spec/features/user-login.feature.coverage",
        &json!({
            "stats": { "totalScenarios": 1, "coveredScenarios": 1, "coveragePercent": 100 },
            "scenarios": [ { "name": "Login", "testMappings": [ { "file": "tests/login.test.ts", "lines": "1-1" } ] } ]
        })
        .to_string(),
    );
}

/// Run `fspec review <args>` in `cwd` (project_root resolved from CWD).
fn run_review(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("review");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec review");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_review_subcommand_and_prints_bare_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec review --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("review")
        .arg("--help")
        .output()
        .expect("spawn fspec review --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "review --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'review'
    assert!(
        stdout.contains("review"),
        "help must mention review; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'work-unit-id'
    assert!(
        stdout.contains("<work-unit-id>"),
        "help must name the work-unit-id positional; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_reviews_an_existing_work_unit_and_prints_the_report() {
    // @step Given a project root whose work-units store contains the work unit being reviewed
    let ws = tempfile::tempdir().expect("tempdir");
    scaffold_reviewable(ws.path());

    // @step When I run `./rust/target/release/fspec review <id>` from that directory
    let (code, stdout, stderr) = run_review(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "review must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the substring 'REVIEW:'
    assert!(
        stdout.contains("REVIEW:"),
        "stdout must show the review header; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '## ACDD Compliance'
    assert!(
        stdout.contains("## ACDD Compliance"),
        "stdout must show the ACDD Compliance section; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '## Summary'
    assert!(
        stdout.contains("## Summary"),
        "stdout must show the Summary section; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_errors_on_a_missing_work_unit_id() {
    // @step Given a project root whose work-units store does not contain the id 'BOGUS-999'
    let ws = tempfile::tempdir().expect("tempdir");
    scaffold_reviewable(ws.path());

    // @step When I run `./rust/target/release/fspec review BOGUS-999` from that directory
    let (code, _stdout, stderr) = run_review(ws.path(), &["BOGUS-999"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "review of a missing id must exit 1");

    // @step Then stderr contains the substring "Error: Work unit 'BOGUS-999' does not exist"
    assert!(
        stderr.contains("Error: Work unit 'BOGUS-999' does not exist"),
        "stderr must surface the not-found error; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose work-units store contains the work unit being reviewed
    let ws = tempfile::tempdir().expect("tempdir");
    scaffold_reviewable(ws.path());

    // @step When I dispatch review through fspec_core::dispatch::dispatch_command with that work unit id against that project root
    let req = codelet_fspec_core::DispatchRequest {
        command: "review".to_string(),
        args_json: json!({ "workUnitId": "AUTH-001" }).to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    let (code, stdout, stderr) = run_review(ws.path(), &["AUTH-001"]);
    assert_eq!(code, 0, "CLI review must exit 0; stderr={stderr}");

    // @step Then the dispatcher result text equals the stdout produced by the CLI bridge for the same work unit
    assert_eq!(
        stdout.trim_end(),
        result.data.trim_end(),
        "CLI stdout must equal the dispatcher report text"
    );

    // @step And the CLI bridge module rust/fspec/src/review.rs contains NO review logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/review.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/review.rs must exist as the CLI bridge module"
    );
    let bridge_src = std::fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "## ACDD Compliance",
        "Critical Issues",
        "AI-DRIVEN DEEP CODE REVIEW",
        "Use of `any` type detected",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed review report logic (`{forbidden}`); got:\n{bridge_src}"
        );
    }
    assert!(
        bridge_src.contains("review::run") || bridge_src.contains("commands::review"),
        "bridge must delegate to fspec_core review::run; got:\n{bridge_src}"
    );
}
