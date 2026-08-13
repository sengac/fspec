//! CLI surface for the `tag-stats` subcommand on the standalone fspec
//! Rust binary — RPC-310.
//!
//! Feature: spec/features/tag-stats-cli-subcommand.feature
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

fn run_tag_stats(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("tag-stats");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec tag-stats");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(cwd: &Path, rel: &str, content: &str) {
    let abs = cwd.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir");
    fs::write(&abs, content).expect("write feature");
}

fn write_tags_json(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("tags.json"), raw).expect("write tags.json");
}

fn feature_with_tags(tags: &[&str], name: &str) -> String {
    format!(
        "{}\nFeature: {name}\n  Scenario: A\n    Given x\n",
        tags.join(" ")
    )
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes tag-stats as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_tag_stats_with_flag_aware_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec tag-stats --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("tag-stats")
        .arg("--help")
        .output()
        .expect("spawn fspec tag-stats --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec tag-stats --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains a description of the tag-stats subcommand
    assert!(
        stdout.contains("tag-stats") || stdout.contains("TAG-STATS"),
        "help must describe the tag-stats subcommand; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--category'
    assert!(
        !stdout.contains("--category"),
        "tag-stats --help must NOT advertise --category; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "tag-stats --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "tag-stats --help must NOT advertise --workspace; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "tag-stats --help must NOT advertise --status; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty directory prints zero-totals output
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_directory_prints_zero_totals() {
    // @step Given an empty directory with no spec subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec tag-stats` from that directory
    let (code, stdout, stderr) = run_tag_stats(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec tag-stats must exit 0 on empty workspace; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Total feature files: 0'
    assert!(
        stdout.contains("Total feature files: 0"),
        "stdout must contain 'Total feature files: 0'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '⚠ Warning: spec/tags.json not found'
    assert!(
        stdout.contains("⚠ Warning: spec/tags.json not found"),
        "stdout must contain tags.json warning; got:\n{stdout}"
    );

    // @step Then spec/tags.json was NOT created in the directory
    assert!(
        !ws.path().join("spec/tags.json").exists(),
        "tag-stats must NOT auto-create spec/tags.json"
    );

    // @step Then spec/features/ was NOT created in the directory
    assert!(
        !ws.path().join("spec/features").exists(),
        "tag-stats must NOT auto-create spec/features/"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders category counts for the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_category_counts() {
    // @step Given spec/tags.json declares Phase Tags=[@critical, @high]
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags_json(
        ws.path(),
        r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": false,
      "tags": [
        { "name": "@critical", "description": "c" },
        { "name": "@high", "description": "h" }
      ]
    }
  ]
}"#,
    );

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    write_feature(
        ws.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step Given spec/features/b.feature has feature-level tags '@critical @high'
    write_feature(
        ws.path(),
        "spec/features/b.feature",
        &feature_with_tags(&["@critical", "@high"], "B"),
    );

    // @step When I run `./rust/target/release/fspec tag-stats` from the workspace
    let (code, stdout, stderr) = run_tag_stats(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec tag-stats must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Tag Usage Statistics'
    assert!(
        stdout.contains("Tag Usage Statistics"),
        "missing header; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Total feature files: 2'
    assert!(
        stdout.contains("Total feature files: 2"),
        "missing total files line; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Unique tags used: 2'
    assert!(
        stdout.contains("Unique tags used: 2"),
        "missing unique tags line; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Total tag occurrences: 3'
    assert!(
        stdout.contains("Total tag occurrences: 3"),
        "missing total occurrences line; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Phase Tags (2 tags)'
    assert!(
        stdout.contains("Phase Tags (2 tags)"),
        "missing 'Phase Tags (2 tags)'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output prints invalid-files warning
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_prints_invalid_files_warning() {
    // @step Given spec/features/bad.feature contains the bytes 'not gherkin'
    let ws = tempfile::tempdir().expect("tempdir");
    write_feature(ws.path(), "spec/features/bad.feature", "not gherkin\n");

    // @step When I run `./rust/target/release/fspec tag-stats` from the workspace
    let (code, stdout, stderr) = run_tag_stats(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec tag-stats must exit 0 with invalid file; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring '⚠ Warning: 1 file(s) with invalid syntax skipped:'
    assert!(
        stdout.contains("⚠ Warning: 1 file(s) with invalid syntax skipped:"),
        "missing invalid-files warning; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  - spec/features/bad.feature'
    assert!(
        stdout.lines().any(|l| l == "  - spec/features/bad.feature"),
        "missing bullet line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_tag_stats() {
    // @step Given the fspec Rust binary has tag-stats registered as a clap subcommand
    // (asserted by the help-listing check below)

    // @step When I run `./rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "fspec --help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists tag-stats as an available subcommand
    assert!(
        help.contains("tag-stats"),
        "fspec --help must list `tag-stats` subcommand; got:\n{help}"
    );

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given spec/tags.json declares Phase Tags=[@critical]
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags_json(
        ws.path(),
        r#"{
  "categories": [
    {
      "name": "Phase Tags",
      "description": "x",
      "required": false,
      "tags": [
        { "name": "@critical", "description": "c" }
      ]
    }
  ]
}"#,
    );

    // @step Given spec/features/a.feature has feature-level tags '@critical'
    write_feature(
        ws.path(),
        "spec/features/a.feature",
        &feature_with_tags(&["@critical"], "A"),
    );

    // @step When I dispatch tag-stats through fspec_core::dispatch::dispatch_command with format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "tag-stats".to_string(),
        args_json: r#"{"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher result has totalFiles=1, uniqueTags=1, totalOccurrences=1
    assert_eq!(data["totalFiles"].as_u64(), Some(1));
    assert_eq!(data["uniqueTags"].as_u64(), Some(1));
    assert_eq!(data["totalOccurrences"].as_u64(), Some(1));

    // @step Then running `./rust/target/release/fspec tag-stats` against the same on-disk state exits 0 and stdout reports the same counters
    let (code, stdout, _stderr) = run_tag_stats(ws.path(), &[]);
    assert_eq!(code, 0, "CLI must exit 0; stdout=\n{stdout}");
    assert!(
        stdout.contains("Total feature files: 1"),
        "CLI stdout must reflect total files; got:\n{stdout}"
    );
    assert!(
        stdout.contains("Unique tags used: 1"),
        "CLI stdout must reflect unique tags; got:\n{stdout}"
    );

    // @step Then the CLI bridge module rust/fspec/src/tag_stats.rs contains NO inline tag-counting, category-projection, or rendering logic
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tag_stats.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/tag_stats.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "tagCounts",
        "tag_counts",
        "uniqueTags",
        "totalOccurrences",
        "Tag Usage Statistics",
        "Tag Counts by Category",
        "Unused Registered Tags",
        "tags_file_found",
        "tagsFileFound",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: tag-stats --help is byte-for-byte identical to TS (RPC-310)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_TS: &str = include_str!("fixtures/help/tag-stats.txt");

#[test]
fn scenario_tag_stats_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec tag-stats --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("tag-stats")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn tag-stats --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "tag-stats --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/tag-stats.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_TS);

    // @step And stdout starts with a blank line followed by 'TAG-STATS'
    assert!(stdout.starts_with("\nTAG-STATS\n"));
}
