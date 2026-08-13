//! CLI surface for the `add-tag-to-scenario` subcommand on the standalone fspec
//! Rust binary — RPC-194.
//!
//! Feature: spec/features/add-tag-to-scenario-cli-subcommand.feature
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

fn run_add(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-tag-to-scenario");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-tag-to-scenario");
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

fn write_tags_json(project_root: &Path, body: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), body).expect("write tags.json");
}

fn scenario_login_no_tags() -> String {
    String::from(
        "Feature: Login\n\
         \n\
         \x20\x20Scenario: Login\n\
         \x20\x20\x20\x20Given a user\n\
         \x20\x20\x20\x20When the user logs in\n\
         \x20\x20\x20\x20Then the dashboard appears\n",
    )
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

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-tag-to-scenario.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_tag_to_scenario_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec add-tag-to-scenario --help`
    let output = Command::new(fspec_bin())
        .arg("add-tag-to-scenario")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-tag-to-scenario --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(
        code, 0,
        "add-tag-to-scenario --help must exit 0; stderr={stderr}"
    );

    // @step And the stdout matches the canonical help fixture at rust/fspec/tests/fixtures/help/add-tag-to-scenario.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'ADD-TAG-TO-SCENARIO'
    assert!(stdout.starts_with("\nADD-TAG-TO-SCENARIO\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI successfully adds a tag and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_successfully_adds_tag_and_prints_success_line() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        &scenario_login_no_tags(),
    );

    // @step When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @smoke` in that tempdir
    let (code, stdout, stderr) = run_add(
        ws.path(),
        &["spec/features/login.feature", "Login", "@smoke"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}; stdout={stdout}");

    // @step And stdout contains the substring "✓ Added @smoke to scenario 'Login'"
    assert!(
        stdout.contains("✓ Added @smoke to scenario 'Login'"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And spec/features/login.feature on disk shows a single '  @smoke' line immediately above the Scenario line
    let body = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @smoke\n  Scenario: Login\n"),
        "expected @smoke above Scenario; got:\n{body}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects duplicate tag with exit 1 and TS-parity error prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_rejects_duplicate_tag_with_exit_1_and_error_prefix() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' tagged @smoke
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        &scenario_login_with_tags(&["@smoke"]),
    );
    let pre = fs::read(ws.path().join("spec/features/login.feature")).unwrap();

    // @step When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @smoke` in that tempdir
    let (code, _stdout, stderr) = run_add(
        ws.path(),
        &["spec/features/login.feature", "Login", "@smoke"],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Tag @smoke already exists on this scenario'
    assert!(
        stderr.contains("Tag @smoke already exists on this scenario"),
        "stderr must contain duplicate-tag message; got:\n{stderr}"
    );

    // @step And spec/features/login.feature on disk is byte-equal to its pre-call contents
    let post = fs::read(ws.path().join("spec/features/login.feature")).unwrap();
    assert_eq!(pre, post);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI variadic positional collects multiple tags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_variadic_positional_collects_multiple_tags() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        &scenario_login_no_tags(),
    );

    // @step When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical @regression` in that tempdir
    let (code, stdout, stderr) = run_add(
        ws.path(),
        &[
            "spec/features/login.feature",
            "Login",
            "@critical",
            "@regression",
        ],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring "✓ Added @critical, @regression to scenario 'Login'"
    assert!(
        stdout.contains("✓ Added @critical, @regression to scenario 'Login'"),
        "stdout must contain canonical multi-tag success; got:\n{stdout}"
    );

    // @step And spec/features/login.feature on disk shows '  @critical' then '  @regression' immediately above the Scenario line
    let body = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @critical\n  @regression\n  Scenario: Login\n"),
        "expected critical then regression; got:\n{body}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --validate-registry rejects unregistered tag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_validate_registry_rejects_unregistered_tag() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags and spec/tags.json that does NOT register @unregistered
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        &scenario_login_no_tags(),
    );
    write_tags_json(
        ws.path(),
        r#"{
          "categories": [
            { "name": "Test Tags", "tags": [{ "name": "@something-else", "description": "x" }] }
          ]
        }"#,
    );
    let pre = fs::read(ws.path().join("spec/features/login.feature")).unwrap();

    // @step When I run `fspec add-tag-to-scenario spec/features/login.feature "Login" @unregistered --validate-registry` in that tempdir
    let (code, _stdout, stderr) = run_add(
        ws.path(),
        &[
            "spec/features/login.feature",
            "Login",
            "@unregistered",
            "--validate-registry",
        ],
    );

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring '@unregistered is not registered in spec/tags.json'
    assert!(
        stderr.contains("@unregistered is not registered in spec/tags.json"),
        "stderr must contain registry-missing message; got:\n{stderr}"
    );

    // @step And spec/features/login.feature on disk is byte-equal to its pre-call contents
    let post = fs::read(ws.path().join("spec/features/login.feature")).unwrap();
    assert_eq!(pre, post);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root tempdir with spec/features/login.feature containing a Scenario 'Login' with no tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(
        ws.path(),
        "spec/features/login.feature",
        &scenario_login_no_tags(),
    );

    // @step When I dispatch add-tag-to-scenario via fspec_core::dispatch::dispatch_command with file='spec/features/login.feature' scenario='Login' tags=['@smoke']
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-tag-to-scenario".to_string(),
        args_json: r#"{"file":"spec/features/login.feature","scenario":"Login","tags":["@smoke"]}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec add-tag-to-scenario spec/features/login.feature "Login" @critical` afterwards exits 0
    let (code, stdout, stderr) = run_add(
        ws.path(),
        &["spec/features/login.feature", "Login", "@critical"],
    );
    assert_eq!(
        code, 0,
        "CLI must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And spec/features/login.feature on disk shows '  @smoke' then '  @critical' immediately above the Scenario line
    let body = read_feature(ws.path(), "spec/features/login.feature");
    assert!(
        body.contains("\n  @smoke\n  @critical\n  Scenario: Login\n"),
        "expected @smoke then @critical; got:\n{body}"
    );

    // @step And the CLI bridge module rust/fspec/src/add_tag_to_scenario.rs contains NO inline tag-format validation, scenario lookup, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_tag_to_scenario.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/add_tag_to_scenario.rs must exist; missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "is_work_unit_tag",
        "parse_feature_lenient",
        "Scenario:",
        "already exists on this scenario",
        "File not found:",
        "Invalid tag format",
        "tags.json",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
