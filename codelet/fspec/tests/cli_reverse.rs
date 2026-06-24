//! CLI surface for the `reverse` subcommand on the standalone fspec Rust
//! binary — RPC-294.
//!
//! Feature: spec/features/reverse-cli-subcommand.feature
//!
//! RED PHASE: these tests exercise the (to-be-wired) clap subcommand
//! `Mode::Reverse` in `codelet/fspec/src/main.rs` and the ported
//! `codelet/fspec-core/src/commands/reverse.rs`. Each scenario maps 1:1 to a
//! Gherkin scenario; @step comments mirror the step text verbatim. Until the
//! shared wiring + port land, these fail (unknown command / NotYetPorted /
//! help-fixture mismatch) which is the correct red signal.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use codelet_fspec_core::types::reverse_session::session_path;
use common::fspec_bin;
use serde_json::json;
use tempfile::TempDir;

// ---------- helpers ----------

fn run_reverse(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("reverse");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec reverse");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// A project-root tempdir (Cargo.toml boundary marker) whose session file is
/// removed before and after the test to keep temp state clean.
struct Workspace {
    dir: TempDir,
}

impl Workspace {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("marker");
        let ws = Self { dir };
        ws.clear_session();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn session_file(&self) -> std::path::PathBuf {
        session_path(self.dir.path())
    }

    fn clear_session(&self) {
        let _ = fs::remove_file(self.session_file());
    }

    fn write_session(&self, session: &serde_json::Value) {
        fs::write(
            self.session_file(),
            serde_json::to_string_pretty(session).unwrap(),
        )
        .expect("write session");
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        self.clear_session();
    }
}

fn executing_session(current: u64, total: u64, files: &[&str]) -> serde_json::Value {
    json!({
        "phase": "executing",
        "strategy": "A",
        "strategyName": "Spec Gap Filling",
        "currentStep": current,
        "totalSteps": total,
        "gaps": {
            "testsWithoutFeatures": files.len(),
            "featuresWithoutTests": 0,
            "unmappedScenarios": 0,
            "unmappedImplementation": 0,
            "files": files,
        },
        "timestamp": "2026-06-01T00:00:00.000Z"
    })
}

fn write_test_files(root: &Path, names: &[&str]) {
    let dir = root.join("src/__tests__");
    fs::create_dir_all(&dir).expect("mkdir tests");
    for n in names {
        fs::write(dir.join(n), "// test\n").expect("write test file");
    }
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/reverse.txt");

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_reverse_with_byte_parity_help() {
    // Scenario: Clap exposes reverse as a subcommand with all six flags and prints byte-parity help

    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `fspec reverse --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("reverse")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn fspec reverse --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "reverse --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/reverse.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step Then stdout starts with a blank line followed by "REVERSE"
    assert!(
        stdout.starts_with("\nREVERSE\n"),
        "help must start with blank line + REVERSE; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_reset_deletes_the_session_and_prints_session_reset() {
    // Scenario: CLI reset deletes the session and prints Session reset

    // @step Given a temp working directory marked as a project root with an active reverse session file
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));
    assert!(ws.session_file().exists());

    // @step When I run `fspec reverse --reset` from that directory
    let (code, stdout, stderr) = run_reverse(ws.path(), &["--reset"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "reverse --reset must exit 0; stderr={stderr}");

    // @step Then stdout contains the substring "Session reset"
    assert!(
        stdout.contains("Session reset"),
        "stdout must contain 'Session reset'; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_status_with_no_session_prints_no_active_session_and_exits_0() {
    // Scenario: CLI status with no session prints no active session and exits 0

    // @step Given a temp working directory marked as a project root with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I run `fspec reverse --status` from that directory
    let (code, stdout, stderr) = run_reverse(ws.path(), &["--status"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "reverse --status must exit 0; stderr={stderr}");

    // @step Then stdout contains the substring "No active reverse session"
    assert!(
        stdout.contains("No active reverse session"),
        "stdout must contain 'No active reverse session'; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_complete_with_no_session_exits_1() {
    // Scenario: CLI complete with no session exits 1

    // @step Given a temp working directory marked as a project root with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I run `fspec reverse --complete` from that directory
    let (code, stdout, _stderr) = run_reverse(ws.path(), &["--complete"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "reverse --complete with no session must exit 1; stdout={stdout}"
    );

    // @step Then stdout contains the substring "No active reverse session to complete"
    assert!(
        stdout.contains("No active reverse session to complete"),
        "stdout must contain the canonical message; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_initial_analysis_prints_gap_analysis_guidance_and_exits_0() {
    // Scenario: CLI initial analysis prints gap-analysis guidance and exits 0

    // @step Given a temp working directory marked as a project root with three *.test.ts files under src/__tests__, no spec/features directory, and no session file
    let ws = Workspace::new();
    write_test_files(ws.path(), &["a.test.ts", "b.test.ts", "c.test.ts"]);
    assert!(!ws.path().join("spec/features").exists());
    assert!(!ws.session_file().exists());

    // @step When I run `fspec reverse` from that directory
    let (code, stdout, stderr) = run_reverse(ws.path(), &[]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "reverse must exit 0 on initial analysis; stderr={stderr}"
    );

    // @step Then stdout contains the substring "Gap analysis complete."
    assert!(
        stdout.contains("Gap analysis complete."),
        "stdout must contain 'Gap analysis complete.'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring "Strategy A (Spec Gap Filling)"
    assert!(
        stdout.contains("Strategy A (Spec Gap Filling)"),
        "stdout must contain the suggested strategy; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_existing_session_detected_prints_suggestions_and_exits_1() {
    // Scenario: CLI existing session detected prints suggestions under Next steps and exits 1

    // @step Given a temp working directory marked as a project root with a parseable executing reverse session file
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I run `fspec reverse` from that directory
    let (code, stdout, _stderr) = run_reverse(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "existing-session detection must exit 1; stdout={stdout}"
    );

    // @step Then stdout contains the substring "Existing reverse session detected"
    assert!(
        stdout.contains("Existing reverse session detected"),
        "stdout must contain the detection message; got:\n{stdout}"
    );

    // @step Then stdout contains the substring "Next steps:"
    assert!(
        stdout.contains("Next steps:"),
        "stdout must contain 'Next steps:'; got:\n{stdout}"
    );

    // @step Then stdout contains the substring "  - fspec reverse --continue"
    assert!(
        stdout.contains("  - fspec reverse --continue"),
        "stdout must list the --continue suggestion; got:\n{stdout}"
    );
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_reverse() {
    // Scenario: Default combined TUI mode is preserved when no subcommand is provided

    // @step Given the fspec Rust binary has reverse registered as a clap subcommand alongside the existing subcommands
    // (asserted by the help-listing check below)

    // @step When I run `fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");

    // @step Then the command exits 0
    let code = output.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "fspec --help must exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // @step Then the help output lists reverse as an available subcommand
    let help = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        help.contains("reverse"),
        "fspec --help must list the `reverse` subcommand; got:\n{help}"
    );
}
