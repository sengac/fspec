//! CLI surface for the `validate-tags` subcommand on the standalone fspec
//! Rust binary — RPC-324.
//!
//! Feature: spec/features/validate-tags-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the clap subcommand + core impl are not yet wired,
//! so these tests are RED until PHASE C.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::{json, Value};

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_validate_tags(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("validate-tags");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec validate-tags");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_tags(root: &Path, component: &[&str], feature_group: &[&str]) {
    let spec = root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    let to_tags = |names: &[&str]| -> Vec<Value> {
        names
            .iter()
            .map(|n| json!({ "name": n, "description": "x" }))
            .collect()
    };
    let data = json!({
        "categories": [
            { "name": "Component Tags", "description": "", "required": true, "tags": to_tags(component) },
            { "name": "Feature Group Tags", "description": "", "required": true, "tags": to_tags(feature_group) }
        ]
    });
    fs::write(
        spec.join("tags.json"),
        serde_json::to_string_pretty(&data).unwrap(),
    )
    .expect("write tags.json");
}

fn write_feature(root: &Path, rel: &str, body: &str) {
    let abs = root.join(rel);
    fs::create_dir_all(abs.parent().unwrap()).expect("mkdir feature parent");
    fs::write(&abs, body).expect("write feature file");
}

fn valid_feature(name: &str) -> String {
    format!("@comp @grp\nFeature: {name}\n\n  Scenario: A\n    Given x\n")
}

const TS_HELP_FIXTURE_VT: &str = include_str!("fixtures/help/validate-tags.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes validate-tags as a subcommand with file argument and flags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_validate_tags_with_byte_exact_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec validate-tags --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("validate-tags")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn validate-tags --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "validate-tags --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/validate-tags.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_VT);

    // @step And stdout starts with a blank line followed by 'VALIDATE-TAGS'
    assert!(stdout.starts_with("\nVALIDATE-TAGS\n"), "got:\n{stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 0 with no output for a single valid file and no flags
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_single_valid_file_no_flags_exits_0_no_output() {
    // @step Given spec/tags.json registers the tags used by a feature file including a component and feature-group tag
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);

    // @step Given a single feature file carries only registered tags
    write_feature(
        ws.path(),
        "spec/features/valid.feature",
        &valid_feature("Valid"),
    );

    // @step When I run `./rust/target/release/fspec validate-tags spec/features/valid.feature`
    let (code, stdout, stderr) = run_validate_tags(ws.path(), &["spec/features/valid.feature"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout is empty
    assert!(stdout.is_empty(), "stdout must be empty; got:\n{stdout}");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 and prints a violation block for an unregistered tag
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_unregistered_tag_exits_1_prints_violation_block() {
    // @step Given a feature file carries the unregistered feature-level tag '@made-up'
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);
    write_feature(
        ws.path(),
        "spec/features/bad.feature",
        "@comp @grp @made-up\nFeature: Bad\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I run `./rust/target/release/fspec validate-tags spec/features/bad.feature`
    let (code, stdout, stderr) = run_validate_tags(ws.path(), &["spec/features/bad.feature"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring 'has tag violations:'
    assert!(stdout.contains("has tag violations:"), "got:\n{stdout}");

    // @step Then stdout contains the substring 'Unregistered tag: @made-up'
    assert!(
        stdout.contains("Unregistered tag: @made-up"),
        "got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --verbose prints a passing line per valid file
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_verbose_prints_passing_line_per_valid_file() {
    // @step Given spec/tags.json registers the tags used by a feature file including a component and feature-group tag
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);

    // @step Given a single feature file carries only registered tags
    write_feature(
        ws.path(),
        "spec/features/valid.feature",
        &valid_feature("Valid"),
    );

    // @step When I run `./rust/target/release/fspec validate-tags spec/features/valid.feature --verbose`
    let (code, stdout, stderr) =
        run_validate_tags(ws.path(), &["spec/features/valid.feature", "--verbose"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ All tags in spec/features/valid.feature are registered'
    assert!(
        stdout.contains("✓ All tags in spec/features/valid.feature are registered"),
        "got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --summary prints only the summary count lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_summary_prints_only_summary_count_lines() {
    // @step Given two feature files where one has an unregistered tag
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);
    write_feature(
        ws.path(),
        "spec/features/good.feature",
        &valid_feature("Good"),
    );
    write_feature(
        ws.path(),
        "spec/features/bad.feature",
        "@comp @grp @made-up\nFeature: Bad\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I run `./rust/target/release/fspec validate-tags --summary`
    let (code, stdout, stderr) = run_validate_tags(ws.path(), &["--summary"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step Then stdout contains the substring 'files passed'
    assert!(stdout.contains("files passed"), "got:\n{stdout}");

    // @step Then stdout contains the substring 'files have tag violations'
    assert!(
        stdout.contains("files have tag violations"),
        "got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring 'has tag violations:'
    assert!(
        !stdout.contains("has tag violations:"),
        "summary mode must suppress per-file blocks; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root with spec/tags.json and a feature file carrying an unregistered tag
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &["@comp"], &["@grp"]);
    write_feature(
        ws.path(),
        "spec/features/bad.feature",
        "@comp @grp @made-up\nFeature: Bad\n\n  Scenario: A\n    Given x\n",
    );

    // @step When I dispatch validate-tags through fspec_core::dispatch::dispatch_command and also run `./rust/target/release/fspec validate-tags` against the same on-disk state
    let req = codelet_fspec_core::DispatchRequest {
        command: "validate-tags".to_string(),
        args_json: "{}".to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let data: Value = serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then both paths agree the file is invalid
    assert_eq!(
        data["invalidCount"].as_u64(),
        Some(1),
        "dispatcher must report invalidCount=1; got {data}"
    );
    let (code, _stdout, _stderr) = run_validate_tags(ws.path(), &[]);
    assert_eq!(code, 1, "CLI must also exit 1 for the same invalid state");

    // @step Then the CLI bridge module rust/fspec/src/validate_tags.rs contains NO inline validation or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/validate_tags.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/validate_tags.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Unregistered tag:",
        "has tag violations:",
        "Missing required component tag",
        "Placeholder tag:",
        "validateFileTags",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
