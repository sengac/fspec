//! CLI surface for the `list-virtual-hooks` subcommand on the standalone
//! fspec Rust binary — RPC-252.
//!
//! Feature: spec/features/list-virtual-hooks-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//! `rust/fspec/src/main.rs` does not yet register a `list-virtual-hooks`
//! clap subcommand (clap returns exit code 2 for "unrecognized
//! subcommand"). Once the orchestrator wires the `Mode::ListVirtualHooks`
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

fn run_list_virtual_hooks(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("list-virtual-hooks");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec list-virtual-hooks");
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

/// Build a single-work-unit JSON document, optionally embedding a
/// `virtualHooks` array via raw `serde_json::json!`.
fn work_units_with_virtual_hooks(id: &str, hooks: Option<&[serde_json::Value]>) -> String {
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

fn lint_hook() -> serde_json::Value {
    serde_json::json!({
        "name": "lint",
        "event": "post-implementing",
        "command": "npm run lint",
        "blocking": true
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Clap exposes list-virtual-hooks as a subcommand and prints flag-aware --help
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_clap_exposes_list_virtual_hooks_with_flag_aware_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec list-virtual-hooks --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("list-virtual-hooks")
        .arg("--help")
        .output()
        .expect("spawn fspec list-virtual-hooks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-virtual-hooks --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains clap-generated help describing the list-virtual-hooks subcommand
    assert!(
        stdout.contains("list-virtual-hooks") || stdout.contains("virtual hooks"),
        "help must describe the list-virtual-hooks subcommand; got:\n{stdout}"
    );

    // @step And stdout contains the positional placeholder "<workUnitId>"
    assert!(
        stdout.contains("<workUnitId>"),
        "help must show the required positional placeholder '<workUnitId>'; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--format'
    assert!(
        !stdout.contains("--format"),
        "list-virtual-hooks --help must NOT advertise --format; got:\n{stdout}"
    );

    // @step And stdout does NOT contain the substring '--workspace'
    assert!(
        !stdout.contains("--workspace"),
        "list-virtual-hooks --help must NOT advertise the global --workspace flag; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the populated text layout when the work unit has virtual hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_populated_text_layout() {
    // @step Given spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'lint',event:'post-implementing',command:'npm run lint',blocking:true}]
    let ws = tempfile::tempdir().expect("tempdir");
    let hook = lint_hook();
    write_work_units(
        ws.path(),
        &work_units_with_virtual_hooks("AUTH-001", Some(&[hook])),
    );

    // @step When I run `./rust/target/release/fspec list-virtual-hooks AUTH-001`
    let (code, stdout, stderr) = run_list_virtual_hooks(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-virtual-hooks AUTH-001 must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring "Virtual Hooks for AUTH-001:"
    assert!(
        stdout.contains("Virtual Hooks for AUTH-001:"),
        "stdout must contain header 'Virtual Hooks for AUTH-001:'; got:\n{stdout}"
    );

    // @step And stdout contains the substring "post-implementing:"
    assert!(
        stdout.contains("post-implementing:"),
        "stdout must contain event header 'post-implementing:'; got:\n{stdout}"
    );

    // @step And stdout contains the substring "[blocking]"
    assert!(
        stdout.contains("[blocking]"),
        "stdout must contain '[blocking]' badge for the lint hook; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints the empty-hooks sentinel and exits 0 when the work unit has no virtual hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_empty_sentinel_and_exits_0() {
    // @step Given spec/work-units.json contains AUTH-001 with no virtualHooks field
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_with_virtual_hooks("AUTH-001", None));

    // @step When I run `./rust/target/release/fspec list-virtual-hooks AUTH-001`
    let (code, stdout, stderr) = run_list_virtual_hooks(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec list-virtual-hooks AUTH-001 must exit 0 on empty-hooks; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring "No virtual hooks configured for AUTH-001"
    assert!(
        stdout.contains("No virtual hooks configured for AUTH-001"),
        "stdout must contain the empty-hooks sentinel; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher (no duplicated business logic)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with virtualHooks=[{name:'lint',event:'post-implementing',command:'npm run lint',blocking:true}]
    let ws = tempfile::tempdir().expect("tempdir");
    let hook = lint_hook();
    write_work_units(
        ws.path(),
        &work_units_with_virtual_hooks("AUTH-001", Some(&[hook])),
    );

    // @step When I dispatch list-virtual-hooks through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001' and format='json'
    let req = codelet_fspec_core::DispatchRequest {
        command: "list-virtual-hooks".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001","format":"json"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );
    let dispatcher_data: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with hooks array of length 1
    let arr = dispatcher_data["hooks"]
        .as_array()
        .expect("hooks array on root");
    assert_eq!(arr.len(), 1, "hooks array length 1; got {arr:?}");

    // @step And the CLI bridge module rust/fspec/src/list_virtual_hooks.rs contains NO inline rendering, hook-grouping, or work-unit-lookup logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/list_virtual_hooks.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/list_virtual_hooks.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Virtual Hooks for",
        "No virtual hooks configured",
        "[blocking]",
        "[non-blocking]",
        "[git-context]",
        "hooksByEvent",
        "ensure_work_units_file",
        "IndexMap",
        "workUnits",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: list-virtual-hooks --help (RPC-252)
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_LVH: &str = include_str!("fixtures/help/list-virtual-hooks.txt");

#[test]
fn scenario_list_virtual_hooks_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec list-virtual-hooks --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("list-virtual-hooks")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn list-virtual-hooks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "list-virtual-hooks --help must exit 0; stderr={stderr}"
    );

    // @step And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/list-virtual-hooks.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_LVH);

    // @step And stdout starts with a blank line followed by 'LIST-VIRTUAL-HOOKS'
    assert!(stdout.starts_with("\nLIST-VIRTUAL-HOOKS\n"));

    // @step And stdout contains the section header 'COMMON PATTERNS'
    assert!(stdout.contains("COMMON PATTERNS\n"));
}
