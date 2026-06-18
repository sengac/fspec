//! CLI surface for the `init` subcommand on the standalone fspec Rust binary
//! — RPC-239.
//!
//! Feature: spec/features/init-cli-subcommand.feature
//!
//! Each scenario maps 1:1 to a Gherkin scenario; @step comments mirror the
//! Gherkin step text verbatim.
//!
//! RED PHASE (Phase B): the `init` clap subcommand is not wired until Phase C
//! and the core impl is still the 1-arg NotYetPorted stub, so these tests are
//! EXPECTED to fail until then.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ---------- helpers ----------

/// Run `fspec init <extra_args>` in `cwd` with HOME redirected into the
/// tempdir (TESTING.md: "redirect, don't intercept") so codex/codex-cli
/// home-dir writes never escape the sandbox.
fn run_init(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("init");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    cmd.env("HOME", cwd);
    let output = cmd.output().expect("spawn fspec init");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ---------- scenarios ----------

#[test]
fn scenario_clap_exposes_init_subcommand_and_prints_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./codelet/target/release/fspec init --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("init")
        .arg("--help")
        .output()
        .expect("spawn fspec init --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "init --help must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the substring 'init'
    assert!(stdout.contains("init"), "help must mention init; got:\n{stdout}");

    // @step Then stdout contains the substring '--agent'
    assert!(
        stdout.contains("--agent"),
        "help must advertise --agent; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_installs_the_claude_agent_and_prints_the_success_summary() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec init --agent claude` from that directory
    let (code, stdout, stderr) = run_init(ws.path(), &["--agent", "claude"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "init must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ Installed fspec for claude'
    assert!(
        stdout.contains("✓ Installed fspec for claude"),
        "stdout must confirm install; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'spec/CLAUDE.md'
    assert!(
        stdout.contains("spec/CLAUDE.md"),
        "stdout must list spec/CLAUDE.md; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Next steps:'
    assert!(
        stdout.contains("Next steps:"),
        "stdout must show Next steps; got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Run /fspec in Claude Code to activate'
    assert!(
        stdout.contains("Run /fspec in Claude Code to activate"),
        "stdout must show the claude activation message; got:\n{stdout}"
    );

    // @step Then spec/CLAUDE.md exists in the directory
    assert!(ws.path().join("spec/CLAUDE.md").exists());

    // @step Then spec/fspec-config.json exists in the directory
    assert!(ws.path().join("spec/fspec-config.json").exists());
}

#[test]
fn scenario_cli_installs_multiple_agents_from_repeated_agent_flags() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec init --agent claude --agent cursor` from that directory
    let (code, stdout, stderr) =
        run_init(ws.path(), &["--agent", "claude", "--agent", "cursor"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "init must exit 0; got {code}, stderr={stderr}");

    // @step Then stdout contains the substring '✓ Installed fspec for claude, cursor'
    assert!(
        stdout.contains("✓ Installed fspec for claude, cursor"),
        "stdout must list both agents; got:\n{stdout}"
    );

    // @step Then spec/CLAUDE.md exists in the directory
    assert!(ws.path().join("spec/CLAUDE.md").exists());

    // @step Then spec/CURSOR.md exists in the directory
    assert!(ws.path().join("spec/CURSOR.md").exists());
}

#[test]
fn scenario_cli_without_agent_fails_because_shell_is_non_tty() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec init` from that directory
    let (code, _stdout, stderr) = run_init(ws.path(), &[]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "init with no --agent must exit 1");

    // @step Then stderr contains the substring 'Interactive mode requires a TTY. Use --agent flag instead:'
    assert!(
        stderr.contains("Interactive mode requires a TTY. Use --agent flag instead:"),
        "stderr must surface the TTY-guard error; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_rejects_an_unknown_agent_id() {
    // @step Given an empty directory is set as the current working directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I run `./codelet/target/release/fspec init --agent bogus` from that directory
    let (code, _stdout, stderr) = run_init(ws.path(), &["--agent", "bogus"]);

    // @step Then the command exits with code 1
    assert_eq!(code, 1, "init with unknown agent must exit 1");

    // @step Then stderr contains the substring '✗ Init failed: Unknown agent: bogus.'
    assert!(
        stderr.contains("✗ Init failed: Unknown agent: bogus."),
        "stderr must surface the unknown-agent error; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given an empty project root directory
    let ws = tempfile::tempdir().expect("tempdir");

    // @step When I dispatch init through fspec_core::dispatch::dispatch_command with agent list ['claude'] against that project root
    let req = codelet_fspec_core::DispatchRequest {
        command: "init".to_string(),
        args_json: serde_json::json!({ "agent": ["claude"] }).to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(result.success, "dispatcher path must succeed; got {result:?}");
    let data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher result reports filesInstalled including 'spec/CLAUDE.md'
    let files: Vec<String> = data["filesInstalled"]
        .as_array()
        .expect("filesInstalled array")
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    assert!(
        files.contains(&"spec/CLAUDE.md".to_string()),
        "filesInstalled must include spec/CLAUDE.md; got: {files:?}"
    );

    // @step Then the CLI bridge module codelet/fspec/src/init.rs contains NO inline scaffolding, registry or template logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/init.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/init.rs must exist as the CLI bridge module"
    );
    let bridge_src = std::fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in ["CLAUDE.md", "create_dir_all", "AGENT_REGISTRY", "docTemplate"] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_INIT: &str = include_str!("fixtures/help/init.txt");

#[test]
fn scenario_init_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec init --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("init")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn init --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "init --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/init.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_INIT);

    // @step And stdout starts with a blank line followed by 'INIT'
    assert!(stdout.starts_with("\nINIT\n"));
}
