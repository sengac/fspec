//! CLI surface for the `add-virtual-hook` subcommand on the standalone
//! fspec Rust binary — RPC-195.
//!
//! Feature: spec/features/add-virtual-hook-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//! `rust/fspec/src/main.rs` does not yet register an `add-virtual-hook`
//! clap subcommand (clap returns exit code 2 for "unrecognized
//! subcommand"). Once the orchestrator wires the `Mode::AddVirtualHook`
//! clap variant and dispatch arm into `main.rs`, the tests turn green.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

fn run_add_virtual_hook(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("add-virtual-hook");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec add-virtual-hook");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn write_work_units(cwd: &Path, raw: &str) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(spec.join("work-units.json"), raw).expect("write work-units.json");
}

fn read_work_units(cwd: &Path) -> serde_json::Value {
    let raw =
        fs::read_to_string(cwd.join("spec").join("work-units.json")).expect("read work-units.json");
    serde_json::from_str(&raw).expect("parse work-units.json")
}

/// Build a single-work-unit JSON document.
fn work_units_with_optional_hooks(id: &str, hooks: Option<&[serde_json::Value]>) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".to_string(), serde_json::json!(id));
    wu.insert(
        "title".to_string(),
        serde_json::json!(format!("title for {id}")),
    );
    wu.insert("status".to_string(), serde_json::json!("backlog"));
    wu.insert(
        "createdAt".to_string(),
        serde_json::json!("2026-06-01T00:00:00.000Z"),
    );
    wu.insert(
        "updatedAt".to_string(),
        serde_json::json!("2026-06-01T00:00:00.000Z"),
    );
    if let Some(h) = hooks {
        wu.insert(
            "virtualHooks".to_string(),
            serde_json::Value::Array(h.to_vec()),
        );
    }

    let mut wus = serde_json::Map::new();
    wus.insert(id.to_string(), serde_json::Value::Object(wu));

    serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": serde_json::Value::Object(wus),
        "states": {
            "backlog": [id], "specifying": [], "testing": [],
            "implementing": [], "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap()
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes add-virtual-hook as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_add_virtual_hook_with_flag_aware_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec add-virtual-hook --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("add-virtual-hook")
        .arg("--help")
        .output()
        .expect("spawn fspec add-virtual-hook --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec add-virtual-hook --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains clap-generated help describing the add-virtual-hook subcommand
    assert!(
        stdout.contains("add-virtual-hook") || stdout.contains("virtual hook"),
        "help must describe the add-virtual-hook subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the positional placeholder "<workUnitId>"
    assert!(
        stdout.contains("<workUnitId>"),
        "help must show <workUnitId> placeholder; got:\n{stdout}"
    );

    // @step And stdout contains the positional placeholder "<event>"
    assert!(
        stdout.contains("<event>"),
        "help must show <event> placeholder; got:\n{stdout}"
    );

    // @step And stdout contains the positional placeholder "<command>"
    assert!(
        stdout.contains("<command>"),
        "help must show <command> placeholder; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--blocking'
    assert!(
        stdout.contains("--blocking"),
        "help must advertise --blocking; got:\n{stdout}"
    );

    // @step And stdout contains the substring '--git-context'
    assert!(
        stdout.contains("--git-context"),
        "help must advertise --git-context; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI adds a simple hook and prints the canonical success lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_adds_simple_hook_and_prints_canonical_success_lines() {
    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_optional_hooks("AUTH-001", None));

    // @step When I run `./rust/target/release/fspec add-virtual-hook AUTH-001 post-implementing "npm test" --blocking`
    let (code, stdout, stderr) = run_add_virtual_hook(
        ws.path(),
        &["AUTH-001", "post-implementing", "npm test", "--blocking"],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "exit code must be 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring '✓ Virtual hook added to AUTH-001'
    assert!(
        stdout.contains("✓ Virtual hook added to AUTH-001"),
        "stdout must contain success message; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Total virtual hooks: 1'
    assert!(
        stdout.contains("  Total virtual hooks: 1"),
        "stdout must contain count line; got:\n{stdout}"
    );

    // @step And the on-disk virtualHooks array for AUTH-001 has length 1
    let v = read_work_units(ws.path());
    let hooks = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array");
    assert_eq!(hooks.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when the work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_work_unit_does_not_exist() {
    // @step Given spec/work-units.json contains AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_optional_hooks("AUTH-001", None));

    // @step When I run `./rust/target/release/fspec add-virtual-hook AUTH-999 post-implementing "npm test"`
    let (code, _stdout, stderr) =
        run_add_virtual_hook(ws.path(), &["AUTH-999", "post-implementing", "npm test"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1; got {code}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to add virtual hook:'
    assert!(
        stderr.contains("✗ Failed to add virtual hook:"),
        "stderr must contain canonical error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Work unit 'AUTH-999' does not exist"
    assert!(
        stderr.contains("Work unit 'AUTH-999' does not exist"),
        "stderr must mention missing work unit; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI with --git-context generates a shell script and stores its relative path
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_with_git_context_generates_script_and_stores_relative_path() {
    // @step Given an empty project root directory with an AUTH-001 work unit
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_optional_hooks("AUTH-001", None));

    // @step When I run `./rust/target/release/fspec add-virtual-hook AUTH-001 post-implementing "eslint src/" --git-context --blocking`
    let (code, _stdout, stderr) = run_add_virtual_hook(
        ws.path(),
        &[
            "AUTH-001",
            "post-implementing",
            "eslint src/",
            "--git-context",
            "--blocking",
        ],
    );

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; got {code}, stderr={stderr}");

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists
    let script_path = ws.path().join("spec/hooks/.virtual/AUTH-001-eslint.sh");
    assert!(
        script_path.exists(),
        "expected generated script at {}",
        script_path.display()
    );

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh has Unix permission bits 0o755
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&script_path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o755, "expected mode 0o755, got 0o{mode:o}");
    }

    // @step And the on-disk virtualHooks[0].command equals 'spec/hooks/.virtual/AUTH-001-eslint.sh'
    let v = read_work_units(ws.path());
    let hook = &v["workUnits"]["AUTH-001"]["virtualHooks"][0];
    assert_eq!(
        hook["command"].as_str(),
        Some("spec/hooks/.virtual/AUTH-001-eslint.sh")
    );

    // @step And the on-disk virtualHooks[0].gitContext equals true
    assert_eq!(hook["gitContext"].as_bool(), Some(true));
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_optional_hooks("AUTH-001", None));

    // @step When I dispatch add-virtual-hook through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' event='post-implementing' command='npm test'
    let req = codelet_fspec_core::DispatchRequest {
        command: "add-virtual-hook".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","event":"post-implementing","command":"npm test"}"#
            .to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with hookCount=1
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(
        dispatcher_data["hookCount"].as_u64(),
        Some(1),
        "expected hookCount=1; got {dispatcher_data:?}"
    );

    // @step And the CLI bridge module rust/fspec/src/add_virtual_hook.rs contains NO inline script-generation, hook-name-derivation, or work-unit-lookup logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/add_virtual_hook.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/add_virtual_hook.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    let stripped = common::strip_comments(&bridge_src);
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "virtualHooks",
        ".sh",
        "#!/bin/bash",
        "PermissionsExt",
        ".split(' ')",
        ".rsplit('/')",
    ] {
        assert!(
            !stripped.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{stripped}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: add-virtual-hook --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_AVH: &str = include_str!("fixtures/help/add-virtual-hook.txt");

#[test]
fn scenario_add_virtual_hook_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec add-virtual-hook --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("add-virtual-hook")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn add-virtual-hook --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "add-virtual-hook --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/add-virtual-hook.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_AVH);

    // @step And stdout starts with a blank line followed by 'ADD-VIRTUAL-HOOK'
    assert!(stdout.starts_with("\nADD-VIRTUAL-HOOK\n"));

    // @step And stdout contains the section header 'COMMON PATTERNS'
    assert!(stdout.contains("COMMON PATTERNS\n"));
}
