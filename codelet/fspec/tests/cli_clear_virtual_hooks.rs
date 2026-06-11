//! CLI surface for the `clear-virtual-hooks` subcommand on the standalone
//! fspec Rust binary — RPC-205.
//!
//! Feature: spec/features/clear-virtual-hooks-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//! `codelet/fspec/src/main.rs` does not yet register a `clear-virtual-hooks`
//! clap subcommand (clap returns exit code 2 for "unrecognized
//! subcommand"). Once the orchestrator wires the `Mode::ClearVirtualHooks`
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

fn run_clear_virtual_hooks(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("clear-virtual-hooks");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec clear-virtual-hooks");
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

fn lint_hook() -> serde_json::Value {
    serde_json::json!({
        "name": "lint",
        "event": "post-implementing",
        "command": "npm run lint",
        "blocking": true
    })
}
fn test_hook() -> serde_json::Value {
    serde_json::json!({
        "name": "test",
        "event": "post-implementing",
        "command": "npm test",
        "blocking": false
    })
}

fn work_units_with_virtual_hooks(
    id: &str,
    hooks: Option<&[serde_json::Value]>,
) -> String {
    let mut wu = serde_json::Map::new();
    wu.insert("id".to_string(), serde_json::json!(id));
    wu.insert("title".to_string(), serde_json::json!(format!("title for {id}")));
    wu.insert("status".to_string(), serde_json::json!("backlog"));
    wu.insert("createdAt".to_string(), serde_json::json!("2026-06-01T00:00:00.000Z"));
    wu.insert("updatedAt".to_string(), serde_json::json!("2026-06-01T00:00:00.000Z"));
    if let Some(h) = hooks {
        wu.insert("virtualHooks".to_string(), serde_json::Value::Array(h.to_vec()));
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
// Scenario: CLI prints success message when clearing hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_success_message_when_clearing_hooks() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with two virtualHooks
    let ws = tempfile::tempdir().expect("tempdir");
    let hooks = vec![lint_hook(), test_hook()];
    write_work_units(
        ws.path(),
        &work_units_with_virtual_hooks("AUTH-001", Some(&hooks)),
    );

    // @step When I run `./codelet/target/release/fspec clear-virtual-hooks AUTH-001` in that project root
    let (code, stdout, stderr) = run_clear_virtual_hooks(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec clear-virtual-hooks AUTH-001 must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring "✓ Cleared 2 virtual hook(s) from AUTH-001"
    assert!(
        stdout.contains("✓ Cleared 2 virtual hook(s) from AUTH-001"),
        "stdout must contain success message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI succeeds with clearedCount=0 when the unit has no hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_succeeds_with_cleared_count_zero_when_unit_has_no_hooks() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks field
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with_virtual_hooks("AUTH-001", None),
    );

    // @step When I run `./codelet/target/release/fspec clear-virtual-hooks AUTH-001` in that project root
    let (code, stdout, stderr) = run_clear_virtual_hooks(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec clear-virtual-hooks AUTH-001 must exit 0 with no hooks; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring "✓ Cleared 0 virtual hook(s) from AUTH-001"
    assert!(
        stdout.contains("✓ Cleared 0 virtual hook(s) from AUTH-001"),
        "stdout must contain idempotent success message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 with chalk failure prefix when the work unit does not exist
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_work_unit_does_not_exist() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        &work_units_with_virtual_hooks("AUTH-001", None),
    );

    // @step When I run `./codelet/target/release/fspec clear-virtual-hooks AUTH-999` in that project root
    let (code, stdout, stderr) = run_clear_virtual_hooks(ws.path(), &["AUTH-999"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec clear-virtual-hooks AUTH-999 must exit 1 when missing; got {code}, stdout={stdout}"
    );

    // @step And stderr contains the substring "Work unit 'AUTH-999' does not exist"
    assert!(
        stderr.contains("Work unit 'AUTH-999' does not exist"),
        "stderr must mention canonical missing-work-unit message; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with one virtualHook 'lint'
    let ws = tempfile::tempdir().expect("tempdir");
    let hooks = vec![lint_hook()];
    write_work_units(
        ws.path(),
        &work_units_with_virtual_hooks("AUTH-001", Some(&hooks)),
    );

    // @step When I dispatch clear-virtual-hooks through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    let req = codelet_fspec_core::DispatchRequest {
        command: "clear-virtual-hooks".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with clearedCount=1
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(
        parsed["clearedCount"].as_u64(),
        Some(1),
        "clearedCount must be 1; got {}",
        result.data
    );

    // @step And the CLI bridge module codelet/fspec/src/clear_virtual_hooks.rs contains NO inline rendering, file IO, or work-unit-lookup logic — its only computation is JSON arg marshalling
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/clear_virtual_hooks.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/clear_virtual_hooks.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "virtualHooks",
        "workUnits",
        "remove_file",
        "iso8601_now",
        "Cleared ",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: clear-virtual-hooks --help is byte-for-byte identical to TS formatCommandHelp reference output
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_CVH: &str = include_str!("fixtures/help/clear-virtual-hooks.txt");

#[test]
fn scenario_clear_virtual_hooks_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec clear-virtual-hooks --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("clear-virtual-hooks")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn clear-virtual-hooks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "clear-virtual-hooks --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/clear-virtual-hooks.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CVH);

    // @step And stdout starts with a blank line followed by 'CLEAR-VIRTUAL-HOOKS'
    assert!(stdout.starts_with("\nCLEAR-VIRTUAL-HOOKS\n"));

    // @step And stdout contains the section header 'COMMON PATTERNS'
    assert!(stdout.contains("COMMON PATTERNS\n"));
}
