//! CLI surface for the `show-acceptance-criteria` subcommand on the
//! standalone fspec Rust binary — RPC-299.
//!
//! Features:
//!   - spec/features/show-acceptance-criteria-rust-port.feature
//!   - spec/features/show-acceptance-criteria-cli-subcommand.feature
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

fn run_sac(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("show-acceptance-criteria");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec show-acceptance-criteria");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn dispatch(project_root: &Path, args_json: &str) -> codelet_fspec_core::DispatchResult {
    codelet_fspec_core::dispatch_command(codelet_fspec_core::DispatchRequest {
        command: "show-acceptance-criteria".to_string(),
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

fn mkdir(cwd: &Path, rel: &str) {
    fs::create_dir_all(cwd.join(rel)).expect("mkdir");
}

// ─── Sample feature bodies ───

const LOGIN_AUTH_FEATURE: &str = "@auth
Feature: Login

  Background:
    Given a user account exists

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should see the dashboard
";

const LOGIN_AUTH_SIMPLE: &str = "@auth
Feature: Login

  Scenario: Login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should see the dashboard
";

const MISC_FEATURE: &str = "@misc
Feature: Miscellaneous

  Scenario: Foo
    Given x
    Then y
";

const CRITICAL_AUTH_FEATURE: &str = "@critical @auth
Feature: CriticalAuth

  Scenario: A
    Given x
    Then y
";

const CRITICAL_ONLY_FEATURE: &str = "@critical
Feature: CritOnly

  Scenario: B
    Given x
    Then y
";

const NOBACK_FEATURE: &str = "@test
Feature: NoBack

  Scenario: A
    Given x
    Then y
";

const EMPTY_SCENARIOS_FEATURE: &str = "@empty
Feature: Empty
";

fn write_critical_three_features_15_scenarios(ws: &Path) {
    // Each scenario has 1 Given step. We add 5+5+5 = 15 total scenarios.
    let mut a = String::from("@critical\nFeature: A\n\n");
    for i in 0..5 {
        a.push_str(&format!("  Scenario: A{i}\n    Given step\n\n"));
    }
    let mut b = String::from("@critical\nFeature: B\n\n");
    for i in 0..5 {
        b.push_str(&format!("  Scenario: B{i}\n    Given step\n\n"));
    }
    let mut c = String::from("@critical\nFeature: C\n\n");
    for i in 0..5 {
        c.push_str(&format!("  Scenario: C{i}\n    Given step\n\n"));
    }
    write_file(ws, "spec/features/a.feature", &a);
    write_file(ws, "spec/features/b.feature", &b);
    write_file(ws, "spec/features/c.feature", &c);
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 1: Missing spec/features returns error
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_missing_spec_features_directory_returns_structured_error() {
    // @step Given an empty temp project root with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I dispatch show-acceptance-criteria with no arguments
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false; got {result:?}");

    // @step And the error field contains the substring 'spec/features directory not found'
    assert!(
        result
            .error
            .as_deref()
            .unwrap_or("")
            .contains("spec/features directory not found"),
        "error must contain canonical message; got: {:?}",
        result.error
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 2: Empty spec/features returns success+message
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_empty_spec_features_directory_returns_success_with_message() {
    // @step Given a temp project root with an empty spec/features/ directory
    let ws = tempfile::tempdir().expect("tempdir");
    mkdir(ws.path(), "spec/features");

    // @step When I dispatch show-acceptance-criteria with no arguments
    let result = dispatch(ws.path(), "{}");

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.features array is empty
    assert_eq!(parsed["features"].as_array().unwrap().len(), 0);

    // @step And the data.message equals 'No feature files found in spec/features/'
    assert_eq!(
        parsed["message"].as_str(),
        Some("No feature files found in spec/features/")
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 3: Tag filter selects only matching features
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_tag_filter_selects_only_features_carrying_that_tag() {
    // @step Given a temp project root with two feature files - 'login.feature' tagged '@auth' and 'misc.feature' tagged '@misc'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);
    write_file(ws.path(), "spec/features/misc.feature", MISC_FEATURE);

    // @step When I dispatch show-acceptance-criteria with tags=['@auth']
    let result = dispatch(ws.path(), r#"{"tags":["@auth"]}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.features array has 1 element
    let features = parsed["features"].as_array().expect("features array");
    assert_eq!(features.len(), 1);

    // @step And the data.features[0].tags contains '@auth'
    let tags = features[0]["tags"].as_array().expect("tags array");
    assert!(tags.iter().any(|t| t.as_str() == Some("@auth")));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 4: Multiple tags require ALL tags present
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_multiple_tag_filters_require_all_tags_present() {
    // @step Given a temp project root with one feature tagged '@critical @auth' and one tagged '@critical' only
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "spec/features/both.feature",
        CRITICAL_AUTH_FEATURE,
    );
    write_file(
        ws.path(),
        "spec/features/only.feature",
        CRITICAL_ONLY_FEATURE,
    );

    // @step When I dispatch show-acceptance-criteria with tags=['@critical','@auth']
    let result = dispatch(ws.path(), r#"{"tags":["@critical","@auth"]}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.features array has 1 element
    let features = parsed["features"].as_array().expect("features array");
    assert_eq!(features.len(), 1);

    // @step And the data.features[0].tags contains both '@critical' and '@auth'
    let tags = features[0]["tags"].as_array().expect("tags array");
    assert!(tags.iter().any(|t| t.as_str() == Some("@critical")));
    assert!(tags.iter().any(|t| t.as_str() == Some("@auth")));
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 5: markdown render emits H1/H2/bullet steps
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_format_markdown_renders_h1_blockquote_h2_bullet_steps() {
    // @step Given a temp project root with one feature 'login.feature' tagged '@auth' containing a background and one scenario with steps
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_FEATURE);

    // @step When I dispatch show-acceptance-criteria with tags=['@auth'] and format='markdown'
    let result = dispatch(ws.path(), r#"{"tags":["@auth"],"format":"markdown"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let out = parsed["output"].as_str().expect("output string");

    // @step And the data.output contains the substring '# '
    assert!(out.contains("# "), "output must contain '# '; got:\n{out}");

    // @step And the data.output contains the substring '## '
    assert!(
        out.contains("## "),
        "output must contain '## '; got:\n{out}"
    );

    // @step And the data.output contains the substring '- **'
    assert!(
        out.contains("- **"),
        "output must contain '- **'; got:\n{out}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 6: json renders 2-space JSON array
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_format_json_renders_two_space_json_array() {
    // @step Given a temp project root with one feature 'login.feature' tagged '@auth' with one scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I dispatch show-acceptance-criteria with tags=['@auth'] and format='json'
    let result = dispatch(ws.path(), r#"{"tags":["@auth"],"format":"json"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let out = parsed["output"].as_str().expect("output string");

    // @step And the data.output parses as a JSON array
    let inner: serde_json::Value =
        serde_json::from_str(out).expect("data.output must parse as JSON");
    let arr = inner.as_array().expect("data.output is JSON array");

    // @step And the first element has name, tags, and scenarios properties
    assert!(arr[0].get("name").is_some());
    assert!(arr[0].get("tags").is_some());
    assert!(arr[0].get("scenarios").is_some());

    // @step And the data.output uses 2-space indentation
    assert!(
        out.contains("\n  ") || out.contains("[\n  {"),
        "output must use 2-space indentation; got:\n{out}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 7: Tag matches zero features → message
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_tag_matches_zero_features_returns_no_features_message() {
    // @step Given a temp project root with one feature 'misc.feature' tagged '@misc'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/misc.feature", MISC_FEATURE);

    // @step When I dispatch show-acceptance-criteria with tags=['@deprecated']
    let result = dispatch(ws.path(), r#"{"tags":["@deprecated"]}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.features array is empty
    assert_eq!(parsed["features"].as_array().unwrap().len(), 0);

    // @step And the data.message equals 'No features found matching tags: @deprecated'
    assert_eq!(
        parsed["message"].as_str(),
        Some("No features found matching tags: @deprecated")
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 8: Feature without Background rendered as null
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_feature_without_background_rendered_with_no_background_block() {
    // @step Given a temp project root with one feature 'noback.feature' tagged '@test' that has no Background section
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/noback.feature", NOBACK_FEATURE);

    // @step When I dispatch show-acceptance-criteria with tags=['@test']
    let result = dispatch(ws.path(), r#"{"tags":["@test"]}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.features[0].background is null
    let bg = &parsed["features"][0]["background"];
    assert!(
        bg.is_null() || bg.as_str() == Some(""),
        "features[0].background must be null/empty; got {bg:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 9: No scenarios renders 'No scenarios defined'
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_feature_with_no_scenarios_shows_no_scenarios_defined_marker() {
    // @step Given a temp project root with one feature 'empty.feature' tagged '@empty' having no scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(
        ws.path(),
        "spec/features/empty.feature",
        EMPTY_SCENARIOS_FEATURE,
    );

    // @step When I dispatch show-acceptance-criteria with tags=['@empty'] and format='markdown'
    let result = dispatch(ws.path(), r#"{"tags":["@empty"],"format":"markdown"}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let out = parsed["output"].as_str().expect("output string");

    // @step And the data.output contains the substring '_No scenarios defined_'
    assert!(
        out.contains("_No scenarios defined_"),
        "output must contain '_No scenarios defined_'; got:\n{out}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 10: Output path writes file + updates message
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_output_path_writes_rendered_content_to_disk_and_changes_message() {
    // @step Given a temp project root with one feature 'login.feature' tagged '@auth' with one scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I dispatch show-acceptance-criteria with tags=['@auth'], format='markdown', and output='out/acs.md'
    let result = dispatch(
        ws.path(),
        r#"{"tags":["@auth"],"format":"markdown","output":"out/acs.md"}"#,
    );

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the file <project_root>/out/acs.md exists with the same bytes as the formatted markdown
    let acs_path = ws.path().join("out").join("acs.md");
    assert!(acs_path.exists(), "out/acs.md must exist");
    // (Note: when output is set, data.output may equal the formatted content)
    let written = fs::read_to_string(&acs_path).expect("read out/acs.md");
    if let Some(formatted) = parsed["output"].as_str() {
        assert_eq!(written, formatted, "file bytes must equal formatted output");
    }

    // @step And the data.message equals 'Acceptance criteria written to acs.md'
    assert_eq!(
        parsed["message"].as_str(),
        Some("Acceptance criteria written to acs.md")
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 11: Summary line reports scenario+feature counts
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_summary_line_reports_scenario_and_feature_counts() {
    // @step Given a temp project root with three feature files all tagged '@critical' having 15 total scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    write_critical_three_features_15_scenarios(ws.path());

    // @step When I dispatch show-acceptance-criteria with tags=['@critical']
    let result = dispatch(ws.path(), r#"{"tags":["@critical"]}"#);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step And the data.totalScenarios equals 15
    assert_eq!(parsed["totalScenarios"].as_u64(), Some(15));

    // @step And the data.message contains the substring 'Showing acceptance criteria for 15 scenarios from 3 features'
    assert!(
        parsed["message"]
            .as_str()
            .unwrap_or("")
            .contains("Showing acceptance criteria for 15 scenarios from 3 features"),
        "message must contain summary line; got: {:?}",
        parsed["message"]
    );
}

// ═════════════════════════════════════════════════════════════════════════
// rust-port.feature — Scenario 12: Shared infrastructure module is registered
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn rust_port_shared_infrastructure_module_is_registered() {
    // @step Given the codelet/fspec-core crate is built

    // @step When I inspect codelet/fspec-core/src/commands/show_acceptance_criteria.rs
    let ws = tempfile::tempdir().expect("tempdir");
    mkdir(ws.path(), "spec/features");
    let result = dispatch(ws.path(), "{}");

    // @step Then the module no longer returns FspecCoreError::NotYetPorted
    let err = result.error.as_deref().unwrap_or("");
    assert!(
        !err.contains("NotYetPorted")
            && !err.contains("not yet ported")
            && !err.contains("RPC-299"),
        "module must no longer return NotYetPorted; got error: {err:?}"
    );

    // @step And the dispatcher routes show-acceptance-criteria to the new run function
    assert!(
        result.success,
        "dispatcher must succeed on valid spec/features dir; got {result:?}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 1: Clap exposes subcommand with help
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_clap_exposes_subcommand_with_flag_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("show-acceptance-criteria")
        .arg("--help")
        .output()
        .expect("spawn fspec show-acceptance-criteria --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'show-acceptance-criteria'
    assert!(
        stdout.contains("show-acceptance-criteria") || stdout.contains("SHOW-ACCEPTANCE-CRITERIA"),
        "help must mention subcommand; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 2: CLI no spec/features exits 1
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_against_workspace_with_no_spec_features_exits_1() {
    // @step Given an empty directory with no spec/ subdirectory is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria` from that directory
    let (code, stdout, stderr) = run_sac(ws.path(), &[]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "must exit 1; stdout={stdout}, stderr={stderr}");

    // @step And stderr contains the substring 'Error:'
    assert!(
        stderr.contains("Error:"),
        "stderr must contain 'Error:'; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'spec/features'
    assert!(
        stderr.contains("spec/features"),
        "stderr must mention spec/features; got:\n{stderr}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 3: CLI default text output
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_text_output_prints_feature_name_and_scenario_steps() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario 'Login with valid credentials' and three steps
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --tag @auth` from that workspace
    let (code, stdout, stderr) = run_sac(ws.path(), &["--tag", "@auth"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'login'
    assert!(
        stdout.to_lowercase().contains("login"),
        "stdout must mention 'login'; got:\n{stdout}"
    );

    // @step And stdout contains the substring 'Login with valid credentials'
    assert!(
        stdout.contains("Login with valid credentials"),
        "stdout must contain scenario name; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 4: --format=markdown H1/H2/bullet
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_format_markdown_prints_h1_h2_bullet_step_output() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario and three steps
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --tag @auth --format markdown` from that workspace
    let (code, stdout, stderr) = run_sac(ws.path(), &["--tag", "@auth", "--format", "markdown"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring '# '
    assert!(
        stdout.contains("# "),
        "stdout must contain '# '; got:\n{stdout}"
    );

    // @step And stdout contains the substring '## '
    assert!(
        stdout.contains("## "),
        "stdout must contain '## '; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 5: --format=json
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_format_json_prints_two_space_json_array() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --tag @auth --format json` from that workspace
    let (code, stdout, stderr) = run_sac(ws.path(), &["--tag", "@auth", "--format", "json"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains a JSON array as a substring
    assert!(
        stdout.contains("[") && stdout.contains("]"),
        "stdout must contain JSON array brackets; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 6: --output writes file + message
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_output_writes_file_and_prints_message_without_dumping_content() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --tag @auth --format markdown --output out.md` from that workspace
    let (code, stdout, stderr) = run_sac(
        ws.path(),
        &[
            "--tag", "@auth", "--format", "markdown", "--output", "out.md",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And the file out.md in the workspace contains the rendered markdown
    let out_path = ws.path().join("out.md");
    assert!(out_path.exists(), "out.md must exist in workspace");
    let body = fs::read_to_string(&out_path).expect("read out.md");
    assert!(
        body.contains("# ") || body.contains("## "),
        "out.md must contain rendered markdown; got:\n{body}"
    );

    // @step And stdout contains the substring 'Acceptance criteria written to out.md'
    assert!(
        stdout.contains("Acceptance criteria written to out.md"),
        "stdout must contain output message; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 7: --tag matching zero features
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_tag_matching_zero_features_prints_no_features_found() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth'
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --tag @missing` from that workspace
    let (code, stdout, stderr) = run_sac(ws.path(), &["--tag", "@missing"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; stderr={stderr}");

    // @step And stdout contains the substring 'No features found matching tags: @missing'
    assert!(
        stdout.contains("No features found matching tags: @missing"),
        "stdout must contain canonical no-match message; got:\n{stdout}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 8: --help byte-for-byte identical
// ═════════════════════════════════════════════════════════════════════════

const TS_HELP_FIXTURE_SAC: &str = include_str!("fixtures/help/show-acceptance-criteria.txt");

#[test]
fn cli_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec show-acceptance-criteria --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("show-acceptance-criteria")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn show-acceptance-criteria --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/show-acceptance-criteria.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_SAC);

    // @step And stdout starts with a blank line followed by 'SHOW-ACCEPTANCE-CRITERIA'
    assert!(stdout.starts_with("\nSHOW-ACCEPTANCE-CRITERIA\n"));
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 9: Default combined TUI mode preserved
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_default_combined_tui_mode_preserved() {
    // @step Given the fspec Rust binary has show-acceptance-criteria registered as a clap subcommand alongside daemon, client, status, and other ported subcommands

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(code, 0, "fspec --help must exit 0");
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists show-acceptance-criteria as an available subcommand
    assert!(
        help.contains("show-acceptance-criteria"),
        "fspec --help must list show-acceptance-criteria; got:\n{help}"
    );

    // @step And the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "fspec --help long-about must document combined-mode default; got:\n{help}"
    );
}

// ═════════════════════════════════════════════════════════════════════════
// cli-subcommand.feature — Scenario 10: CLI delegates to fspec_core function
// ═════════════════════════════════════════════════════════════════════════

#[test]
fn cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a temp workspace contains spec/features/login.feature tagged '@auth' with one scenario
    let ws = tempfile::tempdir().expect("tempdir");
    write_file(ws.path(), "spec/features/login.feature", LOGIN_AUTH_SIMPLE);

    // @step When I dispatch show-acceptance-criteria through fspec_core::dispatch::dispatch_command with tags=['@auth'] and format='json' against that workspace
    let result = dispatch(ws.path(), r#"{"tags":["@auth"],"format":"json"}"#);
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    let dispatcher_out = parsed["output"].as_str().expect("output string");
    let dispatcher_features: serde_json::Value =
        serde_json::from_str(dispatcher_out).expect("output is JSON");

    // @step And I run `./codelet/target/release/fspec show-acceptance-criteria --tag @auth --format json` against the same workspace
    let (code, stdout, _stderr) = run_sac(ws.path(), &["--tag", "@auth", "--format", "json"]);
    assert_eq!(code, 0, "CLI must exit 0");

    // @step Then both invocations produce equivalent JSON for the features array
    // (The CLI prints message line + the JSON body; extract the JSON array.)
    let cli_json_start = stdout
        .find('[')
        .expect("CLI stdout must contain JSON array");
    let cli_json_end = stdout
        .rfind(']')
        .expect("CLI stdout must contain JSON array end");
    let cli_json_slice = &stdout[cli_json_start..=cli_json_end];
    let cli_features: serde_json::Value =
        serde_json::from_str(cli_json_slice).expect("CLI JSON slice parses");
    assert_eq!(
        dispatcher_features, cli_features,
        "dispatcher and CLI must produce equivalent JSON arrays"
    );

    // @step And the CLI bridge module codelet/fspec/src/show_acceptance_criteria.rs contains NO inline gherkin parsing, filter, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/show_acceptance_criteria.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/show_acceptance_criteria.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Acceptance criteria written to",
        "No features found matching tags",
        "No scenarios defined",
        "spec/features directory not found",
        "Background:",
        "FeatureAC",
        "generate_markdown",
        "generate_text_output",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}
