//! CLI surface for the `show-foundation` subcommand on the standalone
//! fspec Rust binary — RPC-305.
//!
//! Feature: spec/features/show-foundation-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand is wired (Phase C), these tests
//! exercise the binary and expect either a NotYetPorted error path or
//! a missing-subcommand failure. Once the subcommand is wired, the
//! green-phase assertions take over.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_sf(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-foundation");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-foundation");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_foundation(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json"), raw).expect("write foundation.json");
}

fn dispatch_sf(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    let req = codelet_fspec_core::DispatchRequest {
        command: "show-foundation".to_string(),
        args_json: args_json.to_string(),
        project_root: project_root.to_path_buf(),
    };
    codelet_fspec_core::dispatch_command(req)
}

/// Minimal v2.0.0 foundation with project.name set.
fn minimal_foundation(name: &str) -> String {
    format!(
        r#"{{
  "schemaVersion": "2.0.0",
  "project": {{"name":"{name}","vision":"V","projectType":"cli-tool"}},
  "problemSpace": {{"primaryProblem":{{"title":"P","description":"D","impact":"I"}}}},
  "solutionSpace": {{"overview":"O","capabilities":[{{"name":"C","description":"D"}}]}},
  "personas": [{{"name":"User","description":"d"}}]
}}"#
    )
}

// =========================================================================
// Scenarios from show-foundation-cli-subcommand.feature
// =========================================================================

#[test]
fn scenario_clap_exposes_show_foundation_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // @step When I run `./rust/target/release/fspec show-foundation --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-foundation")
        .arg("--help")
        .output()
        .expect("spawn fspec show-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the substring 'show-foundation'
    assert!(
        stdout.contains("show-foundation") || stdout.contains("SHOW-FOUNDATION"),
        "help must describe the subcommand; got:\n{stdout}"
    );

    // @step Then stdout advertises the optional positional <section> argument
    assert!(
        stdout.contains("section") || stdout.contains("SECTION") || stdout.contains("[section]"),
        "help must advertise <section>; got:\n{stdout}"
    );

    // @step Then stdout does NOT advertise the '--format' flag
    assert!(
        !stdout.contains("--format"),
        "help must not advertise --format (TS parity); got:\n{stdout}"
    );

    // @step Then stdout does NOT advertise the '--output' flag
    assert!(
        !stdout.contains("--output"),
        "help must not advertise --output (TS parity); got:\n{stdout}"
    );

    // @step Then stdout advertises the '--draft' flag
    assert!(
        stdout.contains("--draft"),
        "help must advertise --draft; got:\n{stdout}"
    );

    // @step Then stdout advertises the '--list-sections' flag
    assert!(
        stdout.contains("--list-sections"),
        "help must advertise --list-sections; got:\n{stdout}"
    );

    // @step Then stdout advertises the '--line-numbers' flag
    assert!(
        stdout.contains("--line-numbers"),
        "help must advertise --line-numbers; got:\n{stdout}"
    );

    // Note: --format and --output exist on the command but are
    // intentionally NOT advertised in --help to mirror the TS reference.
}

#[test]
fn scenario_cli_default_render_prints_project_section() {
    // @step Given spec/foundation.json contains project.name='fspec', project.vision='V', project.projectType='cli-tool'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the exact line '=== PROJECT ==='
    assert!(
        stdout.lines().any(|l| l == "=== PROJECT ==="),
        "stdout must contain '=== PROJECT ==='; got:\n{stdout}"
    );

    // @step Then stdout contains the line 'Name: fspec'
    assert!(
        stdout.lines().any(|l| l == "Name: fspec"),
        "stdout must contain 'Name: fspec'; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_positional_section_emits_raw_string_in_text_format() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation projectName` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["projectName"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout equals exactly 'fspec' (with a trailing newline)
    assert_eq!(
        stdout, "fspec\n",
        "stdout must equal 'fspec\\n'; got: {stdout:?}"
    );
}

#[test]
fn scenario_cli_format_json_emits_json() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation projectName --format json` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["projectName", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout starts with the bytes '"fspec"'
    assert!(
        stdout.starts_with("\"fspec\""),
        "stdout must start with '\"fspec\"'; got: {stdout:?}"
    );
}

#[test]
fn scenario_cli_exits_1_when_section_unknown() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation nonexistent` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["nonexistent"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring "Field 'nonexistent' not found"
    assert!(
        stderr.contains("Field 'nonexistent' not found"),
        "stderr must contain field-not-found message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_draft_surfaces_missing_draft_error() {
    // @step Given spec/foundation.json.draft does NOT exist in the working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/foundation.json.draft").exists());

    // @step When I run `./rust/target/release/fspec show-foundation --draft` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["--draft"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step Then stderr contains the substring 'No draft found at spec/foundation.json.draft'
    assert!(
        stderr.contains("No draft found at spec/foundation.json.draft"),
        "stderr must contain missing-draft message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_output_writes_file_and_prints_success_line() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation projectName --output out/name.txt` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["projectName", "--output", "out/name.txt"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then the file out/name.txt exists with the exact bytes 'fspec'
    let written =
        fs::read_to_string(ws.path().join("out/name.txt")).expect("out/name.txt must exist");
    assert_eq!(
        written, "fspec",
        "out/name.txt must contain exactly 'fspec'; got: {written:?}"
    );

    // @step Then stdout contains the substring '✓'
    assert!(
        stdout.contains("✓"),
        "stdout must contain '✓'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Output written to out/name.txt'
    assert!(
        stdout.contains("Output written to out/name.txt"),
        "stdout must contain 'Output written to out/name.txt'; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_exits_1_when_foundation_malformed() {
    // @step Given spec/foundation.json exists in the working directory but contains invalid JSON
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), "{ not json");

    // @step When I run `./rust/target/release/fspec show-foundation` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_list_sections_is_parsed_but_ignored() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation --list-sections` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["--list-sections"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the exact line '=== PROJECT ==='
    assert!(
        stdout.lines().any(|l| l == "=== PROJECT ==="),
        "stdout must contain '=== PROJECT ==='; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_line_numbers_is_parsed_but_ignored() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./rust/target/release/fspec show-foundation --line-numbers` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["--line-numbers"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the exact line '=== PROJECT ==='
    assert!(
        stdout.lines().any(|l| l == "=== PROJECT ==="),
        "stdout must contain '=== PROJECT ==='; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation through fspec_core::dispatch::dispatch_command with section='projectName' and format='json'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"json"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly '"fspec"'
    assert_eq!(
        result.data, "\"fspec\"",
        "data must equal '\"fspec\"'; got: {:?}",
        result.data
    );

    // @step Then the CLI bridge module rust/fspec/src/show_foundation.rs contains NO inline FIELD_MAP, formatter, or filesystem logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_foundation.rs");
    assert!(
        bridge_path.exists(),
        "bridge module must exist: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "FIELD_MAP",
        "projectName",
        "problemSpace",
        "primaryProblem",
        "=== PROJECT ===",
        "=== PERSONAS ===",
        "formatFoundationAsText",
        "format_foundation_as_text",
        "getNestedProperty",
        "get_nested_property",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_SF: &str = include_str!("fixtures/help/show-foundation.txt");

#[test]
fn scenario_show_foundation_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // @step When I run `./rust/target/release/fspec show-foundation --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-foundation")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-foundation --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/show-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SF);

    // @step Then stdout starts with a blank line followed by 'SHOW-FOUNDATION'
    assert!(stdout.starts_with("\nSHOW-FOUNDATION\n"));
}

// =========================================================================
// BUG-537: --section is accepted alongside the positional [section].
// =========================================================================

#[test]
fn scenario_cli_section_flag_shows_the_same_section_as_the_positional() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `fspec show-foundation --section projectName` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["--section", "projectName"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout equals exactly 'fspec' (with a trailing newline), the same as `fspec show-foundation projectName`
    assert_eq!(stdout, "fspec\n");
    let (positional_code, positional_stdout, _) = run_sf(ws.path(), &["projectName"]);
    assert_eq!((code, stdout), (positional_code, positional_stdout));
}

#[test]
fn scenario_cli_section_flag_with_an_unknown_field_exits_1() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `fspec show-foundation --section nonexistent` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["--section", "nonexistent"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring "Field 'nonexistent' not found"
    assert!(
        stderr.contains("Field 'nonexistent' not found"),
        "stderr must contain field-not-found message; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_rejects_a_positional_section_together_with_section_flag() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `fspec show-foundation projectName --section projectVision` from that directory
    let (code, stdout, stderr) = run_sf(ws.path(), &["projectName", "--section", "projectVision"]);

    // @step Then the command exits non-zero without printing a section
    assert_ne!(code, 0, "stdout={stdout}, stderr={stderr}");
    assert!(stdout.is_empty(), "{stdout}");

    // @step And stderr names the --section argument
    assert!(stderr.contains("--section"), "{stderr}");
}

#[test]
fn scenario_show_foundation_help_lists_the_section_option() {
    // @step When I run `fspec show-foundation --help`
    let output = Command::new(fspec_bin())
        .args(["show-foundation", "--help"])
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-foundation --help");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the OPTIONS block lists '--section <section>' described as the same section as the positional [section]
    let options = &stdout[stdout.find("\nOPTIONS\n").expect("OPTIONS block")..];
    assert!(
        options.contains(
            "  --section <section>\n    Specific section to show (same as the positional [section])\n"
        ),
        "{stdout}"
    );
}
