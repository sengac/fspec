//! CLI surface for the `retag` subcommand on the standalone fspec Rust binary
//! — RPC-293.
//!
//! Feature: spec/features/retag-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the clap subcommand + bridge are not wired yet, so the
//! binary does not recognise `retag`. These tests are RED until PHASE C.
//!
//! SURFACE NOTE: TS registers --from/--to/--dry-run FLAGS (not positionals);
//! the Rust clap surface mirrors the flags.

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
    cmd.arg("retag");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec retag");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(project_root: &Path, rel: &str, body: &str) {
    let abs = project_root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

/// A valid Gherkin feature file whose feature-level tag is `tag`.
fn tagged(tag: &str, name: &str) -> String {
    format!("{tag}\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

fn read(project_root: &Path, rel: &str) -> String {
    fs::read_to_string(project_root.join(rel)).expect("read feature file")
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/retag.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_retag_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec retag --help`
    let output = Command::new(fspec_bin())
        .arg("retag")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn retag --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "retag --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/retag.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI renames a tag and prints the success summary
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_renames_a_tag_and_prints_success_summary() {
    // @step Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    write_feature(ws.path(), "spec/features/b.feature", &tagged("@wip", "B"));

    // @step When I run `fspec retag --from @wip --to @in-progress` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--from", "@wip", "--to", "@in-progress"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓'
    assert!(
        stdout.contains('✓'),
        "stdout must contain check mark; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Modified files:'
    assert!(
        stdout.contains("Modified files:"),
        "stdout must list modified files; got:\n{stdout}"
    );

    // @step And neither feature file on disk contains the token '@wip' anymore
    assert!(!read(ws.path(), "spec/features/a.feature").contains("@wip"));
    assert!(!read(ws.path(), "spec/features/b.feature").contains("@wip"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI dry run prints the preview and modifies nothing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_dry_run_prints_preview_and_modifies_nothing() {
    // @step Given a project root tempdir with two spec/features feature files that each tag a scenario with @wip
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    write_feature(ws.path(), "spec/features/b.feature", &tagged("@wip", "B"));
    let pre_a = fs::read(ws.path().join("spec/features/a.feature")).unwrap();
    let pre_b = fs::read(ws.path().join("spec/features/b.feature")).unwrap();

    // @step When I run `fspec retag --from @wip --to @in-progress --dry-run` in that tempdir
    let (code, stdout, stderr) = run_cmd(
        ws.path(),
        &["--from", "@wip", "--to", "@in-progress", "--dry-run"],
    );

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Dry run mode - no files modified'
    assert!(
        stdout.contains("Dry run mode - no files modified"),
        "stdout must contain dry-run banner; got:\n{stdout}"
    );

    // @step And both feature files on disk are byte-equal to their pre-call contents
    assert_eq!(
        fs::read(ws.path().join("spec/features/a.feature")).unwrap(),
        pre_a
    );
    assert_eq!(
        fs::read(ws.path().join("spec/features/b.feature")).unwrap(),
        pre_b
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a not-found tag with exit 1 and the TS-parity prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_not_found_tag_with_exit_1() {
    // @step Given a project root tempdir with one spec/features feature file tagged @wip
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/a.feature", &tagged("@wip", "A"));
    let pre = fs::read(ws.path().join("spec/features/a.feature")).unwrap();

    // @step When I run `fspec retag --from @missing --to @found` in that tempdir
    let (code, _stdout, stderr) = run_cmd(ws.path(), &["--from", "@missing", "--to", "@found"]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain Error: prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Tag @missing not found in any feature files'
    assert!(
        stderr.contains("Tag @missing not found in any feature files"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );

    // @step And the feature file on disk is byte-equal to its pre-call contents
    assert_eq!(
        fs::read(ws.path().join("spec/features/a.feature")).unwrap(),
        pre
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function() {
    // @step Given a project root tempdir with one spec/features feature file tagged @wip
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/a.feature", &tagged("@wip", "A"));

    // @step When I dispatch retag via fspec_core::dispatch::dispatch_command with from='@wip' to='@done'
    let req = codelet_fspec_core::DispatchRequest {
        command: "retag".to_string(),
        args_json: r#"{"from":"@wip","to":"@done"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And running `fspec retag --from @done --to @wip` afterwards exits 0
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--from", "@done", "--to", "@wip"]);
    assert_eq!(
        code, 0,
        "CLI retag must succeed; stdout={stdout}, stderr={stderr}"
    );

    // @step And the CLI bridge module codelet/fspec/src/retag.rs contains NO inline glob, regex replace, Gherkin re-parse, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/retag.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/retag.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "glob_feature_files",
        "parse_feature_lenient",
        "Regex",
        "Invalid tag format",
        "not found in any feature files",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
