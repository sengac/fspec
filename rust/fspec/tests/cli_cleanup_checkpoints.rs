//! CLI surface for the `cleanup-checkpoints` subcommand on the standalone
//! fspec Rust binary — RPC-203.
//!
//! Feature: spec/features/cleanup-checkpoints-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! NOTE (Phase B): the `cleanup-checkpoints` clap subcommand is not wired
//! until Phase C, so these tests are EXPECTED to fail until then.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

fn run_cleanup(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("cleanup-checkpoints");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec cleanup-checkpoints");
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

fn create_checkpoint(dir: &Path, work_unit_id: &str, checkpoint_name: &str) {
    let ref_name = format!("refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}");
    Command::new("git")
        .args(["update-ref", &ref_name, "HEAD"])
        .current_dir(dir)
        .status()
        .expect("git update-ref");
}

fn write_index_file(dir: &Path, work_unit_id: &str, entries: &[(String, String)]) {
    let index_dir = dir.join(".git").join("fspec-checkpoints-index");
    fs::create_dir_all(&index_dir).expect("mkdir index dir");
    let arr: Vec<serde_json::Value> = entries
        .iter()
        .map(|(name, ts)| serde_json::json!({ "name": name, "sha": "deadbeef", "timestamp": ts }))
        .collect();
    let payload = serde_json::json!({ "checkpoints": arr });
    fs::write(
        index_dir.join(format!("{work_unit_id}.json")),
        serde_json::to_string_pretty(&payload).unwrap(),
    )
    .expect("write index file");
}

fn seed_checkpoints(dir: &Path, work_unit_id: &str, n: usize) {
    let mut entries = Vec::new();
    for i in 0..n {
        let name = format!("cp-{i:02}");
        let ts = format!("2026-06-01T{:02}:{:02}:00.000Z", i / 60, i % 60);
        create_checkpoint(dir, work_unit_id, &name);
        entries.push((name, ts));
    }
    write_index_file(dir, work_unit_id, &entries);
}

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_cleanup_checkpoints_with_required_keep_last() {
    // @step Given the fspec Rust binary has been compiled
    // @step When I run "fspec cleanup-checkpoints --help"
    let output = Command::new(fspec_bin())
        .arg("cleanup-checkpoints")
        .arg("--help")
        .output()
        .expect("spawn cleanup-checkpoints --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "cleanup-checkpoints --help must exit 0; got {code}"
    );

    // @step And stdout describes the cleanup-checkpoints subcommand
    assert!(
        stdout.to_lowercase().contains("cleanup-checkpoints")
            || stdout.to_lowercase().contains("clean up"),
        "help must describe cleanup-checkpoints; got:\n{stdout}"
    );

    // @step And stdout advertises the "--keep-last" option
    assert!(
        stdout.contains("--keep-last"),
        "help must advertise --keep-last; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_cleans_up_and_exits_0() {
    // @step Given a git repository with several checkpoints for "AUTH-001" is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    seed_checkpoints(ws.path(), "AUTH-001", 5);

    // @step When I run "fspec cleanup-checkpoints AUTH-001 --keep-last 1" from that directory
    let (code, stdout, stderr) = run_cleanup(ws.path(), &["AUTH-001", "--keep-last", "1"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains "✓ Cleanup complete:"
    assert!(
        stdout.contains("\u{2713} Cleanup complete:"),
        "missing completion banner; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_rejects_a_non_positive_keep_last() {
    // @step Given a git repository is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());

    // @step When I run "fspec cleanup-checkpoints AUTH-001 --keep-last 0" from that directory
    let (code, _stdout, stderr) = run_cleanup(ws.path(), &["AUTH-001", "--keep-last", "0"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "non-positive keep-last must exit 1; got {code}");

    // @step And stderr contains "--keep-last must be a positive number"
    assert!(
        stderr.contains("--keep-last must be a positive number"),
        "missing validation message; got stderr:\n{stderr}"
    );
}

#[test]
fn scenario_cli_rejects_a_non_numeric_keep_last() {
    // @step Given a git repository is the current working directory
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());

    // @step When I run "fspec cleanup-checkpoints AUTH-001 --keep-last abc" from that directory
    let (code, _stdout, stderr) = run_cleanup(ws.path(), &["AUTH-001", "--keep-last", "abc"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "non-numeric keep-last must exit 1; got {code}");

    // @step And stderr contains "--keep-last must be a positive number"
    assert!(
        stderr.contains("--keep-last must be a positive number"),
        "missing validation message; got stderr:\n{stderr}"
    );
}

#[test]
fn scenario_default_combined_tui_mode_preserved_after_adding_cleanup_checkpoints() {
    // @step Given the fspec Rust binary registers cleanup-checkpoints alongside the existing subcommands
    // @step When I run "fspec --help"
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn --help");
    let code = output.status.code().unwrap_or(-1);
    let help = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "--help must exit 0; got {code}");

    // @step And the help output lists "cleanup-checkpoints" as an available subcommand
    assert!(
        help.contains("cleanup-checkpoints"),
        "--help must list cleanup-checkpoints; got:\n{help}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a git repository with several checkpoints for "AUTH-001"
    let ws = tempfile::tempdir().expect("tempdir");
    init_git_repo(ws.path());
    seed_checkpoints(ws.path(), "AUTH-001", 5);

    // @step When I dispatch cleanup-checkpoints through fspec_core::dispatch::dispatch_command with format "json"
    let req = codelet_fspec_core::DispatchRequest {
        command: "cleanup-checkpoints".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","keepLast":2,"format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher result succeeds and reports deletedCount and preservedCount
    assert!(result.success, "dispatcher must succeed; got {result:?}");
    let data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert!(
        data["deletedCount"].is_number(),
        "missing deletedCount; got {}",
        result.data
    );
    assert!(
        data["preservedCount"].is_number(),
        "missing preservedCount; got {}",
        result.data
    );

    // @step And the CLI bridge module rust/fspec/src/cleanup_checkpoints.rs contains NO inline list, sort, delete, or rendering logic — its only computation is arg parsing and JSON marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cleanup_checkpoints.rs");
    assert!(
        bridge_path.exists(),
        "src/cleanup_checkpoints.rs must exist: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "list_ghost_checkpoints",
        "delete_ghost_checkpoint",
        "fspec-checkpoints-index",
        "Cleanup complete",
        "Deleted",
        "Preserved",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}`; got:\n{bridge_src}"
        );
    }
}

#[test]
fn scenario_cleanup_checkpoints_help_is_byte_for_byte_identical_to_ts() {
    // @step Given the fspec Rust binary has been compiled
    // @step When I run "fspec cleanup-checkpoints --help" piped to non-TTY with NO_COLOR set
    let output = Command::new(fspec_bin())
        .arg("cleanup-checkpoints")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn cleanup-checkpoints --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "cleanup-checkpoints --help must exit 0; got {code}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/cleanup-checkpoints.txt
    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/help/cleanup-checkpoints.txt");
    let fixture = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("read help fixture {}: {e}", fixture_path.display()));
    assert_eq!(
        stdout, fixture,
        "cleanup-checkpoints --help must match TS fixture"
    );

    // @step And stdout starts with a blank line followed by "CLEANUP-CHECKPOINTS"
    assert!(
        stdout.starts_with("\nCLEANUP-CHECKPOINTS"),
        "stdout must start with blank line + CLEANUP-CHECKPOINTS; got:\n{stdout}"
    );
}
