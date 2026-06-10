//! CLI surface for the `update-tag` subcommand on the standalone fspec
//! Rust binary — RPC-316.
//!
//! Feature: spec/features/update-tag-cli-subcommand.feature
//!
//! These tests exercise the wired-up clap subcommand and the ported
//! `codelet/fspec-core/src/commands/update_tag.rs` implementation
//! through the standalone fspec binary. Each scenario maps 1:1 to a
//! Gherkin scenario in the feature file above; @step comments mirror
//! the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_update_tag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("update-tag");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec update-tag");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_tags(project_root: &Path, raw: &str) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

const FIXTURE_CRITICAL: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]},
    {"name": "Priority Tags", "description": "pri", "required": false, "tags": []}
  ]
}"#;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI updates description in place and prints multi-line success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_updates_description_in_place_and_prints_success_block() {
    // @step Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), FIXTURE_CRITICAL);

    // @step When I run 'fspec update-tag @critical --description "Critical paths only"' in that tempdir
    let (code, stdout, stderr) = run_update_tag(
        ws.path(),
        &["@critical", "--description", "Critical paths only"],
    );

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "fspec update-tag must exit 0 on the happy path; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Successfully updated @critical'
    assert!(
        stdout.contains("✓ Successfully updated @critical"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Updated: spec/tags.json'
    assert!(
        stdout.contains("Updated: spec/tags.json"),
        "stdout must contain Updated line; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Regenerated: spec/TAGS.md'
    assert!(
        stdout.contains("Regenerated: spec/TAGS.md"),
        "stdout must contain Regenerated line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI rejects missing updates with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_rejects_missing_updates_with_error_prefix_and_exit_1() {
    // @step Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), FIXTURE_CRITICAL);

    // @step When I run 'fspec update-tag @critical' in that tempdir
    let (code, stdout, stderr) = run_update_tag(ws.path(), &["@critical"]);

    // @step Then the process exits with code 1
    assert_eq!(
        code, 1,
        "fspec update-tag must exit 1 when no updates; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'No updates specified'
    assert!(
        stderr.contains("No updates specified"),
        "stderr must contain 'No updates specified'; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI moves tag between categories with --category flag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_moves_tag_between_categories_with_category_flag() {
    // @step Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags and an empty Priority Tags category
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), FIXTURE_CRITICAL);

    // @step When I run 'fspec update-tag @critical --category "Priority Tags"' in that tempdir
    let (code, stdout, stderr) = run_update_tag(
        ws.path(),
        &["@critical", "--category", "Priority Tags"],
    );

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "fspec update-tag must exit 0 on cross-category move; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Successfully updated @critical'
    assert!(
        stdout.contains("✓ Successfully updated @critical"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the Priority Tags category on disk contains a tag named '@critical'
    let raw = fs::read_to_string(ws.path().join("spec/tags.json")).expect("read tags.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    let priority = v["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .find(|c| c["name"].as_str() == Some("Priority Tags"))
        .expect("Priority Tags present");
    let names: Vec<String> = priority["tags"]
        .as_array()
        .expect("tags array")
        .iter()
        .filter_map(|t| t["name"].as_str().map(String::from))
        .collect();
    assert!(
        names.iter().any(|n| n == "@critical"),
        "Priority Tags must contain @critical after move; got: {names:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_UT: &str = include_str!("fixtures/help/update-tag.txt");

#[test]
fn cli_update_tag_help_output_matches_captured_typescript_fixture() {
    // @step Given the standalone fspec Rust binary is built
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run 'fspec update-tag --help'
    let output = Command::new(fspec_bin())
        .arg("update-tag")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn update-tag --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "update-tag --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/update-tag.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_UT);
}
