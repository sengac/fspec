//! CLI surface for the `remove-hook` subcommand on the standalone fspec
//! Rust binary — RPC-275.
//!
//! Feature: spec/features/remove-hook-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//!   - `rust/fspec/src/main.rs` does not yet register a `remove-hook`
//!     clap subcommand (clap exits 2 for unrecognized subcommand), and
//!   - `rust/fspec-core/src/commands/remove_hook.rs` is still a
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

fn run_remove_hook(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-hook");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-hook");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_hooks(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("fspec-hooks.json"), raw).expect("write fspec-hooks.json");
}

fn read_hooks_json(cwd: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(cwd.join("spec").join("fspec-hooks.json"))
        .expect("read fspec-hooks.json");
    serde_json::from_str(&raw).expect("parse fspec-hooks.json")
}

fn read_hooks_raw(cwd: &Path) -> String {
    fs::read_to_string(cwd.join("spec").join("fspec-hooks.json"))
        .expect("read fspec-hooks.json raw")
}

/// Captured byte-exact TS reference output of
/// `node dist/index.js remove-hook --help` when piped to non-TTY (no colour).
const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/remove-hook.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: remove-hook --help is byte-for-byte identical to TS reference output
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_remove_hook_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary has been compiled

    // @step When I run `fspec remove-hook --help` with NO_COLOR=1
    let output = Command::new(fspec_bin())
        .arg("remove-hook")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-hook --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "remove-hook --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the captured TS help fixture at rust/fspec/tests/fixtures/help/remove-hook.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "remove-hook --help output must be byte-for-byte identical to TS reference"
    );

    // @step Then stdout contains the section header "USAGE" followed by "  fspec remove-hook <event> <name>"
    assert!(
        stdout.contains("USAGE\n  fspec remove-hook <event> <name>\n"),
        "help must contain USAGE block with the documented signature"
    );

    // @step Then stdout contains the section header "ARGUMENTS"
    assert!(
        stdout.contains("ARGUMENTS\n"),
        "help must contain ARGUMENTS section"
    );

    // @step Then stdout contains the section header "OPTIONS" followed by "  No options available"
    assert!(
        stdout.contains("OPTIONS\n  No options available\n"),
        "help must contain OPTIONS\\n  No options available"
    );

    // @step Then stdout does NOT contain the substring '--command'
    assert!(
        !stdout.contains("--command"),
        "help must NOT advertise --command"
    );

    // @step Then stdout does NOT contain the substring '--blocking'
    assert!(
        !stdout.contains("--blocking"),
        "help must NOT advertise --blocking"
    );

    // @step Then stdout does NOT contain the substring '--timeout'
    assert!(
        !stdout.contains("--timeout"),
        "help must NOT advertise --timeout"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI writes zero bytes to stdout on success and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_writes_zero_stdout_bytes_on_success() {
    // @step Given spec/fspec-hooks.json contains event 'post-implementing' with entries named 'lint' and 'test'
    let tmp = tempfile::tempdir().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false },
      { "name": "test", "command": "spec/hooks/test.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I run `fspec remove-hook post-implementing lint`
    let (code, stdout, stderr) = run_remove_hook(tmp.path(), &["post-implementing", "lint"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step Then stdout is exactly zero bytes
    assert!(stdout.is_empty(), "expected empty stdout, got {stdout:?}");

    // @step Then the on-disk 'post-implementing' array has exactly one entry named 'test'
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["post-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("test"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits non-zero with error when spec/fspec-hooks.json is missing
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_nonzero_when_config_is_missing() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I run `fspec remove-hook pre-implementing lint`
    let (code, _stdout, stderr) = run_remove_hook(tmp.path(), &["pre-implementing", "lint"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "expected exit 1 on missing config; stderr={stderr}"
    );

    // @step Then stderr starts with 'Error:'
    assert!(
        stderr.starts_with("Error:"),
        "stderr must start with 'Error:'; got {stderr:?}"
    );

    // @step Then spec/fspec-hooks.json was NOT created in the directory
    assert!(
        !tmp.path().join("spec/fspec-hooks.json").exists(),
        "remove-hook must NOT auto-create the config file"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits non-zero with error when spec/fspec-hooks.json is invalid JSON
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_nonzero_when_config_is_invalid_json() {
    // @step Given spec/fspec-hooks.json exists in the working directory but contains invalid JSON syntax
    let tmp = tempfile::tempdir().expect("tempdir");
    write_hooks(tmp.path(), "{ not json");

    // @step When I run `fspec remove-hook pre-implementing lint`
    let (code, _stdout, stderr) = run_remove_hook(tmp.path(), &["pre-implementing", "lint"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1 on parse error; stderr={stderr}");

    // @step Then stderr starts with 'Error:'
    assert!(
        stderr.starts_with("Error:"),
        "stderr must start with 'Error:'; got {stderr:?}"
    );

    // @step Then the raw bytes of spec/fspec-hooks.json are unchanged
    let raw = read_hooks_raw(tmp.path());
    assert_eq!(
        raw, "{ not json",
        "config file must be unchanged on parse error"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI is silent when called with a no-op (missing key/name)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_silent_noop_when_name_not_in_event_array() {
    // @step Given spec/fspec-hooks.json contains event 'pre-implementing' with a single entry named 'lint'
    let tmp = tempfile::tempdir().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "pre-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I run `fspec remove-hook pre-implementing nonexistent`
    let (code, stdout, stderr) = run_remove_hook(tmp.path(), &["pre-implementing", "nonexistent"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0 on silent no-op; stderr={stderr}");

    // @step Then stdout is exactly zero bytes
    assert!(stdout.is_empty(), "expected empty stdout, got {stdout:?}");

    // @step Then the on-disk 'pre-implementing' array is unchanged (one entry named 'lint')
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/fspec-hooks.json contains event 'post-implementing' with hooks ['lint','test']
    let tmp = tempfile::tempdir().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false },
      { "name": "test", "command": "spec/hooks/test.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch remove-hook through fspec_core::dispatch::dispatch_command with event='post-implementing' name='lint'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-hook".to_string(),
        args_json: serde_json::json!({ "event": "post-implementing", "name": "lint" }).to_string(),
        project_root: tmp.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the CLI bridge module rust/fspec/src/remove_hook.rs contains NO inline parsing or write logic — its only computation is JSON arg marshalling
    let crate_root = common::fspec_crate_root();
    let bridge = crate_root.join("src").join("remove_hook.rs");
    let bridge_src = fs::read_to_string(&bridge)
        .unwrap_or_else(|e| panic!("read CLI bridge at {bridge:?}: {e}"));
    let bridge_code = strip_rust_comments(&bridge_src);
    assert!(
        !bridge_code.contains("write_json_atomic"),
        "CLI bridge MUST NOT perform on-disk writes; got:\n{bridge_code}"
    );
    assert!(
        !bridge_code.contains("fs::write")
            && !bridge_code.contains("tokio::fs::write")
            && !bridge_code.contains("OpenOptions"),
        "CLI bridge MUST NOT invoke filesystem write APIs; got:\n{bridge_code}"
    );
    assert!(
        !bridge_code.contains("read_to_string"),
        "CLI bridge MUST NOT read fspec-hooks.json directly; got:\n{bridge_code}"
    );
    assert!(
        bridge_code.contains("remove_hook::run"),
        "CLI bridge MUST delegate to fspec_core::commands::remove_hook::run; got:\n{bridge_code}"
    );
}

/// Strip Rust line comments (`//…`) and the contents of block comments
/// (`/* … */`) so doc-comment prose in the bridge module (which legitimately
/// mentions `write_json_atomic` / `read_to_string` to spell out the contract)
/// does not falsely trigger the no-domain-logic asserts above. We only care
/// about CODE — not commentary.
fn strip_rust_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut in_block = false;
    while i < bytes.len() {
        if in_block {
            if i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                in_block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            in_block = true;
            i += 2;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}
