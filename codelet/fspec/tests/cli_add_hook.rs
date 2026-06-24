//! CLI surface for the `add-hook` subcommand on the standalone fspec Rust
//! binary — RPC-184.
//!
//! Feature: spec/features/add-hook-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//!   - `codelet/fspec/src/main.rs` does not yet register an `add-hook`
//!     clap subcommand (clap exits 2 for unrecognized subcommand), and
//!   - `codelet/fspec-core/src/commands/add_hook.rs` is still a
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

fn run_add_hook(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-hook");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-hook");
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

/// Captured byte-exact TS reference output of
/// `node dist/index.js add-hook --help` when piped to non-TTY (no colour).
const TS_HELP_FIXTURE: &str = include_str!("fixtures/help/add-hook.txt");

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-hook --help is byte-for-byte identical to TS reference output
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_add_hook_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec.)

    // @step When I run `fspec add-hook --help` with NO_COLOR=1
    let output = Command::new(fspec_bin())
        .arg("add-hook")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-hook --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "add-hook --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the captured TS help fixture at codelet/fspec/tests/fixtures/help/add-hook.txt
    assert_eq!(
        stdout, TS_HELP_FIXTURE,
        "add-hook --help output must be byte-for-byte identical to TS reference"
    );

    // @step Then stdout contains the section header "USAGE" followed by "  fspec add-hook <event> <name> --command <path> [options]"
    assert!(
        stdout.contains("USAGE\n  fspec add-hook <event> <name> --command <path> [options]\n"),
        "help must contain USAGE block with the documented signature"
    );

    // @step Then stdout contains the section header "ARGUMENTS"
    assert!(
        stdout.contains("ARGUMENTS\n"),
        "help must contain ARGUMENTS section"
    );

    // @step Then stdout contains the section header "OPTIONS"
    assert!(
        stdout.contains("OPTIONS\n"),
        "help must contain OPTIONS section"
    );

    // @step Then stdout contains the substring '--command <path>'
    assert!(
        stdout.contains("--command <path>"),
        "help must advertise --command <path>"
    );

    // @step Then stdout contains the substring '--blocking'
    assert!(
        stdout.contains("--blocking"),
        "help must advertise --blocking"
    );

    // @step Then stdout contains the substring '--timeout <seconds>'
    assert!(
        stdout.contains("--timeout <seconds>"),
        "help must advertise --timeout <seconds>"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI writes zero bytes to stdout on success and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_writes_zero_stdout_bytes_on_success() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let tmp = tempfile::tempdir().expect("tempdir");
    assert!(!tmp.path().join("spec").exists());

    // @step When I run `fspec add-hook pre-implementing lint --command spec/hooks/lint.sh`
    let (code, stdout, stderr) = run_add_hook(
        tmp.path(),
        &[
            "pre-implementing",
            "lint",
            "--command",
            "spec/hooks/lint.sh",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step Then stdout is exactly zero bytes
    assert!(stdout.is_empty(), "expected empty stdout, got {stdout:?}");

    // @step Then spec/fspec-hooks.json was created in the directory
    assert!(
        tmp.path().join("spec/fspec-hooks.json").exists(),
        "config file must be created"
    );

    // @step Then the new file contains exactly one entry under 'pre-implementing' with name='lint' and command='spec/hooks/lint.sh'
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"].as_str(), Some("lint"));
    assert_eq!(arr[0]["command"].as_str(), Some("spec/hooks/lint.sh"));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI passes --blocking and --timeout as JSON marshalling fields
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_marshals_blocking_and_timeout_flags() {
    // @step Given an empty directory with no spec/ subdirectory is set as the current working directory
    let tmp = tempfile::tempdir().expect("tempdir");

    // @step When I run `fspec add-hook post-implementing test --command spec/hooks/test.sh --blocking --timeout 300`
    let (code, _stdout, stderr) = run_add_hook(
        tmp.path(),
        &[
            "post-implementing",
            "test",
            "--command",
            "spec/hooks/test.sh",
            "--blocking",
            "--timeout",
            "300",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step Then the on-disk entry under 'post-implementing' has name='test', blocking=true, and timeout=300
    let v = read_hooks_json(tmp.path());
    let entry = &v["hooks"]["post-implementing"][0];
    assert_eq!(entry["name"].as_str(), Some("test"));
    assert_eq!(entry["blocking"].as_bool(), Some(true));
    assert_eq!(entry["timeout"].as_u64(), Some(300));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI is silent when invoked against a populated config
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_is_silent_against_populated_config() {
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

    // @step When I run `fspec add-hook pre-implementing test --command spec/hooks/test.sh`
    let (code, stdout, stderr) = run_add_hook(
        tmp.path(),
        &[
            "pre-implementing",
            "test",
            "--command",
            "spec/hooks/test.sh",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; stderr={stderr}");

    // @step Then stdout is exactly zero bytes
    assert!(stdout.is_empty(), "expected empty stdout, got {stdout:?}");

    // @step Then the on-disk 'pre-implementing' array has exactly two entries
    let v = read_hooks_json(tmp.path());
    let arr = v["hooks"]["pre-implementing"].as_array().expect("array");
    assert_eq!(arr.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function as the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/fspec-hooks.json contains event 'post-implementing' with hooks ['lint']
    let tmp = tempfile::tempdir().expect("tempdir");
    write_hooks(
        tmp.path(),
        r#"{
  "hooks": {
    "post-implementing": [
      { "name": "lint", "command": "spec/hooks/lint.sh", "blocking": false }
    ]
  }
}"#,
    );

    // @step When I dispatch add-hook through fspec_core::dispatch::dispatch_command with event='post-implementing' name='test' command='t.sh' blocking=false
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-hook".to_string(),
        args_json: serde_json::json!({
            "event": "post-implementing",
            "name": "test",
            "command": "t.sh",
            "blocking": false,
        })
        .to_string(),
        project_root: tmp.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the CLI bridge module codelet/fspec/src/add_hook.rs contains NO inline parsing or write logic — its only computation is JSON arg marshalling
    let crate_root = common::fspec_crate_root();
    let bridge = crate_root.join("src").join("add_hook.rs");
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
        bridge_code.contains("add_hook::run"),
        "CLI bridge MUST delegate to fspec_core::commands::add_hook::run; got:\n{bridge_code}"
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
            // line comment — skip to end of line (preserve the newline)
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
