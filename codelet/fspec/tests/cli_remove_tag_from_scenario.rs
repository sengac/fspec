//! CLI surface for the `remove-tag-from-scenario` subcommand on the standalone fspec
//! Rust binary — RPC-282.
//!
//! Feature: spec/features/remove-tag-from-scenario-cli-subcommand.feature
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

fn run_rm(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-tag-from-scenario");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-tag-from-scenario");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let p = project_root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).expect("mkdir parents");
    fs::write(&p, body).expect("write feature");
}

fn read_feature(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature")
}

fn scenario_login_with_tags(tags: &[&str]) -> String {
    let mut s = String::from("Feature: Login\n\n");
    for t in tags {
        s.push_str(&format!("  {t}\n"));
    }
    s.push_str(
        "  Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n",
    );
    s
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-tag-from-scenario.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_remove_tag_from_scenario_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec remove-tag-from-scenario --help`
    let output = Command::new(fspec_bin())
        .arg("remove-tag-from-scenario")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-tag-from-scenario --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "remove-tag-from-scenario --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/remove-tag-from-scenario.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'REMOVE-TAG-FROM-SCENARIO'
    assert!(stdout.starts_with("\nREMOVE-TAG-FROM-SCENARIO\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully removes a tag and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_removes_tag_and_prints_success_line() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", &scenario_login_with_tags(&["@smoke", "@critical"]));

    // @step When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical` in that tempdir
    let (code, stdout, stderr) = run_rm(
        ws.path(),
        &["spec/features/login.feature", "Login", "@critical"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring "✓ Removed @critical from scenario 'Login'"
    assert!(
        stdout.contains("✓ Removed @critical from scenario 'Login'"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/features/login.feature on disk shows the Login scenario tagged @smoke
    let body = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @smoke\n  Scenario: Login\n"),
        "expected @smoke alone above Scenario; got:\n{body}"
    );
    assert!(!body.contains("@critical"), "@critical must be gone");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI variadic positional collects multiple tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_variadic_positional_collects_multiple_tags() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical @wip
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", &scenario_login_with_tags(&["@smoke", "@critical", "@wip"]));

    // @step When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical @wip` in that tempdir
    let (code, stdout, stderr) = run_rm(
        ws.path(),
        &["spec/features/login.feature", "Login", "@critical", "@wip"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring "✓ Removed @critical, @wip from scenario 'Login'"
    assert!(
        stdout.contains("✓ Removed @critical, @wip from scenario 'Login'"),
        "stdout must contain canonical multi-tag success; got:\n{stdout}"
    );

    // @step And spec/features/login.feature on disk shows the Login scenario tagged @smoke
    let body = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @smoke\n  Scenario: Login\n"),
        "expected @smoke above Scenario; got:\n{body}"
    );
    assert!(!body.contains("@critical"));
    assert!(!body.contains("@wip"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI idempotent path for non-matching tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_idempotent_path_for_non_matching_tags() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", &scenario_login_with_tags(&["@smoke"]));
    let pre = fs::read(ws.path().join("spec/features/login.feature")).unwrap();

    // @step When I run `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical` in that tempdir
    let (code, stdout, stderr) = run_rm(
        ws.path(),
        &["spec/features/login.feature", "Login", "@critical"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "idempotent path must exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring "No changes made - none of the specified tags found on scenario 'Login'"
    assert!(
        stdout.contains("No changes made - none of the specified tags found on scenario 'Login'"),
        "stdout must contain idempotent message; got:\n{stdout}"
    );

    // @step And spec/features/login.feature on disk is byte-equal to its pre-call contents
    let post = fs::read(ws.path().join("spec/features/login.feature")).unwrap();
    assert_eq!(pre, post);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports missing feature file with exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_missing_feature_file_with_exit_1() {
    // @step Given an empty project root directory with no spec/features/missing.feature
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/features/missing.feature").exists());

    // @step When I run `fspec remove-tag-from-scenario spec/features/missing.feature "Login" @smoke` in that tempdir
    let (code, _stdout, stderr) = run_rm(
        ws.path(),
        &["spec/features/missing.feature", "Login", "@smoke"],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must have Error: prefix; got:\n{stderr}");

    // @step And stderr contains the substring 'File not found: spec/features/missing.feature'
    assert!(
        stderr.contains("File not found: spec/features/missing.feature"),
        "stderr must mention missing file; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke @critical
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/login.feature", &scenario_login_with_tags(&["@smoke", "@critical"]));

    // @step When I dispatch remove-tag-from-scenario via fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' scenario='Login' tags=['@smoke']
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-tag-from-scenario".to_string(),
        args_json: r#"{"file":"spec/features/login.feature","scenario":"Login","tags":["@smoke"]}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher must succeed; got {result:?}");

    // @step And running `fspec remove-tag-from-scenario spec/features/login.feature "Login" @critical` afterwards exits 0
    let (code, stdout, stderr) = run_rm(
        ws.path(),
        &["spec/features/login.feature", "Login", "@critical"],
    );
    assert_eq!(code, 0, "CLI must succeed; stdout={stdout}, stderr={stderr}");

    // @step And spec/features/login.feature on disk shows the Login scenario with no tag lines immediately above it
    let body = read_feature(ws.path(), "spec/features/login.feature");
    let idx = body.find("  Scenario: Login\n").expect("Scenario line");
    let prefix = &body[..idx];
    let last_line = prefix.rsplit('\n').nth(1).unwrap_or("");
    assert!(
        !last_line.trim_start().starts_with('@'),
        "no tag line should remain immediately above Scenario; prev='{last_line}'; got:\n{body}"
    );

    // @step And the CLI bridge module codelet/fspec/src/remove_tag_from_scenario.rs contains NO inline scenario lookup, line-walk filter, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_tag_from_scenario.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_tag_from_scenario.rs must exist; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "parse_feature_lenient",
        "Scenario:",
        "File not found:",
        "no changes made",
        "Removed",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
