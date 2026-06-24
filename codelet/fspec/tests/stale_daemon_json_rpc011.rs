//! Stale daemon.json + autodiscovery hardening tests — RPC-011.
//!
//! Feature: spec/features/stale-daemon-json-autodiscovery.feature
//!
//! Covers:
//!   - read_and_verify_daemon_json deletes stale file when pid is dead
//!   - read_and_verify_daemon_json accepts a live pid
//!   - fspec client falls back gracefully on stale daemon.json
//!
//! Red phase: requires `common::read_and_verify_daemon_json` (exercised
//! observably via `fspec status` + `fspec client`), the new schema fields
//! pid/started_at/version, and the live-pid-verified preservation of the
//! file. Compile failure / behaviour failure IS the red signal.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

mod common;

use common::make_workspace;

/// Write a daemon.json file with the requested pid + port to the given path.
fn write_daemon_json(path: &std::path::Path, port: u16, pid: u32) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir daemon.json parent");
    }
    let body = serde_json::json!({
        "port": port,
        "pid": pid,
        "workspace": "/tmp",
        "started_at": "2026-05-11T00:00:00Z",
        "version": "0.0.0-test",
    });
    fs::write(path, serde_json::to_string_pretty(&body).unwrap()).expect("write daemon.json");
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: read_and_verify_daemon_json deletes stale file when pid is dead
//
// Observable via `fspec status` (which is the first caller of
// read_and_verify_daemon_json — `fspec client` is the second). When the
// pid is dead the helper deletes the file and returns Err with the stable
// text "no daemon.json found".
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test]
async fn read_and_verify_daemon_json_deletes_stale_file_when_pid_is_dead() {
    // @step Given a daemon.json on disk pointing at PID 99999 (guaranteed dead) on port 12345
    let xdg = tempfile::tempdir().expect("xdg tempdir");
    let djson = xdg.path().join("fspec").join("daemon.json");
    write_daemon_json(&djson, 12345, 99999);
    assert!(djson.is_file(), "precondition: daemon.json must exist");

    // @step When any caller invokes common::read_and_verify_daemon_json
    // Observable surrogate: invoke `fspec status` which routes its
    // resolve step through read_and_verify_daemon_json.
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec status");

    // @step Then it parses the JSON and extracts pid=99999
    // @step And it calls nix::sys::signal::kill(Pid::from_raw(99999), None) which returns Err(ESRCH)
    // @step And it deletes the file from disk
    assert!(
        !djson.exists(),
        "read_and_verify_daemon_json must delete the stale file"
    );

    // @step And it returns Err containing the stable text "no daemon.json found"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no daemon.json found"),
        "stderr must contain the stable text 'no daemon.json found'. Got: {stderr}"
    );
    assert!(
        stderr.contains("stale daemon.json removed"),
        "stderr must indicate the stale file was removed. Got: {stderr}"
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "fspec status must exit 1 when daemon.json is stale"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: read_and_verify_daemon_json accepts a live pid
//
// Observable via spawning a real daemon, then running `fspec status`
// which invokes read_and_verify_daemon_json. The file must NOT be
// deleted, and status must exit 0.
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_and_verify_daemon_json_accepts_a_live_pid() {
    // @step Given a daemon.json on disk pointing at the test process's own PID and an arbitrary port
    // Real-daemon variant: spawn fspec daemon and let it write the live
    // daemon.json itself. This is the closest observable to "kill(pid,
    // None) returns Ok(())" — the daemon's own pid IS the live pid.
    let (_dir, work_units) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = work_units.parent().unwrap().parent().unwrap();
    let xdg = tempfile::tempdir().expect("xdg tempdir");

    let mut child = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("daemon")
        .arg("--workspace")
        .arg(workspace)
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon");
    let mut reader = std::io::BufReader::new(child.stdout.take().expect("stdout"));
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");
    let _port: u16 = line.trim().parse().expect("port u16");
    let guard = common::ChildGuard(child);
    tokio::time::sleep(Duration::from_millis(200)).await;

    let djson = xdg.path().join("fspec").join("daemon.json");
    assert!(djson.is_file(), "live daemon.json must exist");

    // @step When common::read_and_verify_daemon_json runs
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec status");

    // @step Then kill(pid, None) returns Ok(())
    // @step And it returns Ok(DaemonHandshake { port, pid, started_at, version }) — file is NOT deleted
    assert!(
        djson.is_file(),
        "live daemon.json must NOT be deleted by verify step"
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "fspec status against live daemon must exit 0"
    );

    drop(guard);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: fspec client falls back gracefully on stale daemon.json
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fspec_client_falls_back_gracefully_on_stale_daemon_json() {
    // @step Given a stale daemon.json on disk (dead pid) and NO running daemon
    let xdg = tempfile::tempdir().expect("xdg tempdir");
    let djson = xdg.path().join("fspec").join("daemon.json");
    write_daemon_json(&djson, 12345, 99999);
    assert!(djson.is_file(), "precondition: stale daemon.json on disk");

    // @step When the user runs "fspec client" (no --connect flag)
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("client")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec client");

    // @step Then resolve_connect_url calls read_and_verify_daemon_json
    // @step And the stale file is deleted as part of the verify step
    assert!(
        !djson.exists(),
        "stale daemon.json must be deleted by fspec client"
    );

    // @step And the client prints to stderr a single line containing "no daemon.json found" AND "fspec daemon"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no daemon.json found"),
        "stderr must contain 'no daemon.json found'. Got: {stderr}"
    );
    assert!(
        stderr.contains("fspec daemon"),
        "stderr must mention 'fspec daemon'. Got: {stderr}"
    );

    // @step And the client exits with status 1
    assert_eq!(
        output.status.code(),
        Some(1),
        "fspec client must exit with status 1 on stale daemon.json"
    );
}
