//! CLI surface for the `dependencies` subcommand on the standalone fspec
//! Rust binary — RPC-224.
//!
//! Feature: spec/features/dependencies-cli-subcommand.feature
//!
//! Red phase: until the clap subcommand + help intercept are wired (Phase C),
//! these tests fail — the binary rejects the unknown subcommand and the bridge
//! module does not yet exist. Once wired, the green-phase assertions hold.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

// ───────── helpers ─────────

fn run_deps(cwd: &Path, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = Command::new(fspec_bin());
    cmd.arg("dependencies");
    for a in extra_args {
        cmd.arg(a);
    }
    cmd.current_dir(cwd);
    let output = cmd.output().expect("spawn fspec dependencies");
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

/// Canonical work-units fixture: AUTH-001 blocks AUTH-002 and dependsOn SCHEMA-001.
fn work_units_auth_blocks_and_depends() -> String {
    r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "a", "status": "backlog", "relationships": { "blocks": ["AUTH-002"], "dependsOn": ["SCHEMA-001"] }, "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "b", "status": "backlog", "createdAt": "x", "updatedAt": "x" },
    "SCHEMA-001": { "id": "SCHEMA-001", "title": "s", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-001", "AUTH-002", "SCHEMA-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#
    .to_string()
}

// ───────── scenarios ─────────

#[test]
fn scenario_clap_exposes_dependencies_with_help() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled
    // (Enforced at compile time by CARGO_BIN_EXE_fspec in fspec_bin().)

    // @step When I run `./rust/target/release/fspec dependencies --help` from a shell
    let output = Command::new(fspec_bin())
        .arg("dependencies")
        .arg("--help")
        .output()
        .expect("spawn fspec dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec dependencies --help must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'dependencies'
    assert!(
        stdout.contains("dependencies") || stdout.contains("DEPENDENCIES"),
        "help must describe the dependencies subcommand; got:\n{stdout}"
    );
}

#[test]
fn scenario_cli_prints_header_and_relationship_lines_for_a_unit_with_dependencies() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with blocks=['AUTH-002'] and dependsOn=['SCHEMA-001']
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(ws.path(), &work_units_auth_blocks_and_depends());

    // @step When I run `./rust/target/release/fspec dependencies AUTH-001` from that workspace
    let (code, stdout, stderr) = run_deps(ws.path(), &["AUTH-001"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec dependencies AUTH-001 must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'Dependencies for AUTH-001:'
    assert!(
        stdout.contains("Dependencies for AUTH-001:"),
        "got:\n{stdout}"
    );

    // @step Then stdout contains the substring 'Blocks: AUTH-002'
    assert!(stdout.contains("Blocks: AUTH-002"), "got:\n{stdout}");

    // @step Then stdout contains the substring 'Depends on: SCHEMA-001'
    assert!(stdout.contains("Depends on: SCHEMA-001"), "got:\n{stdout}");
}

#[test]
fn scenario_cli_graph_prints_the_depth_first_blocks_tree() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with blocks=['AUTH-002'] and AUTH-002 with no relationships
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "a", "status": "backlog", "relationships": { "blocks": ["AUTH-002"] }, "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "b", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I run `./rust/target/release/fspec dependencies AUTH-001 --graph` from that workspace
    let (code, stdout, stderr) = run_deps(ws.path(), &["AUTH-001", "--graph"]);

    // @step Then the command exits 0
    assert_eq!(
        code, 0,
        "fspec dependencies AUTH-001 --graph must exit 0; got {code}, stderr={stderr}"
    );

    // @step Then stdout contains the substring 'AUTH-001'
    assert!(stdout.contains("AUTH-001"), "got:\n{stdout}");

    // @step Then stdout contains the substring 'blocks → AUTH-002'
    assert!(stdout.contains("blocks → AUTH-002"), "got:\n{stdout}");
}

#[test]
fn scenario_cli_exits_1_and_writes_to_stderr_when_the_work_unit_does_not_exist() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 only
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "a", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-001"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I run `./rust/target/release/fspec dependencies INVALID-999` from that workspace
    let (code, stdout, stderr) = run_deps(ws.path(), &["INVALID-999"]);

    // @step Then the command exits with code 1
    assert_eq!(
        code, 1,
        "fspec dependencies INVALID-999 must exit 1; got {code}, stdout={stdout}, stderr={stderr}"
    );

    // @step Then stderr contains the substring 'does not exist'
    assert!(
        stderr.contains("does not exist"),
        "stderr must contain 'does not exist'; got:\n{stderr}"
    );
}

#[test]
fn scenario_cli_delegates_to_same_fspec_core_function_as_dispatcher() {
    // @step Given a project root whose spec/work-units.json contains AUTH-001 with blocks=['AUTH-002'] and AUTH-002 with no relationships
    let ws = tempfile::tempdir().expect("tempdir");
    write_work_units(
        ws.path(),
        r#"{
  "version": "0.7.1",
  "workUnits": {
    "AUTH-001": { "id": "AUTH-001", "title": "a", "status": "backlog", "relationships": { "blocks": ["AUTH-002"] }, "createdAt": "x", "updatedAt": "x" },
    "AUTH-002": { "id": "AUTH-002", "title": "b", "status": "backlog", "createdAt": "x", "updatedAt": "x" }
  },
  "states": {
    "backlog": ["AUTH-001", "AUTH-002"], "specifying": [], "testing": [],
    "implementing": [], "validating": [], "done": [], "blocked": []
  }
}"#,
    );

    // @step When I dispatch dependencies through fspec_core::dispatch::dispatch_command with workUnitId='AUTH-001'
    let req = codelet_fspec_core::DispatchRequest {
        command: "dependencies".to_string(),
        args_json: r#"{"workUnitId":"AUTH-001"}"#.to_string(),
        project_root: ws.path().to_path_buf(),
    };
    let result = codelet_fspec_core::dispatch_command(req);
    assert!(
        result.success,
        "dispatcher path must succeed; got {result:?}"
    );

    // @step Then the DispatchResult.data equals the stdout produced by running `./rust/target/release/fspec dependencies AUTH-001`
    let (code, stdout, stderr) = run_deps(ws.path(), &["AUTH-001"]);
    assert_eq!(code, 0, "CLI must exit 0; stderr={stderr}");
    // Parity with TS `output.log(result)` → `console.log`, which ALWAYS
    // appends exactly one trailing newline. The dispatcher data (the default
    // text body) already ends with '\n', so the CLI stdout is the dispatcher
    // data plus one trailing newline (a trailing blank line).
    assert_eq!(
        stdout,
        format!("{}\n", result.data),
        "CLI stdout must equal dispatcher data + trailing newline; cli=\n{stdout}\ndispatcher=\n{}",
        result.data
    );

    // @step Then the CLI bridge module rust/fspec/src/dependencies.rs contains NO inline rendering, traversal, or filter logic — its only computation is JSON arg marshalling and stdout printing
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/dependencies.rs");
    assert!(
        bridge_path.exists(),
        "rust/fspec/src/dependencies.rs must exist as the CLI bridge module; got missing: {}",
        bridge_path.display()
    );
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Dependencies for",
        "Blocked by",
        "Depends on",
        "Related to",
        "blocks → ",
        "relationships",
        "traverse",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

const TS_HELP_FIXTURE_DEPS: &str = include_str!("fixtures/help/dependencies.txt");

#[test]
fn scenario_dependencies_help_matches_ts_reference() {
    // @step Given the fspec Rust binary at rust/target/release/fspec has been compiled

    // @step When I run `./rust/target/release/fspec dependencies --help` piped to non-TTY
    let output = Command::new(fspec_bin())
        .arg("dependencies")
        .arg("--help")
        .env_remove("CLICOLOR_FORCE")
        .env("NO_COLOR", "1")
        .output()
        .expect("spawn dependencies --help");
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // @step Then the command exits 0
    assert_eq!(code, 0, "dependencies --help must exit 0; stderr={stderr}");

    // @step Then stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/dependencies.txt
    assert_eq!(stdout, TS_HELP_FIXTURE_DEPS);

    // @step Then stdout starts with a blank line followed by 'DEPENDENCIES'
    assert!(stdout.starts_with("\nDEPENDENCIES\n"));
}
