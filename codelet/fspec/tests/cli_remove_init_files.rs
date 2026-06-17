//! CLI surface for the `remove-init-files` subcommand on the standalone
//! fspec Rust binary — RPC-276.
//!
//! Feature: spec/features/remove-init-files-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand + help intercept + bridge module are
//! wired (Phase C), these tests fail — the binary rejects the unknown
//! subcommand and the bridge module does not yet exist.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;
use serde_json::Value;

// ───────── helpers ─────────

fn run_remove(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-init-files");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-init-files");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_config(cwd: &Path, agent: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("fspec-config.json"),
        serde_json::json!({ "agent": agent }).to_string(),
    )
    .expect("write fspec-config.json");
}

fn touch(cwd: &Path, rel: &str) {
    let path = cwd.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir parent");
    }
    fs::write(&path, "").expect("touch file");
}

/// Standard claude workspace: config + both agent files present.
fn setup_claude_workspace(cwd: &Path) {
    write_config(cwd, "claude");
    touch(cwd, "spec/CLAUDE.md");
    touch(cwd, ".claude/commands/fspec.md");
}

// ───────── scenarios ─────────

#[test]
fn scenario_clap_exposes_remove_init_files_with_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec remove-init-files --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("remove-init-files")
        .arg("--help")
        .output()
        .expect("spawn remove-init-files --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "remove-init-files --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring 'remove-init-files'
    assert!(
        stdout.contains("remove-init-files") || stdout.contains("REMOVE-INIT-FILES"),
        "help must describe the remove-init-files subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_removes_claude_agent_files_and_prints_the_success_summary() {
    // @step Given a workspace with spec/fspec-config.json containing agent='claude' and the files spec/CLAUDE.md and .claude/commands/fspec.md
    let ws = tempfile::tempdir().expect("tempdir");
    setup_claude_workspace(ws.path());

    // @step When I run `./codelet/target/release/fspec remove-init-files --no-keep-config` from that workspace
    let (code, stdout, stderr) = run_remove(ws.path(), &["--no-keep-config"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring '✓ Successfully removed fspec init files'
    assert!(
        stdout.contains("✓ Successfully removed fspec init files"),
        "got:\n{stdout}"
    );

    // @step And stdout contains the substring 'spec/CLAUDE.md'
    assert!(stdout.contains("spec/CLAUDE.md"), "got:\n{stdout}");

    // @step And spec/CLAUDE.md no longer exists
    assert!(
        !ws.path().join("spec/CLAUDE.md").exists(),
        "spec/CLAUDE.md must be removed"
    );
}

#[test]
fn scenario_cli_keep_config_preserves_spec_fspec_config_json() {
    // @step Given a workspace with spec/fspec-config.json containing agent='claude' and the files spec/CLAUDE.md and .claude/commands/fspec.md
    let ws = tempfile::tempdir().expect("tempdir");
    setup_claude_workspace(ws.path());

    // @step When I run `./codelet/target/release/fspec remove-init-files --keep-config` from that workspace
    let (code, stdout, stderr) = run_remove(ws.path(), &["--keep-config"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "must exit 0; got {code}, stdout={stdout}, stderr={stderr}");

    // @step And spec/fspec-config.json still exists
    assert!(
        ws.path().join("spec/fspec-config.json").exists(),
        "spec/fspec-config.json must be preserved with --keep-config"
    );
}

#[test]
fn scenario_cli_exits_1_when_no_agent_installation_is_detected() {
    // @step Given a workspace with no spec/fspec-config.json and no agent detection directories
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec remove-init-files --no-keep-config` from that workspace
    let (code, stdout, stderr) = run_remove(ws.path(), &["--no-keep-config"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "must exit 1; got {code}, stdout={stdout}");

    // @step And stderr contains the substring 'No fspec agent installation detected. Nothing to remove.'
    assert!(
        stderr.contains("No fspec agent installation detected. Nothing to remove."),
        "stderr:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a workspace with spec/fspec-config.json containing agent='claude' and the files spec/CLAUDE.md and .claude/commands/fspec.md
    let ws = tempfile::tempdir().expect("tempdir");
    setup_claude_workspace(ws.path());

    // @step When I dispatch remove-init-files through fspec_core::dispatch::dispatch_command with keepConfig=true against that workspace
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-init-files".to_string(),
        args_json: r#"{"keepConfig":true}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");

    // @step Then the dispatcher returns JSON whose filesRemoved includes 'spec/CLAUDE.md'
    let parsed: Value = serde_json::from_str(&result.data).expect("dispatcher data must be JSON");
    let removed = parsed["filesRemoved"]
        .as_array()
        .expect("filesRemoved must be an array");
    assert!(
        removed
            .iter()
            .any(|v| v.as_str() == Some("spec/CLAUDE.md")),
        "filesRemoved must include spec/CLAUDE.md; got {}",
        result.data
    );

    // @step And the CLI bridge module codelet/fspec/src/remove_init_files.rs contains NO inline detection or deletion logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_init_files.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_init_files.rs must exist as the CLI bridge module"
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "detectionPaths",
        "docTemplate",
        "remove_file",
        "slashCommandPath",
        "Unknown agent",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-init-files.txt");

#[test]
fn scenario_remove_init_files_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec remove-init-files --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("remove-init-files")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-init-files --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "remove-init-files --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-init-files.txt
    assert_eq!(stdout, TS_HELP_FIXTURE);

    // @step And stdout starts with a blank line followed by 'REMOVE-INIT-FILES'
    assert!(stdout.starts_with("\nREMOVE-INIT-FILES\n"));
}
