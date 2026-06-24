//! CLI surface for the `remove-virtual-hook` subcommand on the standalone
//! fspec Rust binary — RPC-283.
//!
//! Feature: spec/features/remove-virtual-hook-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//! `codelet/fspec/src/main.rs` does not yet register a `remove-virtual-hook`
//! clap subcommand (clap returns exit code 2 for "unrecognized
//! subcommand"). Once the orchestrator wires the `Mode::RemoveVirtualHook`
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

fn run_remove_virtual_hook(cwd: &Path, args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("remove-virtual-hook");
    for a in args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec remove-virtual-hook");
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

fn eslint_hook() -> serde_json::Value {
    serde_json::json!({
        "name": "eslint",
        "event": "post-implementing",
        "command": "eslint .",
        "blocking": true
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes remove-virtual-hook as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_remove_virtual_hook_with_flag_aware_help() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("remove-virtual-hook")
        .arg("--help")
        .output()
        .expect("spawn fspec remove-virtual-hook --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec remove-virtual-hook --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains clap-generated help describing the remove-virtual-hook subcommand
    assert!(
        stdout.contains("remove-virtual-hook") || stdout.contains("virtual hook"),
        "help must describe the remove-virtual-hook subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the positional placeholder "<workUnitId>"
    assert!(
        stdout.contains("<workUnitId>"),
        "help must show <workUnitId> placeholder; got:\n{stdout}"
    );

    // @step And stdout contains the positional placeholder "<hookName>"
    assert!(
        stdout.contains("<hookName>"),
        "help must show <hookName> placeholder; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--blocking'
    // @step And stdout does NOT contain the substring '--git-context'
    // @step And stdout contains the substring 'No options available'
    // (remove-virtual-hook accepts no options; the OPTIONS section advertises "No options available".
    //  --blocking may legitimately appear in COMMON PATTERNS example commands referencing
    //  add-virtual-hook, so we instead assert the OPTIONS section is empty.)
    assert!(
        stdout.contains("No options available"),
        "remove-virtual-hook help must declare 'No options available'; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI removes an existing hook and prints the canonical success lines
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_removes_existing_hook_and_prints_canonical_success_lines() {
    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,blocking:true}]
    let ws = tempfile::tempdir().expect("tempdir");
    let hook = eslint_hook();
    write_work_units(
        ws.path(),
        &work_units_with_optional_hooks("AUTH-001", Some(&[hook])),
    );

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 eslint`
    let (code, stdout, stderr) = run_remove_virtual_hook(ws.path(), &["AUTH-001", "eslint"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; got {code}, stderr={stderr}");

    // @step And stdout contains the substring "✓ Removed virtual hook 'eslint' from AUTH-001"
    assert!(
        stdout.contains("✓ Removed virtual hook 'eslint' from AUTH-001"),
        "stdout must contain success line; got:\n{stdout}"
    );

    // @step And stdout contains the substring '  Remaining virtual hooks: 0'
    assert!(
        stdout.contains("  Remaining virtual hooks: 0"),
        "stdout must contain remaining count line; got:\n{stdout}"
    );

    // @step And the on-disk virtualHooks array for AUTH-001 has length 0
    let v = read_work_units(ws.path());
    let hooks = v["workUnits"]["AUTH-001"]["virtualHooks"]
        .as_array()
        .expect("virtualHooks array");
    assert!(
        hooks.is_empty(),
        "expected empty virtualHooks; got {hooks:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when the work unit has no virtual hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_no_virtual_hooks() {
    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_optional_hooks("AUTH-001", None));

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 eslint`
    let (code, _stdout, stderr) = run_remove_virtual_hook(ws.path(), &["AUTH-001", "eslint"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1; got {code}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to remove virtual hook:'
    assert!(
        stderr.contains("✗ Failed to remove virtual hook:"),
        "stderr must contain canonical error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring 'No virtual hooks configured for AUTH-001'
    assert!(
        stderr.contains("No virtual hooks configured for AUTH-001"),
        "stderr must mention canonical empty-hooks message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when the work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_work_unit_does_not_exist() {
    // @step Given spec/work-units.json contains AUTH-001
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_optional_hooks("AUTH-001", None));

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-999 eslint`
    let (code, _stdout, stderr) = run_remove_virtual_hook(ws.path(), &["AUTH-999", "eslint"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1; got {code}, stderr={stderr}");

    // @step And stderr contains the substring '✗ Failed to remove virtual hook:'
    assert!(
        stderr.contains("✗ Failed to remove virtual hook:"),
        "stderr must contain canonical error prefix; got:\n{stderr}"
    );

    // @step And stderr contains the substring "Work unit 'AUTH-999' does not exist"
    assert!(
        stderr.contains("Work unit 'AUTH-999' does not exist"),
        "stderr must mention missing work unit; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI fails with exit 1 when the named hook is not found
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_fails_with_exit_1_when_named_hook_not_found() {
    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,blocking:true}]
    let ws = tempfile::tempdir().expect("tempdir");
    let hook = eslint_hook();
    write_work_units(
        ws.path(),
        &work_units_with_optional_hooks("AUTH-001", Some(&[hook])),
    );

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 missing`
    let (code, _stdout, stderr) = run_remove_virtual_hook(ws.path(), &["AUTH-001", "missing"]);

    // @step Then the command exits 1
    assert_eq!(code, 1, "expected exit 1; got {code}, stderr={stderr}");

    // @step And stderr contains the substring "Virtual hook 'missing' not found in AUTH-001"
    assert!(
        stderr.contains("Virtual hook 'missing' not found in AUTH-001"),
        "stderr must mention missing hook; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI deletes the associated script file on removal
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_deletes_associated_script_file_on_removal() {
    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,command:'spec/hooks/.virtual/AUTH-001-eslint.sh',blocking:true,gitContext:true}]
    let ws = tempfile::tempdir().expect("tempdir");
    let hook = serde_json::json!({
        "name": "eslint",
        "event": "post-implementing",
        "command": "spec/hooks/.virtual/AUTH-001-eslint.sh",
        "blocking": true,
        "gitContext": true
    });
    write_work_units(
        ws.path(),
        &work_units_with_optional_hooks("AUTH-001", Some(&[hook])),
    );

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh exists on disk
    let script_dir = ws.path().join("spec/hooks/.virtual");
    fs::create_dir_all(&script_dir).expect("mkdir .virtual");
    let script_path = script_dir.join("AUTH-001-eslint.sh");
    fs::write(&script_path, "#!/bin/bash\necho hi\n").expect("write script");
    assert!(script_path.exists());

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook AUTH-001 eslint`
    let (code, _stdout, stderr) = run_remove_virtual_hook(ws.path(), &["AUTH-001", "eslint"]);

    // @step Then the command exits 0
    assert_eq!(code, 0, "expected exit 0; got {code}, stderr={stderr}");

    // @step And the file spec/hooks/.virtual/AUTH-001-eslint.sh no longer exists
    assert!(
        !script_path.exists(),
        "script must be removed; still present at {}",
        script_path.display()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'eslint',...,blocking:true}]
    let ws = tempfile::tempdir().expect("tempdir");
    let hook = eslint_hook();
    write_work_units(
        ws.path(),
        &work_units_with_optional_hooks("AUTH-001", Some(&[hook])),
    );

    // @step When I dispatch remove-virtual-hook through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' hookName='eslint'
    let req = codelet_fspec_core::DispatchRequest {
        command: "remove-virtual-hook".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","hookName":"eslint"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with remainingCount=0
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(
        dispatcher_data["remainingCount"].as_u64(),
        Some(0),
        "expected remainingCount=0; got {dispatcher_data:?}"
    );

    // @step And the CLI bridge module codelet/fspec/src/remove_virtual_hook.rs contains NO inline script-removal, retain, or work-unit-lookup logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/remove_virtual_hook.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/remove_virtual_hook.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    let stripped = common::strip_comments(&bridge_src);
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "virtualHooks",
        "remove_file",
        "retain",
        ".sh",
    ] {
        assert!(
            !stripped.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{stripped}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: remove-virtual-hook --help is byte-for-byte identical to TS reference
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_RVH: &str = include_str!("fixtures/help/remove-virtual-hook.txt");

#[test]
fn scenario_remove_virtual_hook_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec remove-virtual-hook --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("remove-virtual-hook")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn remove-virtual-hook --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "remove-virtual-hook --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/remove-virtual-hook.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_RVH);

    // @step And stdout starts with a blank line followed by 'REMOVE-VIRTUAL-HOOK'
    assert!(stdout.starts_with("\nREMOVE-VIRTUAL-HOOK\n"));

    // @step And stdout contains the section header 'COMMON PATTERNS'
    assert!(stdout.contains("COMMON PATTERNS\n"));
}
