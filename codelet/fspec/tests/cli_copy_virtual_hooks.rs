//! CLI surface for the `copy-virtual-hooks` subcommand on the standalone
//! fspec Rust binary — RPC-209.
//!
//! Feature: spec/features/copy-virtual-hooks-cli-subcommand.feature
//!
//! Red phase: these tests MUST fail today because
//! `codelet/fspec/src/main.rs` does not yet register a `copy-virtual-hooks`
//! clap subcommand (clap returns exit code 2 for "unrecognized
//! subcommand"). Once the orchestrator wires the `Mode::CopyVirtualHooks`
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

fn run_copy_virtual_hooks(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("copy-virtual-hooks");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec copy-virtual-hooks");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn lint_hook() -> serde_json::Value {
    serde_json::json!({
        "name": "lint",
        "event": "post-implementing",
        "command": "npm run lint",
        "blocking": true
    })
}
fn eslint_hook() -> serde_json::Value {
    serde_json::json!({
        "name": "eslint",
        "event": "pre-validating",
        "command": "eslint .",
        "blocking": true,
        "gitContext": true
    })
}

/// Write a work-units.json containing the listed (id, optional-hooks) pairs.
fn write_workspace(cwd: &Path, units: &[(&str, Option<Vec<serde_json::Value>>)]) {
    let spec = cwd.join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");

    let mut wus = serde_json::Map::new();
    let mut backlog = Vec::new();
    for (id, hooks) in units {
        let mut wu = serde_json::Map::new();
        wu.insert("id".into(), serde_json::json!(id));
        wu.insert(
            "title".into(),
            serde_json::json!(format!("title for {id}")),
        );
        wu.insert("status".into(), serde_json::json!("backlog"));
        wu.insert(
            "createdAt".into(),
            serde_json::json!("2026-06-01T00:00:00.000Z"),
        );
        wu.insert(
            "updatedAt".into(),
            serde_json::json!("2026-06-01T00:00:00.000Z"),
        );
        if let Some(h) = hooks {
            wu.insert("virtualHooks".into(), serde_json::Value::Array(h.clone()));
        }
        wus.insert((*id).to_string(), serde_json::Value::Object(wu));
        backlog.push(serde_json::Value::String((*id).to_string()));
    }
    let body = serde_json::to_string_pretty(&serde_json::json!({
        "version": "0.7.1",
        "workUnits": serde_json::Value::Object(wus),
        "states": {
            "backlog": serde_json::Value::Array(backlog),
            "specifying": [], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []
        }
    }))
    .unwrap();
    fs::write(spec.join("work-units.json"), body).expect("write work-units.json");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints success message when copying all hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_success_message_when_copying_all_hooks() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with two virtualHooks and AUTH-002 with no hooks
    let ws = tempfile::tempdir().expect("tempdir");
    write_workspace(
        ws.path(),
        &[
            ("AUTH-001", Some(vec![lint_hook(), eslint_hook()])),
            ("AUTH-002", None),
        ],
    );

    // @step When I run `./codelet/target/release/fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002` in that project root
    let (code, stdout, stderr) =
        run_copy_virtual_hooks(ws.path(), &["--from", "AUTH-001", "--to", "AUTH-002"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec copy-virtual-hooks must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring "✓ Copied 2 virtual hook(s) from AUTH-001 to AUTH-002"
    assert!(
        stdout.contains("✓ Copied 2 virtual hook(s) from AUTH-001 to AUTH-002"),
        "stdout must contain success message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI prints success message when copying a single named hook
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_prints_success_message_when_copying_named_hook() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with virtualHooks 'lint' and 'eslint', and AUTH-002 with no hooks
    let ws = tempfile::tempdir().expect("tempdir");
    write_workspace(
        ws.path(),
        &[
            ("AUTH-001", Some(vec![lint_hook(), eslint_hook()])),
            ("AUTH-002", None),
        ],
    );

    // @step When I run `./codelet/target/release/fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002 --hook-name eslint` in that project root
    let (code, stdout, stderr) = run_copy_virtual_hooks(
        ws.path(),
        &["--from", "AUTH-001", "--to", "AUTH-002", "--hook-name", "eslint"],
    );

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec copy-virtual-hooks --hook-name must exit 0; got {code}, stderr={stderr}"
    );

    // @step And stdout contains the substring "✓ Copied 1 virtual hook(s) from AUTH-001 to AUTH-002"
    assert!(
        stdout.contains("✓ Copied 1 virtual hook(s) from AUTH-001 to AUTH-002"),
        "stdout must contain single-hook success message; got:\n{stdout}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when --from is omitted
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_from_is_omitted() {
    // @step Given an empty project root with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec copy-virtual-hooks --to AUTH-002` in that project root
    let (code, stdout, stderr) =
        run_copy_virtual_hooks(ws.path(), &["--to", "AUTH-002"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec copy-virtual-hooks without --from must exit 1; got {code}, stdout={stdout}"
    );

    // @step And stderr contains the substring "--from option is required"
    assert!(
        stderr.contains("--from option is required"),
        "stderr must mention --from required; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when --to is omitted
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_to_is_omitted() {
    // @step Given an empty project root with no spec/ subdirectory
    let ws = tempfile::tempdir().expect("tempdir");
    assert!(!ws.path().join("spec").exists());

    // @step When I run `./codelet/target/release/fspec copy-virtual-hooks --from AUTH-001` in that project root
    let (code, stdout, stderr) =
        run_copy_virtual_hooks(ws.path(), &["--from", "AUTH-001"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec copy-virtual-hooks without --to must exit 1; got {code}, stdout={stdout}"
    );

    // @step And stderr contains the substring "--to option is required"
    assert!(
        stderr.contains("--to option is required"),
        "stderr must mention --to required; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI exits 1 when source has no hooks
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_exits_1_when_source_has_no_hooks() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with no virtualHooks and AUTH-002 with no virtualHooks
    let ws = tempfile::tempdir().expect("tempdir");
    write_workspace(
        ws.path(),
        &[("AUTH-001", None), ("AUTH-002", None)],
    );

    // @step When I run `./codelet/target/release/fspec copy-virtual-hooks --from AUTH-001 --to AUTH-002` in that project root
    let (code, stdout, stderr) =
        run_copy_virtual_hooks(ws.path(), &["--from", "AUTH-001", "--to", "AUTH-002"]);

    // @step Then the command exits 1
    assert_eq!(
        code, 1,
        "fspec copy-virtual-hooks must exit 1 when source has no hooks; got {code}, stdout={stdout}"
    );

    // @step And stderr contains the substring "No virtual hooks configured for source work unit AUTH-001"
    assert!(
        stderr.contains("No virtual hooks configured for source work unit AUTH-001"),
        "stderr must mention no-hooks-configured error; got:\n{stderr}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: CLI delegates to the same fspec_core function used by the dispatcher
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with one virtualHook 'lint' and AUTH-002 with no hooks
    let ws = tempfile::tempdir().expect("tempdir");
    write_workspace(
        ws.path(),
        &[
            ("AUTH-001", Some(vec![lint_hook()])),
            ("AUTH-002", None),
        ],
    );

    // @step When I dispatch copy-virtual-hooks through fspec_core::dispatch::dispatch_command with from='AUTH-001' and to='AUTH-002'
    let req = codelet_fspec_core::DispatchRequest {
        command: "copy-virtual-hooks".to_string(),
        args_json: r#"{"from":"AUTH-001","to":"AUTH-002"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the dispatcher's DispatchResult.data parses to a JSON object with copiedCount=1
    let parsed: serde_json::Value =
        serde_json::from_str(&result.data).expect("dispatcher data is JSON");
    assert_eq!(
        parsed["copiedCount"].as_u64(),
        Some(1),
        "copiedCount must be 1; got {}",
        result.data
    );

    // @step And the CLI bridge module codelet/fspec/src/copy_virtual_hooks.rs contains NO inline rendering, file IO beyond cwd resolution, or work-unit-lookup logic — its only computation is JSON arg marshalling plus the --from/--to presence guard
    let bridge_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/copy_virtual_hooks.rs");
    assert!(
        bridge_path.exists(),
        "codelet/fspec/src/copy_virtual_hooks.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "ensure_work_units_file",
        "write_json_atomic",
        "virtualHooks",
        "workUnits",
        "iso8601_now",
        "Copied ",
        "Source work unit ",
        "Target work unit ",
        "No virtual hooks configured",
        "Hook '",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: copy-virtual-hooks --help is byte-for-byte identical to TS formatCommandHelp reference output
// ─────────────────────────────────────────────────────────────────────────

const TS_HELP_FIXTURE_CPVH: &str = include_str!("fixtures/help/copy-virtual-hooks.txt");

#[test]
fn scenario_copy_virtual_hooks_help_matches_ts_formatcommandhelp_reference() {
    // @step Given the fspec Rust binary at codelet/target/release/fspec has been compiled

    // @step When I run `./codelet/target/release/fspec copy-virtual-hooks --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("copy-virtual-hooks")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn copy-virtual-hooks --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "copy-virtual-hooks --help must exit 0; stderr={stderr}");

    // @step And stdout is byte-for-byte identical to the fixture at codelet/fspec/tests/fixtures/help/copy-virtual-hooks.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_CPVH);

    // @step And stdout starts with a blank line followed by 'COPY-VIRTUAL-HOOKS'
    assert!(stdout.starts_with("\nCOPY-VIRTUAL-HOOKS\n"));

    // @step And stdout contains the section header 'COMMON PATTERNS'
    assert!(stdout.contains("COMMON PATTERNS\n"));
}
