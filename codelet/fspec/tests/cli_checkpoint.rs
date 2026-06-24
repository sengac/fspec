//! CLI surface for the `checkpoint` subcommand on the standalone fspec Rust
//! binary — RPC-202.
//!
//! Feature: spec/features/checkpoint-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! NOTE (Phase B): the `checkpoint` clap subcommand is not wired until Phase C,
//! so these tests are EXPECTED to fail until then.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

fn run_checkpoint(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("checkpoint");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec checkpoint");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn init_git_repo(dir: &Path) {
    for args in [
        vec!["init", "--quiet"],
        vec!["config", "user.email", "test@example.com"],
        vec!["config", "user.name", "Test User"],
    ] {
        Command::new("git")
            .args(&args)
            .current_dir(dir)
            .status()
            .expect("git setup");
    }
    fs::write(dir.join("README.md"), "# test\n").expect("seed README");
    Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir)
        .status()
        .expect("git add");
    Command::new("git")
        .args(["commit", "--quiet", "-m", "initial"])
        .current_dir(dir)
        .status()
        .expect("git commit");
}

fn make_dirty(dir: &Path, n: usize) {
    for i in 0..n {
        fs::write(dir.join(format!("dirty-{i}.txt")), format!("change {i}\n"))
            .expect("write dirty file");
    }
}

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_checkpoint_with_two_positionals() {
    // @step Given the fspec Rust binary has been compiled
    // @step When I run "fspec checkpoint --help"
    let output = Command::new(fspec_bin())
        .arg("checkpoint")
        .arg("--help")
        .output()
        .expect("spawn checkpoint --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "checkpoint --help must exit 0; got {code}");

    // @step And stdout describes the checkpoint subcommand
    assert!(
        stdout.to_lowercase().contains("checkpoint"),
        "help must describe the checkpoint subcommand; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring "--workspace"
    assert!(
        !stdout.contains("--workspace"),
        "checkpoint --help must NOT advertise --workspace; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_creates_a_checkpoint_and_exits_0() {
    // @step Given a git repository with uncommitted changes is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    make_dirty(ws.path(), 2);

    // @step When I run "fspec checkpoint AUTH-001 baseline" from that directory
    let (code, stdout, stderr) = run_checkpoint(ws.path(), &["AUTH-001", "baseline"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains "✓ Created checkpoint \"baseline\" for AUTH-001"
    assert!(
        stdout.contains("\u{2713} Created checkpoint \"baseline\" for AUTH-001"),
        "missing banner; got:\n{stdout}"
    );

    // @step And stdout contains "Captured"
    assert!(
        stdout.contains("Captured"),
        "missing Captured; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_exits_1_when_working_tree_is_clean() {
    // @step Given a git repository with no uncommitted changes is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());

    // @step When I run "fspec checkpoint AUTH-001 baseline" from that directory
    let (code, _stdout, _stderr) = run_checkpoint(ws.path(), &["AUTH-001", "baseline"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "clean tree must exit 1; got {code}");
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_checkpoint() {
    // @step Given the fspec Rust binary registers checkpoint alongside the existing subcommands
    // @step When I run "fspec --help"
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn --help");
    let code = output.status.code().unwrap_or(-1);
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; got {code}");

    // @step And the help output lists "checkpoint" as an available subcommand
    assert!(
        help.contains("checkpoint"),
        "--help must list checkpoint; got:\n{help}"
    );

    // @step And the long-about still documents the combined TUI default
    assert!(
        help.contains("combined"),
        "long-about must document combined-mode default; got:\n{help}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a git repository with uncommitted changes
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    make_dirty(ws.path(), 2);

    // @step When I dispatch checkpoint through fspec_core::dispatch::dispatch_command with format "json"
    let req = codelet_fspec_core::DispatchRequest {
        command: "checkpoint".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","checkpointName":"baseline","format":"json"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher result succeeds and reports capturedFiles
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert!(
        data["capturedFiles"].is_array(),
        "dispatcher data must report capturedFiles; got {}",
        result.data
    );

    // @step And the CLI bridge module codelet/fspec/src/checkpoint.rs contains NO inline capture, index-write, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/checkpoint.rs");
    assert!(
        bridge_path.exists(),
        "src/checkpoint.rs must exist as the CLI bridge module: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "create_ghost_commit",
        "fspec-checkpoints-index",
        "Created checkpoint",
        "Captured",
        "capturedFiles",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}

#[test]
fn scenario_checkpoint_help_is_byte_for_byte_identical_to_ts() {
    // @step Given the fspec Rust binary has been compiled
    // @step When I run "fspec checkpoint --help" piped to non-TTY with NO_COLOR set
    let output = Command::new(fspec_bin())
        .arg("checkpoint")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn checkpoint --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "checkpoint --help must exit 0; got {code}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/checkpoint.txt
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/help/checkpoint.txt");
    let fixture = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("read help fixture {}: {e}", fixture_path.display()));
    assert_eq!(stdout, fixture, "checkpoint --help must match TS fixture");

    // @step And stdout starts with a blank line followed by "CHECKPOINT"
    assert!(
        stdout.starts_with("\nCHECKPOINT"),
        "stdout must start with blank line + CHECKPOINT; got:\n{stdout}"
    );
}
