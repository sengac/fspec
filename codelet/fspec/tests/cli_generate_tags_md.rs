//! CLI surface for the `generate-tags-md` subcommand on the standalone fspec
//! Rust binary — RPC-236.
//!
//! Feature: spec/features/generate-tags-md-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario in the feature file above;
//! @step comments mirror the Gherkin step text verbatim.
//!
//! PHASE B (TESTING): the clap subcommand + bridge are not wired yet, so the
//! binary does not recognise `generate-tags-md`. These tests are RED until
//! PHASE C.

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
    cmd.arg("generate-tags-md");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec generate-tags-md");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// A fully schema-valid tags.json satisfying every required top-level key in
/// src/schemas/tags.schema.json.
fn valid_tags() -> serde_json::Value {
    serde_json::json!({
        "categories": [
            {
                "name": "Phase Tags",
                "description": "Phase identification tags",
                "required": true,
                "tags": [ { "name": "@critical", "description": "Critical priority" } ]
            }
        ],
        "combinationExamples": [
            { "title": "Example 1", "tags": "@cli @critical", "interpretation": ["CLI", "Critical"] }
        ],
        "usageGuidelines": {
            "requiredCombinations": { "title": "Required", "requirements": ["Must have a phase tag"], "minimumExample": "@phase-1" },
            "recommendedCombinations": { "title": "Recommended", "includes": ["component tag"], "recommendedExample": "@cli @phase-1" },
            "orderingConvention": { "title": "Ordering", "order": ["phase", "component"], "example": "@phase-1 @cli" }
        },
        "addingNewTags": {
            "process": [ { "step": "Step 1", "description": "Register the tag" } ],
            "namingConventions": ["lowercase-with-hyphens"],
            "antiPatterns": { "dont": [ { "description": "Create overlapping tags" } ], "do": [ { "description": "Reuse existing tags" } ] }
        },
        "queries": { "title": "Tag-Based Queries", "examples": [ { "description": "Find by tag", "command": "fspec list-tags" } ] },
        "statistics": {
            "lastUpdated": "2025-01-15T10:30:00Z",
            "phaseStats": [ { "phase": "Phase 1", "total": 5, "complete": 5, "inProgress": 0, "planned": 0 } ],
            "componentStats": [ { "component": "@cli", "count": 28, "percentage": "100%" } ],
            "featureGroupStats": [ { "featureGroup": "@validation", "count": 10, "percentage": "50%" } ],
            "updateCommand": "fspec tag-stats"
        },
        "validation": {
            "rules": [ { "rule": "Format", "description": "Tags must start with @" } ],
            "commands": [ { "description": "Validate", "command": "fspec validate-tags" } ]
        },
        "references": [ { "title": "Markdown tables", "url": "https://example.com/tables" } ]
    })
}

fn write_tags(project_root: &Path, value: &serde_json::Value) {
    let spec = project_root.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("tags.json"),
        serde_json::to_string_pretty(value).expect("ser tags"),
    )
    .expect("write tags.json");
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/generate-tags-md.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Help output matches the captured TS fixture
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_generate_tags_md_help_matches_ts_fixture() {
    // @step Given the fspec Rust binary is built and on PATH

    // @step When I run `fspec generate-tags-md --help`
    let output = Command::new(fspec_bin())
        .arg("generate-tags-md")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn generate-tags-md --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the exit code is 0
    assert_eq!(code, 0, "generate-tags-md --help must exit 0; stderr={stderr}");

    // @step And the stdout matches the canonical help fixture at codelet/fspec/tests/fixtures/help/generate-tags-md.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI generates TAGS.md and prints the success line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_generates_tags_md_and_prints_success_line() {
    // @step Given a project root tempdir with a schema-valid spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &valid_tags());

    // @step When I run `fspec generate-tags-md` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &[]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Generated spec/TAGS.md from spec/tags.json'
    assert!(
        stdout.contains("✓ Generated spec/TAGS.md from spec/tags.json"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file spec/TAGS.md is created in that tempdir
    assert!(
        ws.path().join("spec").join("TAGS.md").exists(),
        "spec/TAGS.md must be created"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI forwards the --output flag to a custom path
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_forwards_output_flag() {
    // @step Given a project root tempdir with a schema-valid spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &valid_tags());

    // @step When I run `fspec generate-tags-md --output docs/TAGS.md` in that tempdir
    let (code, stdout, stderr) = run_cmd(ws.path(), &["--output", "docs/TAGS.md"]);

    // @step Then the exit code is 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step And stdout contains the substring '✓ Generated docs/TAGS.md from spec/tags.json'
    assert!(
        stdout.contains("✓ Generated docs/TAGS.md from spec/tags.json"),
        "stdout must contain canonical success line; got:\n{stdout}"
    );

    // @step And the file docs/TAGS.md is created in that tempdir
    assert!(
        ws.path().join("docs").join("TAGS.md").exists(),
        "docs/TAGS.md must be created"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI reports a missing tags.json with exit 1 and the TS-parity prefix
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_reports_missing_tags_json_with_exit_1() {
    // @step Given an empty project root tempdir with no spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec generate-tags-md` in that tempdir
    let (code, _stdout, stderr) = run_cmd(ws.path(), &[]);

    // @step Then the exit code is 1
    assert_eq!(code, 1, "expected exit 1; stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain Error: prefix; got:\n{stderr}");

    // @step And stderr contains the substring 'tags.json not found: spec/tags.json'
    assert!(
        stderr.contains("tags.json not found: spec/tags.json"),
        "stderr must contain canonical missing-tags message; got:\n{stderr}"
    );

    // @step And the file spec/TAGS.md is not created
    assert!(
        !ws.path().join("spec").join("TAGS.md").exists(),
        "spec/TAGS.md must NOT be created"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function() {
    // @step Given a project root tempdir with a schema-valid spec/tags.json
    let ws = tempfile::tempdir().expect("tempdir");
    write_tags(ws.path(), &valid_tags());

    // @step When I dispatch generate-tags-md via fspec_core::dispatch::dispatch_command with no args
    let req = codelet_fspec_core::DispatchRequest {
        command: "generate-tags-md".to_string(),
        args_json: "{}".to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step And running `fspec generate-tags-md` afterwards exits 0
    let (code, stdout, stderr) = run_cmd(ws.path(), &[]);
    assert_eq!(code, 0, "CLI must succeed; stdout={stdout}, stderr={stderr}");

    // @step And the CLI bridge module codelet/fspec/src/generate_tags_md.rs contains NO inline markdown rendering, schema validation, or file-write logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generate_tags_md.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/generate_tags_md.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "AUTO-GENERATED",
        "Feature File Tag Registry",
        "validation errors",
        "write_json_atomic",
        "categories",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
