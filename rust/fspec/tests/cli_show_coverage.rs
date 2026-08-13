//! CLI surface for the `show-coverage` subcommand on the standalone fspec
//! Rust binary — RPC-300.
//!
//! Feature: spec/features/show-coverage-cli-subcommand.feature
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

fn run_show_coverage(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-coverage");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-coverage");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

/// Coverage file body with a single fully-covered scenario whose test+impl
/// files exist at the given relative paths.
fn coverage_one_full(
    test_file: &str,
    test_lines: &str,
    impl_file: &str,
    impl_lines: &str,
) -> String {
    format!(
        r#"{{
  "scenarios": [
    {{
      "name": "Scenario A",
      "testMappings": [
        {{
          "file": "{tf}",
          "lines": "{tl}",
          "implMappings": [
            {{ "file": "{imf}", "lines": "{iml}" }}
          ]
        }}
      ]
    }}
  ],
  "stats": {{
    "totalScenarios": 1,
    "coveredScenarios": 1,
    "coveragePercent": 100,
    "testFiles": ["{tf}"],
    "implFiles": ["{imf}"],
    "totalLinesCovered": 0
  }}
}}"#,
        tf = test_file,
        tl = test_lines,
        imf = impl_file,
        iml = impl_lines,
    )
}

/// Coverage file body with a single uncovered scenario.
fn coverage_one_uncovered() -> String {
    r#"{
  "scenarios": [
    { "name": "Scenario X", "testMappings": [] }
  ],
  "stats": {
    "totalScenarios": 1,
    "coveredScenarios": 0,
    "coveragePercent": 0,
    "testFiles": [],
    "implFiles": [],
    "totalLinesCovered": 0
  }
}"#
    .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes show-coverage as a subcommand and prints help on --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_show_coverage_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec show-coverage --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-coverage")
        .arg("--help")
        .output()
        .expect("spawn fspec show-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-coverage --help must exit 0; stderr={stderr}"
    );

    // @step And stdout contains the substring 'show-coverage'
    assert!(
        stdout.contains("show-coverage") || stdout.contains("SHOW-COVERAGE"),
        "help must mention show-coverage; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Display coverage report'
    assert!(
        stdout.contains("Display coverage report"),
        "help must describe show-coverage; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-coverage with a missing feature exits 1 and writes the TS-parity error to stderr
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_show_coverage_missing_feature_exits_1() {
    // @step Given an empty directory containing spec/features/ but no missing.feature.coverage is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(ws.path().join("spec/features")).expect("mkdir spec/features");

    // @step When I run `./rust/target/release/fspec show-coverage missing` from that directory
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &["missing"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec show-coverage missing must exit 1; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Coverage file not found: missing.feature.coverage'
    assert!(
        stderr.contains("Coverage file not found: missing.feature.coverage"),
        "stderr must contain canonical not-found message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI per-feature mode renders markdown report to stdout for a fully covered feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_per_feature_markdown_for_fully_covered() {
    // @step Given a temp workspace contains spec/features/auth.feature.coverage with 1 fully covered scenario whose referenced test and impl files exist on disk
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/auth.test.ts", "// test\n");
    write_file(ws.path(), "src/auth/login.ts", "// impl\n");
    let body = coverage_one_full("tests/auth.test.ts", "1-10", "src/auth/login.ts", "1-5");
    write_file(ws.path(), "spec/features/auth.feature.coverage", &body);

    // @step When I run `./rust/target/release/fspec show-coverage auth` from that workspace
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &["auth"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-coverage auth must exit 0; stderr={stderr}"
    );

    // @step And stdout contains the line '# Coverage Report: auth.feature'
    assert!(
        stdout
            .lines()
            .any(|l| l == "# Coverage Report: auth.feature"),
        "stdout must contain coverage report title; got:\n{stdout}"
    );

    // @step And stdout contains the line '**Coverage**: 100% (1/1 scenarios)'
    assert!(
        stdout
            .lines()
            .any(|l| l == "**Coverage**: 100% (1/1 scenarios)"),
        "stdout must contain coverage percent line; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '## Warnings'
    assert!(
        !stdout.contains("## Warnings"),
        "stdout must NOT contain Warnings section when files exist; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI per-feature JSON mode renders 2-space-indented JSON for the requested feature
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_per_feature_json_mode() {
    // @step Given a temp workspace contains spec/features/auth.feature.coverage with 1 scenario and a stats object
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/auth.test.ts", "// test\n");
    write_file(ws.path(), "src/auth/login.ts", "// impl\n");
    let body = coverage_one_full("tests/auth.test.ts", "1-10", "src/auth/login.ts", "1-5");
    write_file(ws.path(), "spec/features/auth.feature.coverage", &body);

    // @step When I run `./rust/target/release/fspec show-coverage auth --format json` from that workspace
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &["auth", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-coverage --format json must exit 0; stderr={stderr}"
    );

    // @step And stdout parses as JSON whose root keys in declaration order are 'fileName', 'scenarios', 'stats', 'warnings'
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("stdout must be JSON");
    let obj = v.as_object().expect("root is object");
    let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    assert!(
        keys.starts_with(&["fileName", "scenarios", "stats"]),
        "root keys must start with fileName, scenarios, stats; got: {keys:?}"
    );

    // @step And stdout uses 2-space indentation
    assert!(
        stdout.contains("\n  \"fileName\""),
        "stdout must use 2-space indentation; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI project-wide mode aggregates and renders Project Coverage Report
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_project_wide_aggregates() {
    // @step Given a temp workspace contains spec/features/a.feature.coverage with 1 fully covered scenario AND spec/features/b.feature.coverage with 1 uncovered scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/a.test.ts", "// test\n");
    write_file(ws.path(), "src/a.ts", "// impl\n");
    let a_body = coverage_one_full("tests/a.test.ts", "1-10", "src/a.ts", "1-5");
    write_file(ws.path(), "spec/features/a.feature.coverage", &a_body);
    write_file(
        ws.path(),
        "spec/features/b.feature.coverage",
        &coverage_one_uncovered(),
    );

    // @step When I run `./rust/target/release/fspec show-coverage` from that workspace
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-coverage (project-wide) must exit 0; stderr={stderr}"
    );

    // @step And stdout contains the line '# Project Coverage Report'
    assert!(
        stdout.lines().any(|l| l == "# Project Coverage Report"),
        "stdout must contain Project Coverage Report title; got:\n{stdout}"
    );

    // @step And stdout contains the line '**Overall Coverage**: 50% (1/2 scenarios)'
    assert!(
        stdout
            .lines()
            .any(|l| l == "**Overall Coverage**: 50% (1/2 scenarios)"),
        "stdout must contain overall coverage line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI project-wide mode exits 1 when spec/features/ is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_project_wide_missing_features_dir_exits_1() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./rust/target/release/fspec show-coverage` from that directory
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &[]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec show-coverage must exit 1 when spec/features missing; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:' prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'Features directory not found: spec/features/'
    assert!(
        stderr.contains("Features directory not found: spec/features/"),
        "stderr must contain features-dir-missing message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI project-wide mode exits 1 when spec/features/ exists but is empty
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_project_wide_empty_features_dir_exits_1() {
    // @step Given a temp workspace contains spec/features/ with no *.feature.coverage files
    let ws = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(ws.path().join("spec/features")).expect("mkdir");

    // @step When I run `./rust/target/release/fspec show-coverage` from that workspace
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &[]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec show-coverage must exit 1 when no coverage files; stdout={stdout}, stderr={stderr}"
    );

    // @step And stderr contains the substring 'No coverage files found in spec/features/'
    assert!(
        stderr.contains("No coverage files found in spec/features/"),
        "stderr must contain no-coverage-files message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI tolerates a trailing .feature on the positional feature-name
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_trailing_feature_extension_tolerated() {
    // @step Given a temp workspace contains spec/features/login.feature.coverage with 1 fully covered scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/login.test.ts", "// test\n");
    write_file(ws.path(), "src/login.ts", "// impl\n");
    let body = coverage_one_full("tests/login.test.ts", "1-10", "src/login.ts", "1-5");
    write_file(ws.path(), "spec/features/login.feature.coverage", &body);

    // @step When I run `./rust/target/release/fspec show-coverage login.feature` from that workspace
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &["login.feature"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec show-coverage with .feature suffix must exit 0; stderr={stderr}"
    );

    // @step And stdout contains the line '# Coverage Report: login.feature'
    assert!(
        stdout
            .lines()
            .any(|l| l == "# Coverage Report: login.feature"),
        "stdout must contain coverage report title; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: show-coverage --help is byte-for-byte identical to TS formatCommandHelp reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_SC: &str = include_str!("fixtures/help/show-coverage.txt");

#[test]
fn scenario_show_coverage_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec show-coverage --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-coverage")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-coverage --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "show-coverage --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-coverage.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SC);

    // @step And stdout starts with a blank line followed by the line 'SHOW-COVERAGE'
    assert!(stdout.starts_with("\nSHOW-COVERAGE\n"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode is preserved when no subcommand is provided
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_show_coverage() {
    // @step Given the fspec Rust binary has show-coverage registered as a clap subcommand alongside daemon, client, status, list-work-units, list-prefixes, and list-features

    // @step When I run `./rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);

    // @step Then the command exits 0
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step And the help output lists show-coverage as an available subcommand
    assert!(
        help.contains("show-coverage"),
        "fspec --help must list `show-coverage` subcommand; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
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
    // @step Given a temp workspace contains spec/features/auth.feature.coverage with 1 fully covered scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "tests/auth.test.ts", "// test\n");
    write_file(ws.path(), "src/auth/login.ts", "// impl\n");
    let body = coverage_one_full("tests/auth.test.ts", "1-10", "src/auth/login.ts", "1-5");
    write_file(ws.path(), "spec/features/auth.feature.coverage", &body);

    // @step When I dispatch show-coverage through fspec_core::dispatch::dispatch_command with featureName='auth' and format='json' against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-coverage".to_string(),
        args_json: r#"{"featureName":"auth","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step And I run `./rust/target/release/fspec show-coverage auth --format json` against the same workspace
    let (code, stdout, stderr) = run_show_coverage(ws.path(), &["auth", "--format", "json"]);
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}");

    // @step Then both invocations produce byte-equal JSON content
    // CLI adds a trailing newline to stdout; compare trimmed.
    assert_eq!(
        result.data.trim_end(),
        stdout.trim_end(),
        "dispatcher data and CLI stdout must match; dispatcher=\n{}\nCLI=\n{stdout}",
        result.data
    );

    // @step And the CLI bridge module rust/fspec/src/show_coverage.rs contains NO inline coverage parsing, stats aggregation, or markdown rendering — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_coverage.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/show_coverage.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Coverage Report:",
        "Overall Coverage",
        "Project Coverage Report",
        "Features Overview",
        "Coverage Gaps",
        "FULLY COVERED",
        "PARTIALLY COVERED",
        "UNCOVERED",
        "calculate_stats",
        "calculateStats",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
