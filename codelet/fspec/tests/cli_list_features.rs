//! CLI surface for the `list-features` subcommand on the standalone fspec
//! Rust binary — RPC-245.
//!
//! Feature: spec/features/list-features-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because:
//!   - `codelet/fspec/src/main.rs` does not yet register a `list-features`
//!     clap subcommand (clap returns exit code 2 for "unrecognized
//!     subcommand").
//!   - `codelet/fspec-core/src/commands/list_features.rs` is still a
//!     NotYetPorted stub.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_list_features(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-features");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn list-features");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_feature(cwd: &Path, rel: &str, content: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&path, content).expect("write feature file");
}

fn mk_features_dir(cwd: &Path) {
    let features = cwd.join("spec/features");
    fs::create_dir_all(&features).expect("mkdir spec/features");
}

fn feature_body(name: &str, tags: &[&str], scenarios: usize) -> String {
    let mut s = String::new();
    for t in tags {
        s.push_str(t);
        s.push('\n');
    }
    s.push_str(&format!("Feature: {name}\n"));
    for i in 0..scenarios {
        s.push_str(&format!(
            "\n  Scenario: scenario {i}\n    Given a precondition\n    When something happens\n    Then expect outcome\n",
        ));
    }
    s
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-features as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_features_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec list-features --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-features")
        .arg("--help")
        .output()
        .expect("spawn list-features --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-features --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains clap-generated help describing the list-features subcommand
    assert!(
        stdout.contains("list-features") || stdout.contains("List all feature"),
        "help must describe the list-features subcommand; got:\n{stdout}"
    );

    // @step Then stdout contains the substring '--tag'
    assert!(
        stdout.contains("--tag"),
        "list-features --help must advertise --tag; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--status'
    assert!(
        !stdout.contains("--status"),
        "list-features --help must NOT advertise --status; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--prefix'
    assert!(
        !stdout.contains("--prefix"),
        "list-features --help must NOT advertise --prefix; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--epic'
    assert!(
        !stdout.contains("--epic"),
        "list-features --help must NOT advertise --epic; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-features --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-features --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against directory with no spec/ exits 2 with Directory-not-found error
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_no_spec_dir_exits_2_with_directory_not_found() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec list-features` from that directory
    let (code, stdout, stderr) = run_list_features(ws.path(), &[]);

    // @step Then the command exits with code 2
    assert_eq!(
        code, 2,
        "list-features must exit 2 when spec/features/ is missing; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the exact line 'Directory not found: spec/features/'
    assert!(
        stderr
            .lines()
            .any(|l| l == "Directory not found: spec/features/"),
        "stderr must contain exact bare 'Directory not found: spec/features/' line (NO 'Error:' prefix per RPC-245 TS parity); got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI DirectoryNotFound error renders bare message plus indented
//           Suggestion line (RPC-245 TS-parity fix)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_directory_not_found_renders_bare_message_plus_suggestion() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec list-features` from that directory
    let (code, stdout, stderr) = run_list_features(ws.path(), &[]);

    // @step Then stderr contains the exact line 'Directory not found: spec/features/' (WITHOUT an 'Error:' prefix)
    assert!(
        stderr
            .lines()
            .any(|l| l == "Directory not found: spec/features/"),
        "stderr must contain BARE error message (no 'Error:' prefix); got stdout={stdout}\nstderr={stderr}"
    );
    assert!(
        !stderr.lines().any(|l| l.starts_with("Error: Directory not found")),
        "stderr must NOT prefix the directory-not-found message with 'Error:' (TS parity); got:\n{stderr}"
    );

    // @step Then stderr contains the exact line "  Suggestion: Run 'fspec create-feature' to create your first feature"
    assert!(
        stderr
            .lines()
            .any(|l| l == "  Suggestion: Run 'fspec create-feature' to create your first feature"),
        "stderr must contain the indented Suggestion continuation line; got:\n{stderr}"
    );

    // @step Then the command exits with code 2
    assert_eq!(
        code, 2,
        "list-features must exit 2 on DirectoryNotFound; got {code}, stdout={stdout}, stderr={stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints a Warning line to stderr when a feature file cannot
//           be parsed (RPC-245 TS-parity fix)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_warning_to_stderr_when_feature_file_cannot_be_parsed() {
    // @step Given spec/features/valid.feature contains a parseable feature with 2 scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    mk_features_dir(ws.path());
    write_feature(
        ws.path(),
        "spec/features/valid.feature",
        &feature_body("Valid", &[], 2),
    );

    // @step Given spec/features/broken.feature contains plain text with no Feature header
    write_feature(
        ws.path(),
        "spec/features/broken.feature",
        "not a feature file at all\nrandom bytes\n",
    );

    // @step When I run `./codelet/target/release/fspec list-features` from that directory
    let (code, stdout, stderr) = run_list_features(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-features must succeed when a single file fails to parse; got {code}, stderr={stderr}"
    );

    // @step Then stderr contains the exact line 'Warning: Could not parse spec/features/broken.feature'
    assert!(
        stderr
            .lines()
            .any(|l| l == "Warning: Could not parse spec/features/broken.feature"),
        "stderr must contain the per-file parse-warning line (TS parity); got stderr=\n{stderr}\nstdout=\n{stdout}"
    );

    // @step Then stdout contains the substring 'spec/features/valid.feature - Valid'
    assert!(
        stdout.contains("spec/features/valid.feature - Valid"),
        "stdout must still list the parseable feature; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line 'Found 1 feature files'
    assert!(
        stdout.lines().any(|l| l == "Found 1 feature files"),
        "stdout must show the summary count reflecting only the parseable feature; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI against empty spec/features/ prints sentinel and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_against_empty_features_dir_prints_sentinel_and_exits_0() {
    // @step Given a working directory containing an empty spec/features/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    mk_features_dir(ws.path());

    // @step When I run `./codelet/target/release/fspec list-features` from that directory
    let (code, stdout, stderr) = run_list_features(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-features must exit 0 on empty features dir; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'No feature files found in spec/features/'
    assert!(
        stdout.contains("No feature files found in spec/features/"),
        "stdout must contain sentinel; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI text output renders feature listing for the populated case
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_text_output_renders_feature_listing_populated_case() {
    // @step Given spec/features/auth.feature exists with name 'User Authentication', tags '@critical @auth' and 2 scenarios
    let ws = tempfile::tempdir().expect("tempdir");
    mk_features_dir(ws.path());
    write_feature(
        ws.path(),
        "spec/features/auth.feature",
        &feature_body("User Authentication", &["@critical", "@auth"], 2),
    );

    // @step Given spec/features/billing.feature exists with name 'Billing', tags '@billing' and 1 scenario
    write_feature(
        ws.path(),
        "spec/features/billing.feature",
        &feature_body("Billing", &["@billing"], 1),
    );

    // @step When I run `./codelet/target/release/fspec list-features`
    let (code, stdout, stderr) = run_list_features(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-features must exit 0 on populated case; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the exact line '  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]'
    assert!(
        stdout.lines().any(|l| l
            == "  spec/features/auth.feature - User Authentication (2 scenarios) [@critical @auth]"),
        "stdout must contain exact auth listing line; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line '  spec/features/billing.feature - Billing (1 scenarios) [@billing]'
    assert!(
        stdout
            .lines()
            .any(|l| l == "  spec/features/billing.feature - Billing (1 scenarios) [@billing]"),
        "stdout must contain exact billing listing line; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line 'Found 2 feature files'
    assert!(
        stdout.lines().any(|l| l == "Found 2 feature files"),
        "stdout must contain exact 'Found 2 feature files' summary; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI --tag filter narrows results and updates summary line
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_tag_filter_narrows_results_and_updates_summary() {
    // @step Given spec/features/auth.feature exists with tag '@critical' and 1 scenario
    let ws = tempfile::tempdir().expect("tempdir");
    mk_features_dir(ws.path());
    write_feature(
        ws.path(),
        "spec/features/auth.feature",
        &feature_body("Auth", &["@critical"], 1),
    );

    // @step Given spec/features/billing.feature exists with tag '@billing' and 1 scenario
    write_feature(
        ws.path(),
        "spec/features/billing.feature",
        &feature_body("Billing", &["@billing"], 1),
    );

    // @step When I run `./codelet/target/release/fspec list-features --tag @critical`
    let (code, stdout, stderr) = run_list_features(ws.path(), &["--tag", "@critical"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-features --tag @critical must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'spec/features/auth.feature'
    assert!(
        stdout.contains("spec/features/auth.feature"),
        "stdout must list spec/features/auth.feature; got:\n{stdout}"
    );

    // @step Then stdout does NOT contain the substring 'spec/features/billing.feature'
    assert!(
        !stdout.contains("spec/features/billing.feature"),
        "stdout must NOT list spec/features/billing.feature when filtered by @critical; got:\n{stdout}"
    );

    // @step Then stdout contains the exact line 'Found 1 feature files matching @critical'
    assert!(
        stdout
            .lines()
            .any(|l| l == "Found 1 feature files matching @critical"),
        "stdout must contain exact matching-summary line; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Default combined TUI mode and other subcommands are preserved
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_list_features() {
    // @step Given the fspec Rust binary has list-features registered as a clap subcommand alongside daemon, client, status, list-work-units, and list-prefixes
    // (asserted by the help-listing check below)

    // @step When I run `./codelet/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn --help");
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code, 0,
        "--help must exit 0; got {code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the help output lists daemon, client, status, list-work-units, list-prefixes, and list-features as available subcommands
    for sub in [
        "daemon",
        "client",
        "status",
        "list-work-units",
        "list-prefixes",
        "list-features",
    ] {
        assert!(
            help.contains(sub),
            "--help must list `{sub}` subcommand; got:\n{help}"
        );
    }

    // @step Then the long-about description still documents that running fspec with no subcommand enters combined TUI mode
    assert!(
        help.contains("combined mode") || help.contains("combined"),
        "--help long-about must document the combined-mode default; got:\n{help}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI bridge module embeds NO duplicated business logic
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_bridge_module_embeds_no_duplicated_business_logic() {
    // @step Given the file codelet/fspec/src/list_features.rs exists as the CLI bridge module
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_features.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/list_features.rs must exist as the CLI bridge module; missing: {}",
        bridge_path.display()
    );

    // @step When I read the bridge module source
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");

    // @step Then the source does NOT contain the substring 'No feature files found'
    assert!(
        !bridge_src.contains("No feature files found"),
        "bridge module must NOT embed sentinel string; got:\n{bridge_src}"
    );

    // @step Then the source does NOT contain the substring 'Found {}'
    assert!(
        !bridge_src.contains("Found {}"),
        "bridge module must NOT embed summary-line prefix; got:\n{bridge_src}"
    );

    // @step Then the source does NOT contain the substring 'scenarioCount'
    assert!(
        !bridge_src.contains("scenarioCount"),
        "bridge module must NOT reference the scenarioCount field name; got:\n{bridge_src}"
    );

    // @step Then the source does NOT contain the substring 'glob_feature_files'
    assert!(
        !bridge_src.contains("glob_feature_files"),
        "bridge module must NOT call glob_feature_files directly; got:\n{bridge_src}"
    );
}
