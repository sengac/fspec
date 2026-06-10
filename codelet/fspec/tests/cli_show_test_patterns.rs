//! CLI surface for the `show-test-patterns` subcommand on the standalone
//! fspec Rust binary — RPC-307.
//!
//! Features:
//!   - spec/features/show-test-patterns-rust-port.feature
//!   - spec/features/show-test-patterns-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired and the fspec_core port
//! is implemented (Phase C), these tests exercise the binary/dispatcher
//! and expect NotYetPorted / missing-subcommand failures. Once Phase C
//! lands the green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── Helpers ─────────

fn run_stp(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-test-patterns");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-test-patterns");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "show-test-patterns".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

fn write_file(cwd: &Path, rel: &str, body: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parents");
    }
    fs::write(path, body).expect("write file");
}

fn write_work_units(cwd: &Path, raw: &str) {
    write_file(cwd, "spec/work-units.json", raw);
}

// ─── Sample work-unit bodies ───

fn wus_two_cli_one_untagged() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "WU-001": {"id":"WU-001","title":"a","type":"story","status":"backlog","tags":["@cli"],"createdAt":"x","updatedAt":"x"},
    "WU-002": {"id":"WU-002","title":"b","type":"story","status":"backlog","tags":["@cli"],"createdAt":"x","updatedAt":"x"},
    "WU-003": {"id":"WU-003","title":"c","type":"story","status":"backlog","tags":[],"createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["WU-001","WU-002","WU-003"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#
}

fn wus_two_untagged() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "WU-001": {"id":"WU-001","title":"a","type":"story","status":"backlog","tags":["@auth"],"createdAt":"x","updatedAt":"x"},
    "WU-002": {"id":"WU-002","title":"b","type":"story","status":"backlog","tags":["@db"],"createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["WU-001","WU-002"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#
}

fn wus_one_cli() -> &'static str {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "WU-001": {"id":"WU-001","title":"a","type":"story","status":"backlog","tags":["@cli"],"createdAt":"x","updatedAt":"x"}
  },
  "states": {"backlog":["WU-001"],"specifying":[],"testing":[],"implementing":[],"validating":[],"done":[],"blocked":[]}
}"#
}

fn cov_two_files_three_unique_tests(ws: &Path) {
    // Two coverage files; together reference three unique testMappings.filePath strings.
    write_file(ws, "spec/features/login.feature", "Feature: Login\n");
    write_file(ws, "spec/features/auth.feature", "Feature: Auth\n");
    let cov_login = r#"{
  "featureName": "Login",
  "filePath": "spec/features/login.feature",
  "scenarios": [
    {"name":"S1","testMappings":[{"filePath":"test/a.test.ts","testLines":"1-10"},{"filePath":"test/b.test.ts","testLines":"1-10"}],"implMappings":[]}
  ],
  "stats": {"totalScenarios":1,"coveredScenarios":0,"coveragePercentage":0}
}"#;
    let cov_auth = r#"{
  "featureName": "Auth",
  "filePath": "spec/features/auth.feature",
  "scenarios": [
    {"name":"S2","testMappings":[{"filePath":"test/b.test.ts","testLines":"5-10"},{"filePath":"test/c.test.ts","testLines":"1-5"}],"implMappings":[]}
  ],
  "stats": {"totalScenarios":1,"coveredScenarios":0,"coveragePercentage":0}
}"#;
    write_file(ws, "spec/features/login.feature.coverage", cov_login);
    write_file(ws, "spec/features/auth.feature.coverage", cov_auth);
}

fn cov_one_file_with_paths(ws: &Path) {
    write_file(ws, "spec/features/login.feature", "Feature: Login\n");
    let cov = r#"{
  "featureName": "Login",
  "filePath": "spec/features/login.feature",
  "scenarios": [
    {"name":"S1","testMappings":[{"filePath":"test/x.test.ts","testLines":"1-10"}],"implMappings":[]}
  ],
  "stats": {"totalScenarios":1,"coveredScenarios":0,"coveragePercentage":0}
}"#;
    write_file(ws, "spec/features/login.feature.coverage", cov);
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 1: Missing tag argument surfaces error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_missing_tag_argument_surfaces_invalid_args_error() {
    // @step Given a temp project root contains a valid spec/work-units.json
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I dispatch show-test-patterns with no tag argument
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'tag'
    assert!(
        result.error.as_deref().unwrap_or("").to_lowercase().contains("tag"),
        "error must mention tag; got: {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 2: Tag matches zero work units returns empty
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_tag_matches_zero_work_units_returns_empty_data() {
    // @step Given a temp project root contains spec/work-units.json with two work units neither tagged @missing
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_two_untagged());

    // @step When I dispatch show-test-patterns with tag='@missing'
    let result = dispatch(ws.path(), r#"{"tag":"@missing"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.workUnits array is empty
    assert_eq!(parsed["workUnits"].as_array().unwrap().len(), 0);

    // @step And the data.patterns array is empty
    assert_eq!(parsed["patterns"].as_array().unwrap().len(), 0);

    // @step And the data.testFiles array is empty
    assert_eq!(parsed["testFiles"].as_array().unwrap().len(), 0);

    // @step And the data.format equals 'table'
    assert_eq!(parsed["format"].as_str(), Some("table"));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 3: Tag matches work units returns their tags
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_tag_matches_work_units_returns_their_tags_arrays() {
    // @step Given a temp project root contains spec/work-units.json with two work units tagged @cli and one untagged
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_two_cli_one_untagged());

    // @step When I dispatch show-test-patterns with tag='@cli'
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.workUnits array has 2 elements
    let wus = parsed["workUnits"].as_array().expect("workUnits array");
    assert_eq!(wus.len(), 2);

    // @step And every workUnits[i].tags array contains '@cli'
    for wu in wus {
        let tags = wu["tags"].as_array().expect("tags array");
        assert!(tags.iter().any(|t| t.as_str() == Some("@cli")));
    }
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 4: includeCoverage true reads & dedupes
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_include_coverage_true_reads_and_dedupes_test_files() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli and two .feature.coverage files referencing three unique testMappings file paths
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    cov_two_files_three_unique_tests(ws.path());

    // @step When I dispatch show-test-patterns with tag='@cli' and includeCoverage=true
    let result = dispatch(ws.path(), r#"{"tag":"@cli","includeCoverage":true}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.testFiles array has 3 unique elements
    let tf = parsed["testFiles"].as_array().expect("testFiles array");
    assert_eq!(tf.len(), 3, "expected 3 unique test files; got {tf:?}");
    let mut seen: Vec<&str> = tf.iter().filter_map(|v| v.as_str()).collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 3, "values must be unique; got {seen:?}");
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 5: includeCoverage false leaves testFiles empty
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_include_coverage_false_leaves_test_files_empty() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli and one .feature.coverage file referencing testMappings paths
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    cov_one_file_with_paths(ws.path());

    // @step When I dispatch show-test-patterns with tag='@cli' and no includeCoverage flag
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.testFiles array is empty
    assert_eq!(parsed["testFiles"].as_array().unwrap().len(), 0);
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 6: json format sets data.format to 'json'
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_json_format_sets_data_format_to_json() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I dispatch show-test-patterns with tag='@cli' and json=true
    let result = dispatch(ws.path(), r#"{"tag":"@cli","json":true}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.format equals 'json'
    assert_eq!(parsed["format"].as_str(), Some("json"));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 7: Default format flag yields 'table'
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_default_format_flag_yields_table_format() {
    // @step Given a temp project root contains spec/work-units.json with one work unit tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I dispatch show-test-patterns with tag='@cli' and no json flag
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.format equals 'table'
    assert_eq!(parsed["format"].as_str(), Some("table"));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 8: Shared infrastructure module is registered
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_shared_infrastructure_module_is_registered() {
    // @step Given the codelet/fspec-core crate is built

    // @step When I inspect codelet/fspec-core/src/commands/show_test_patterns.rs
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-307"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes show-test-patterns to the new run function
    assert!(
        result.success,
        "dispatcher must succeed when args are valid; got {result:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 1: Clap exposes subcommand with help
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_clap_exposes_subcommand_with_flag_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec show-test-patterns --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-test-patterns")
        .arg("--help")
        .output()
        .expect("spawn fspec show-test-patterns --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'show-test-patterns'
    assert!(
        stdout.contains("show-test-patterns") || stdout.contains("SHOW-TEST-PATTERNS"),
        "help must mention subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--tag'
    assert!(stdout.contains("--tag"), "help must mention --tag; got:\n{stdout}");
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 2: CLI without --tag flag exits 1
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_without_tag_flag_exits_non_zero() {
    // @step Given an empty directory with no spec/ subdirectory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec show-test-patterns` from that directory
    let (code, _stdout, _stderr) = run_stp(ws.path(), &[]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero when --tag is missing");
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 3: CLI default output prints green summary
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_output_prints_green_summary_line() {
    // @step Given a temp workspace contains spec/work-units.json with two work units tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_two_cli_one_untagged());

    // @step When I run `./codelet/target/release/fspec show-test-patterns --tag @cli` from that workspace
    let (code, stdout, stderr) = run_stp(ws.path(), &["--tag", "@cli"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'Analyzed testing patterns for 2 work units tagged with @cli'
    assert!(
        stdout.contains("Analyzed testing patterns for 2 work units tagged with @cli"),
        "stdout must contain green summary line; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 4: CLI --json prints JSON envelope
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_json_flag_prints_json_envelope_to_stdout() {
    // @step Given a temp workspace contains spec/work-units.json with one work unit tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());

    // @step When I run `./codelet/target/release/fspec show-test-patterns --tag @cli --json` from that workspace
    let (code, stdout, stderr) = run_stp(ws.path(), &["--tag", "@cli", "--json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout parses as JSON with workUnits, testFiles, patterns, and format fields
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    for field in ["workUnits", "testFiles", "patterns", "format"] {
        assert!(parsed.get(field).is_some(), "missing field `{field}` in:\n{stdout}");
    }

    // @step And the JSON.format field equals 'json'
    assert_eq!(parsed["format"].as_str(), Some("json"));
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 5: --include-coverage dedupes test files
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_include_coverage_includes_deduplicated_test_file_paths() {
    // @step Given a temp workspace contains spec/work-units.json with one work unit tagged @cli and two .feature.coverage files referencing three unique testMappings file paths
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_one_cli());
    cov_two_files_three_unique_tests(ws.path());

    // @step When I run `./codelet/target/release/fspec show-test-patterns --tag @cli --include-coverage --json` from that workspace
    let (code, stdout, stderr) = run_stp(
        ws.path(),
        &["--tag", "@cli", "--include-coverage", "--json"],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And the JSON.testFiles array has 3 unique elements
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let tf = parsed["testFiles"].as_array().expect("testFiles array");
    assert_eq!(tf.len(), 3, "expected 3 unique test files; got {tf:?}");
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 6: CLI exits 1 when work-units.json missing
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_exits_1_when_work_units_json_missing() {
    // @step Given an empty directory with no spec/ subdirectory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec show-test-patterns --tag @cli` from that directory
    let (code, _stdout, stderr) = run_stp(ws.path(), &["--tag", "@cli"]);

    // @step Then the command exits with a non-zero status
    assert_ne!(code, 0, "must exit non-zero when work-units.json missing");

    // @step And stderr contains the substring 'Error:'
    assert!(stderr.contains("Error:"), "stderr must contain 'Error:'; got:\n{stderr}");
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 7: --help byte-for-byte identical
// ═════════════════════════════════════════════════════════════════════════

const TS_HELP_FIXTURE_STP: &str = include_str!("fixtures/help/show-test-patterns.txt");

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec show-test-patterns --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-test-patterns")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-test-patterns --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-test-patterns.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_STP);

    // @step And stdout starts with a blank line followed by 'SHOW-TEST-PATTERNS'
    assert!(stdout.starts_with("\nSHOW-TEST-PATTERNS\n"));
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 8: Default combined TUI mode preserved
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_combined_tui_mode_preserved() {
    // @step Given the fspec Rust binary has show-test-patterns registered as a clap subcommand alongside daemon, client, status, and other ported subcommands

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists show-test-patterns as an available subcommand
    assert!(
        help.contains("show-test-patterns"),
        "fspec --help must list show-test-patterns; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 9: CLI delegates to fspec_core function
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a temp workspace contains spec/work-units.json with two work units tagged @cli
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), wus_two_cli_one_untagged());

    // @step When I dispatch show-test-patterns through fspec_core::dispatch::dispatch_command with tag='@cli' against that workspace
    let result = dispatch(ws.path(), r#"{"tag":"@cli"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let wus = parsed["workUnits"].as_array().expect("workUnits array");

    // @step And I run `./codelet/target/release/fspec show-test-patterns --tag @cli --json` against the same workspace
    let (code, stdout, _stderr) = run_stp(ws.path(), &["--tag", "@cli", "--json"]);
    assert_eq!(code, 0, "CLI must exit 0");
    let cli_parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout is JSON");
    let cli_wus = cli_parsed["workUnits"].as_array().expect("workUnits array");

    // @step Then both invocations produce a JSON envelope with workUnits.length=2
    assert_eq!(wus.len(), 2, "dispatcher must report 2 work units");
    assert_eq!(cli_wus.len(), 2, "CLI must report 2 work units");

    // @step And the CLI bridge module codelet/fspec/src/show_test_patterns.rs contains NO inline filtering or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_test_patterns.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/show_test_patterns.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Analyzed testing patterns",
        "testFiles",
        "extract_test_files",
        "read_all_coverage_files",
        "workUnits",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
