//! CLI surface for the `show-foundation` subcommand on the standalone
//! fspec Rust binary — RPC-305.
//!
//! Feature: spec/features/show-foundation-cli-subcommand.feature
//! Feature: spec/features/show-foundation-rust-port.feature
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

fn write_foundation_draft(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("foundation.json.draft"), raw).expect("write foundation.json.draft");
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
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // @step When I run `./codelet/target/release/fspec show-foundation --help` from a shell
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

    // Note: --section, --format, --output, --draft flags exist on the
    // command but are intentionally NOT advertised in --help to mirror
    // the TS reference (`src/commands/show-foundation-help.ts` only
    // lists --list-sections and --line-numbers).
}

#[test]
fn scenario_cli_default_render_prints_project_section() {
    // @step Given spec/foundation.json contains project.name='fspec', project.vision='V', project.projectType='cli-tool'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I run `./codelet/target/release/fspec show-foundation` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation projectName` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation projectName --format json` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation nonexistent` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation --draft` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation projectName --output out/name.txt` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation --list-sections` from that directory
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

    // @step When I run `./codelet/target/release/fspec show-foundation --line-numbers` from that directory
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

    // @step Then the CLI bridge module codelet/fspec/src/show_foundation.rs contains NO inline FIELD_MAP, formatter, or filesystem logic — its only computation is JSON arg marshalling and stdout printing
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
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // @step When I run `./codelet/target/release/fspec show-foundation --help` piped to non-TTY
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

    // @step Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-foundation.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SF);

    // @step Then stdout starts with a blank line followed by 'SHOW-FOUNDATION'
    assert!(stdout.starts_with("\nSHOW-FOUNDATION\n"));
}

// =========================================================================
// Scenarios from show-foundation-rust-port.feature (dispatcher path)
// =========================================================================

#[test]
fn scenario_returns_text_render_with_project_and_other_sections() {
    // @step Given spec/foundation.json contains project.name='fspec', project.vision='V', project.projectType='cli-tool', a primary problem with title/description/impact, a solution overview with one capability, and one persona
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with no section and format='text'
    let result = dispatch_sf(ws.path(), r#"{"format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data contains the exact line '=== PROJECT ==='
    assert!(
        result.data.lines().any(|l| l == "=== PROJECT ==="),
        "data must contain '=== PROJECT ==='; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Name: fspec'
    assert!(
        result.data.lines().any(|l| l == "Name: fspec"),
        "data must contain 'Name: fspec'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Vision: V'
    assert!(
        result.data.lines().any(|l| l == "Vision: V"),
        "data must contain 'Vision: V'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the line 'Type: cli-tool'
    assert!(
        result.data.lines().any(|l| l == "Type: cli-tool"),
        "data must contain 'Type: cli-tool'; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '=== PROBLEM SPACE ==='
    assert!(
        result.data.lines().any(|l| l == "=== PROBLEM SPACE ==="),
        "data must contain '=== PROBLEM SPACE ==='; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '=== SOLUTION SPACE ==='
    assert!(
        result.data.lines().any(|l| l == "=== SOLUTION SPACE ==="),
        "data must contain '=== SOLUTION SPACE ==='; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data contains the exact line '=== PERSONAS ==='
    assert!(
        result.data.lines().any(|l| l == "=== PERSONAS ==="),
        "data must contain '=== PERSONAS ==='; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_returns_entire_foundation_as_pretty_json_when_format_json() {
    // @step Given spec/foundation.json contains a complete v2.0.0 foundation
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with no section and format='json'
    let result = dispatch_sf(ws.path(), r#"{"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root has a 'project' field
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert!(
        parsed.get("project").is_some(),
        "data root must have 'project' field; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.contains("\n  "),
        "data must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_resolves_project_name_via_field_map_text_format() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'fspec'
    assert_eq!(
        result.data, "fspec",
        "data must equal 'fspec'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_resolves_project_name_via_field_map_json_format() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and format='json'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly '"fspec"'
    assert_eq!(
        result.data, "\"fspec\"",
        "data must equal '\"fspec\"'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_section_pointing_to_object_emits_pretty_json_in_text_format() {
    // @step Given spec/foundation.json contains project.name='fspec' and project.projectType='cli-tool'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='project' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"project","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data parses as JSON whose root has 'name' and 'projectType'
    let parsed: serde_json::Value = serde_json::from_str(&result.data).expect("data is JSON");
    assert!(
        parsed.get("name").is_some(),
        "data must have 'name' field; got:\n{}",
        result.data
    );
    assert!(
        parsed.get("projectType").is_some(),
        "data must have 'projectType' field; got:\n{}",
        result.data
    );

    // @step Then the DispatchResult.data uses 2-space indentation
    assert!(
        result.data.contains("\n  "),
        "data must use 2-space indentation; got:\n{}",
        result.data
    );
}

#[test]
fn scenario_missing_section_returns_field_not_found_error() {
    // @step Given spec/foundation.json contains a complete foundation
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='nonexistent' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"nonexistent","format":"text"}"#);

    // @step Then the dispatcher returns success=false with an error message exactly "Field 'nonexistent' not found"
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("Field 'nonexistent' not found"),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_dotted_path_bypasses_field_map_for_unmapped_sections() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='project.name' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"project.name","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'fspec'
    assert_eq!(
        result.data, "fspec",
        "data must equal 'fspec'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_draft_true_with_no_draft_file_returns_missing_draft_error() {
    // @step Given spec/foundation.json.draft does NOT exist in the project root
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec/foundation.json.draft").exists());

    // @step When I dispatch show-foundation with draft=true
    let result = dispatch_sf(ws.path(), r#"{"draft":true}"#);

    // @step Then the dispatcher returns success=false with an error message exactly 'No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one.'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result.error.as_deref().unwrap_or("").contains("No draft found at spec/foundation.json.draft. Run `fspec discover-foundation` to create one."),
        "error must contain canonical message; got {result:?}"
    );
}

#[test]
fn scenario_draft_true_reads_draft_file_instead_of_foundation() {
    // @step Given spec/foundation.json contains project.name='final-name'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("final-name"));

    // @step Given spec/foundation.json.draft contains project.name='draft-name'
    write_foundation_draft(ws.path(), &minimal_foundation("draft-name"));

    // @step When I dispatch show-foundation with section='projectName' and draft=true and format='text'
    let result = dispatch_sf(
        ws.path(),
        r#"{"section":"projectName","draft":true,"format":"text"}"#,
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'draft-name'
    assert_eq!(
        result.data, "draft-name",
        "data must equal 'draft-name'; got: {:?}",
        result.data
    );
}

#[test]
fn scenario_empty_workspace_auto_creates_foundation_json() {
    // @step Given an empty project root directory with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch show-foundation with section='projectName' and format='text'
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName","format":"text"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'Project Name'
    assert_eq!(
        result.data, "Project Name",
        "data must equal 'Project Name' (canonical default); got: {:?}",
        result.data
    );

    // @step Then spec/foundation.json exists after the call (auto-created by ensure_foundation_file)
    assert!(
        ws.path().join("spec/foundation.json").exists(),
        "spec/foundation.json must be auto-created"
    );
}

#[test]
fn scenario_escalates_malformed_foundation_json_dispatcher() {
    // @step Given spec/foundation.json exists but contains the malformed bytes '{ not json'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), "{ not json");

    // @step When I dispatch show-foundation against that project root
    let result = dispatch_sf(ws.path(), r#"{}"#);

    // @step Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse foundation.json'
    assert!(!result.success, "must fail; got {result:?}");
    assert!(
        result
            .error
            .as_ref()
            .map(|e| e.contains("Failed to parse foundation.json"))
            .unwrap_or(false),
        "error must mention parse failure; got {result:?}"
    );
}

#[test]
fn scenario_output_writes_formatted_content_to_disk_via_dispatcher() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and format='text' and output='out/name.txt'
    let result = dispatch_sf(
        ws.path(),
        r#"{"section":"projectName","format":"text","output":"out/name.txt"}"#,
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the file <project_root>/out/name.txt exists with the exact bytes 'fspec'
    let written =
        fs::read_to_string(ws.path().join("out/name.txt")).expect("out/name.txt must exist");
    assert_eq!(
        written, "fspec",
        "file must contain exactly 'fspec'; got: {written:?}"
    );
}

#[test]
fn scenario_default_format_is_text_when_format_flag_omitted() {
    // @step Given spec/foundation.json contains project.name='fspec'
    let ws = tempfile::tempdir().expect("tempdir");
    write_foundation(ws.path(), &minimal_foundation("fspec"));

    // @step When I dispatch show-foundation with section='projectName' and no format flag
    let result = dispatch_sf(ws.path(), r#"{"section":"projectName"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "must succeed; got {result:?}");

    // @step Then the DispatchResult.data equals exactly 'fspec'
    assert_eq!(
        result.data, "fspec",
        "data must equal 'fspec' (default format text); got: {:?}",
        result.data
    );
}
