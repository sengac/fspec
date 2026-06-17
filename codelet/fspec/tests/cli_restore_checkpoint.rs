//! CLI surface for the `restore-checkpoint` subcommand on the standalone
//! fspec Rust binary — RPC-288.
//!
//! Feature: spec/features/restore-checkpoint-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! NOTE (Phase B): the `restore-checkpoint` clap subcommand is not wired until
//! Phase C, so these tests are EXPECTED to fail until then. Fixtures create
//! real ghost-commit checkpoints by invoking the binary's own `checkpoint`
//! subcommand (avoids a codelet-git dependency on the fspec crate).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

fn run_restore(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("restore-checkpoint");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec restore-checkpoint");
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

fn git_commit_all(dir: &Path, msg: &str) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .status()
        .expect("git add -A");
    Command::new("git")
        .args(["commit", "--quiet", "-m", msg])
        .current_dir(dir)
        .status()
        .expect("git commit");
}

/// Create a real ghost-commit checkpoint by writing `marker.txt` and invoking
/// the binary's own `checkpoint` subcommand. Returns nothing; panics if the
/// underlying checkpoint capture exits non-zero.
fn create_checkpoint_via_cli(dir: &Path, work_unit_id: &str, checkpoint_name: &str, content: &str) {
    fs::write(dir.join("marker.txt"), content).expect("write marker");
    let output = Command::new(fspec_bin())
        .arg("checkpoint")
        .arg(work_unit_id)
        .arg(checkpoint_name)
        .current_dir(dir)
        .output()
        .expect("spawn fspec checkpoint (fixture)");
    assert!(
        output.status.success(),
        "fixture checkpoint capture failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_restore_checkpoint_with_two_positionals() {
    // @step Given the fspec Rust binary has been compiled
    // @step When I run "fspec restore-checkpoint --help"
    let output = Command::new(fspec_bin())
        .arg("restore-checkpoint")
        .arg("--help")
        .output()
        .expect("spawn restore-checkpoint --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "restore-checkpoint --help must exit 0; got {code}");

    // @step And stdout describes the restore-checkpoint subcommand
    assert!(
        stdout.to_lowercase().contains("restore"),
        "help must describe restore-checkpoint; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring "--force"
    assert!(
        !stdout.contains("--force"),
        "restore-checkpoint --help must NOT advertise --force; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_restores_against_a_clean_working_tree_and_exits_0() {
    // @step Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    create_checkpoint_via_cli(ws.path(), "AUTH-001", "baseline", "captured\n");
    git_commit_all(ws.path(), "commit checkpoint content");

    // @step When I run "fspec restore-checkpoint AUTH-001 baseline" from that directory
    let (code, stdout, stderr) = run_restore(ws.path(), &["AUTH-001", "baseline"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains "✓ Restored checkpoint \"baseline\" for AUTH-001"
    assert!(
        stdout.contains("\u{2713} Restored checkpoint \"baseline\" for AUTH-001"),
        "missing restore banner; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_exits_1_with_rerun_hint_when_working_tree_is_dirty() {
    // @step Given a git repository with a checkpoint "baseline" for "AUTH-001" and uncommitted changes is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    create_checkpoint_via_cli(ws.path(), "AUTH-001", "baseline", "captured\n");
    git_commit_all(ws.path(), "commit checkpoint content");
    fs::write(ws.path().join("uncommitted.txt"), "dirty\n").expect("dirty file");

    // @step When I run "fspec restore-checkpoint AUTH-001 baseline" from that directory
    let (code, stdout, _stderr) = run_restore(ws.path(), &["AUTH-001", "baseline"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "dirty tree must exit 1; got {code}");

    // @step And stdout contains "Re-run with user choice to proceed with restoration"
    assert!(
        stdout.contains("Re-run with user choice to proceed with restoration"),
        "missing re-run hint; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_exits_1_when_the_checkpoint_does_not_exist() {
    // @step Given a git repository with no checkpoint named "ghost" for "AUTH-001" is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());

    // @step When I run "fspec restore-checkpoint AUTH-001 ghost" from that directory
    let (code, _stdout, _stderr) = run_restore(ws.path(), &["AUTH-001", "ghost"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "missing checkpoint must exit 1; got {code}");
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_restore_checkpoint() {
    // @step Given the fspec Rust binary registers restore-checkpoint alongside the existing subcommands
    // @step When I run "fspec --help"
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn --help");
    let code = output.status.code().unwrap_or(-1);
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; got {code}");

    // @step And the help output lists "restore-checkpoint" as an available subcommand
    assert!(
        help.contains("restore-checkpoint"),
        "--help must list restore-checkpoint; got:\n{help}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a git repository with a checkpoint "baseline" for "AUTH-001" and a clean working tree
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    create_checkpoint_via_cli(ws.path(), "AUTH-001", "baseline", "captured\n");
    git_commit_all(ws.path(), "commit checkpoint content");

    // @step When I dispatch restore-checkpoint through fspec_core::dispatch::dispatch_command with format "json"
    let req = codelet_fspec_core::DispatchRequest {
        command: "restore-checkpoint".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","checkpointName":"baseline","format":"json"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher result succeeds and reports conflictsDetected
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert!(
        data["conflictsDetected"].is_boolean(),
        "dispatcher data must report conflictsDetected; got {}",
        result.data
    );

    // @step And the CLI bridge module codelet/fspec/src/restore_checkpoint.rs contains NO inline dirty-check, conflict-detection, restore, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/restore_checkpoint.rs");
    assert!(
        bridge_path.exists(),
        "src/restore_checkpoint.rs must exist: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "restore_ghost_commit",
        "get_checkpoint_diff_files",
        "get_unstaged_files",
        "CHECKPOINT RESTORATION CONFLICT DETECTED",
        "Restored checkpoint",
        "conflictsDetected",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}

#[test]
fn scenario_restore_checkpoint_help_is_byte_for_byte_identical_to_ts() {
    // @step Given the fspec Rust binary has been compiled
    // @step When I run "fspec restore-checkpoint --help" piped to non-TTY with NO_COLOR set
    let output = Command::new(fspec_bin())
        .arg("restore-checkpoint")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn restore-checkpoint --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "restore-checkpoint --help must exit 0; got {code}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/restore-checkpoint.txt
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/help/restore-checkpoint.txt");
    let fixture = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("read help fixture {}: {e}", fixture_path.display()));
    assert_eq!(stdout, fixture, "restore-checkpoint --help must match TS fixture");

    // @step And stdout starts with a blank line followed by "RESTORE-CHECKPOINT"
    assert!(
        stdout.starts_with("\nRESTORE-CHECKPOINT"),
        "stdout must start with blank line + RESTORE-CHECKPOINT; got:\n{stdout}"
    );
}
