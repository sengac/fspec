//! CLI surface for the `create-feature` subcommand on the standalone fspec
//! Rust binary — RPC-212.
//!
//! Feature: spec/features/create-feature-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file
//! above; @step comments mirror the Gherkin step text verbatim. At PHASE B
//! time the command is still a stub, so these tests are expected to FAIL
//! until PHASE C lands the implementation and the shared wiring.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::{fspec_bin, strip_comments};

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_create_feature(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("create-feature");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec create-feature");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn empty_spec(project_root: &Path) {
    fs::create_dir_all(project_root.join("spec").join("features")).expect("mkdir spec/features");
}

const TS_HELP_FIXTURE_CF: &str = include_str!("fixtures/help/create-feature.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI creates a feature file and prints the success lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_creates_feature_file_and_prints_success_lines() {
    // @step Given a tempdir with an empty spec directory
    let ws = tempfile::tempdir().expect("tempdir");
    empty_spec(ws.path());

    // @step When I run 'fspec create-feature "Payment Processing"' in that tempdir
    let (code, stdout, stderr) = run_create_feature(ws.path(), &["Payment Processing"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Created features/payment-processing.feature'
    // NOTE: TS `createFeatureCommand` derives the printed name via
    // `result.filePath.split('/').slice(-2).join('/')`, which yields only the
    // LAST TWO path segments (`features/<file>`), NOT `spec/features/<file>`.
    // The Rust bridge's `short_path` mirrors this exactly. Substring match also
    // tolerates the trailing prefill `<system-reminder>` block (12 placeholders
    // in the template) that the command appends to stdout — same as TS.
    assert!(
        stdout.contains("✓ Created features/payment-processing.feature"),
        "stdout must contain success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Edit the file to add your scenarios'
    assert!(
        stdout.contains("Edit the file to add your scenarios"),
        "stdout must contain edit hint; got:\n{stdout}"
    );

    // @step And the file spec/features/payment-processing.feature exists in the tempdir
    assert!(ws
        .path()
        .join("spec/features/payment-processing.feature")
        .exists());
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the prefill system-reminder on stdout
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_the_prefill_system_reminder_on_stdout() {
    // @step Given a tempdir with an empty spec directory
    let ws = tempfile::tempdir().expect("tempdir");
    empty_spec(ws.path());

    // @step When I run 'fspec create-feature "Payment Processing"' in that tempdir
    let (code, stdout, stderr) = run_create_feature(ws.path(), &["Payment Processing"]);

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '<system-reminder>'
    assert!(
        stdout.contains("<system-reminder>"),
        "stdout must contain reminder opener; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'PREFILL DETECTED'
    assert!(
        stdout.contains("PREFILL DETECTED"),
        "stdout must contain prefill banner; got:\n{stdout}"
    );

    // @step And stdout contains the substring '</system-reminder>'
    assert!(
        stdout.contains("</system-reminder>"),
        "stdout must contain reminder closer; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 and Error prefix when the file already exists
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_file_already_exists() {
    // @step Given a tempdir whose spec/features/payment-processing.feature already exists
    let ws = tempfile::tempdir().expect("tempdir");
    empty_spec(ws.path());
    fs::write(
        ws.path().join("spec/features/payment-processing.feature"),
        "KEEP ME\n",
    )
    .expect("write existing");

    // @step When I run 'fspec create-feature "Payment Processing"' in that tempdir
    let (code, _stdout, stderr) = run_create_feature(ws.path(), &["Payment Processing"]);

    // @step Then the process exits with code 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'File already exists: spec/features/payment-processing.feature'
    assert!(
        stderr.contains("File already exists: spec/features/payment-processing.feature"),
        "stderr must contain canonical already-exists message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the standalone fspec Rust binary is built

    // @step When I run 'fspec create-feature --help'
    let output = Command::new(fspec_bin())
        .arg("create-feature")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn create-feature --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "create-feature --help must exit 0; stderr={stderr}"
    );

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/create-feature.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CF);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with an empty spec directory
    let ws = tempfile::tempdir().expect("tempdir");
    empty_spec(ws.path());

    // @step When I dispatch create-feature through fspec_core::dispatch::dispatch_command with name='User Authentication'
    let req = codelet_fspec_core::DispatchRequest {
        command: "create-feature".to_string(),
        args_json: r#"{"name":"User Authentication"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher's DispatchResult.data parses to a structure whose filePath ends with 'spec/features/user-authentication.feature'
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value = serde_json::from_str(&result.data).expect("parse data json");
    let file_path = data["filePath"].as_str().unwrap_or("");
    assert!(
        file_path.ends_with("spec/features/user-authentication.feature"),
        "unexpected filePath: {file_path}"
    );

    // @step And the CLI bridge module codelet/fspec/src/create_feature.rs contains NO inline template, kebab-case, coverage, or prefill logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/create_feature.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/create_feature.rs must exist as the CLI bridge module"
    );
    let bridge_src =
        strip_comments(&fs::read_to_string(&bridge_path).expect("bridge module readable"));
    for forbidden in [
        "generateFeatureTemplate",
        "feature_template",
        "to_kebab_case",
        "CoverageFile",
        "detect_prefill",
        "@critical @component",
        "[precondition]",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }

    // @step And the bridge module's only computation is JSON arg marshalling and CWD resolution
    // (Asserted indirectly by the forbidden-token sweep above.)
}
