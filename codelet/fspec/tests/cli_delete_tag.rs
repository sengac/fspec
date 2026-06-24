//! CLI surface for the `delete-tag` subcommand on the standalone fspec
//! Rust binary — RPC-222.
//!
//! Feature: spec/features/delete-tag-cli-subcommand.feature
//!
//! These tests exercise the wired-up clap subcommand and the ported
//! `codelet/fspec-core/src/commands/delete_tag.rs` implementation
//! through the standalone fspec binary. Each scenario maps 1:1 to a
//! Gherkin scenario in the feature file above; @step comments mirror
//! the Gherkin step text verbatim.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn run_delete_tag(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("delete-tag");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec delete-tag");
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

fn write_feature(project_root: &Path, rel_path: &str, body: &str) {
    let full = project_root.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("mkdir feature parent");
    }
    fs::write(&full, body).expect("write feature file");
}

const FIXTURE_DEPRECATED: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": []},
    {"name": "Status Tags", "description": "s", "required": false, "tags": [
      {"name": "@deprecated", "description": "Deprecated"}
    ]}
  ]
}"#;

const FIXTURE_CRITICAL_STATUS: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": []},
    {"name": "Status Tags", "description": "s", "required": false, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]}
  ]
}"#;

const FIXTURE_CRITICAL_PHASE: &str = r#"{
  "categories": [
    {"name": "Phase Tags", "description": "p", "required": true, "tags": [
      {"name": "@critical", "description": "Critical features"}
    ]}
  ]
}"#;

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes a tag and prints the multi-line success block
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_deletes_tag_and_prints_multi_line_success_block_when_no_feature_files_reference_it() {
    // @step Given a tempdir with spec/tags.json containing tag '@deprecated' under Status Tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), FIXTURE_DEPRECATED);

    // @step And no feature files in the tempdir reference '@deprecated'
    // (Directory absent — equivalent to "no matches".)

    // @step When I run 'fspec delete-tag @deprecated' in that tempdir
    let (code, stdout, stderr) = run_delete_tag(ws.path(), &["@deprecated"]);

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "fspec delete-tag must exit 0 on the happy path; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Successfully deleted tag @deprecated from registry'
    assert!(
        stdout.contains("✓ Successfully deleted tag @deprecated from registry"),
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

    // @step And spec/tags.json on disk in the tempdir no longer contains a tag named '@deprecated'
    let raw = fs::read_to_string(ws.path().join("spec/tags.json")).expect("read tags.json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
    let any_match = v["categories"]
        .as_array()
        .expect("categories array")
        .iter()
        .any(|c| {
            c["tags"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .any(|t| t["name"].as_str() == Some("@deprecated"))
                })
                .unwrap_or(false)
        });
    assert!(
        !any_match,
        "@deprecated must be removed from all categories"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI dry-run prints preview, skips Updated/Regenerated lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_dry_run_prints_preview_and_skips_trailing_lines() {
    // @step Given a tempdir with spec/tags.json containing tag '@critical' under Status Tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), FIXTURE_CRITICAL_STATUS);

    // @step When I run 'fspec delete-tag @critical --dry-run' in that tempdir
    let (code, stdout, stderr) = run_delete_tag(ws.path(), &["@critical", "--dry-run"]);

    // @step Then the process exits with code 0
    assert_eq!(
        code, 0,
        "fspec delete-tag --dry-run must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring '✓ Would delete tag @critical from category "Status Tags"'
    assert!(
        stdout.contains("✓ Would delete tag @critical from category \"Status Tags\""),
        "stdout must contain canonical dry-run preview; got:\n{stdout}"
    );

    // @step And stdout does not contain the substring 'Updated: spec/tags.json'
    assert!(
        !stdout.contains("Updated: spec/tags.json"),
        "stdout MUST suppress 'Updated:' line on dry-run; got:\n{stdout}"
    );

    // @step And stdout does not contain the substring 'Regenerated: spec/TAGS.md'
    assert!(
        !stdout.contains("Regenerated: spec/TAGS.md"),
        "stdout MUST suppress 'Regenerated:' line on dry-run; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI blocks deletion with stderr Error prefix and exit 1
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn cli_blocks_deletion_with_error_prefix_and_exit_1_when_tag_referenced_and_no_force() {
    // @step Given a tempdir with spec/tags.json containing tag '@critical' under Phase Tags
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), FIXTURE_CRITICAL_PHASE);

    // @step And spec/features/auth.feature in the tempdir contains the substring '@critical'
    write_feature(
        ws.path(),
        "spec/features/auth.feature",
        "@critical\nFeature: Auth\n  Scenario: ok\n    Given x\n",
    );

    // @step When I run 'fspec delete-tag @critical' in that tempdir
    let (code, stdout, stderr) = run_delete_tag(ws.path(), &["@critical"]);

    // @step Then the process exits with code 1
    assert_eq!(
        code, 1,
        "fspec delete-tag must exit 1 when tag in use; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Tag @critical is used in'
    assert!(
        stderr.contains("Tag @critical is used in"),
        "stderr must contain canonical usage-blocked substring; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Use --force to delete anyway'
    assert!(
        stderr.contains("Use --force to delete anyway"),
        "stderr must contain --force tail; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI help output matches captured TypeScript fixture byte-for-byte
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_DT: &str = include_str!("fixtures/help/delete-tag.txt");

#[test]
fn cli_delete_tag_help_output_matches_captured_typescript_fixture() {
    // @step Given the standalone fspec Rust binary is built
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run 'fspec delete-tag --help'
    let output = Command::new(fspec_bin())
        .arg("delete-tag")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn delete-tag --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the process exits with code 0
    assert_eq!(code, 0, "delete-tag --help must exit 0; stderr={stderr}");

    // @step And stdout matches the captured fixture at codelet/fspec/tests/fixtures/help/delete-tag.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_DT);
}
