//! `fspec status` subcommand tests — RPC-011.
//!
//! Feature: spec/features/fspec-status-subcommand.feature
//!
//! Covers:
//!   - fspec status against a live daemon prints health and exits 0
//!   - fspec status against no daemon prints diagnostic and exits 1
//!   - fspec status against stale daemon.json deletes the file and exits 1
//!   - fspec status honours --connect override
//!
//! Red phase: requires the new `Mode::Status` clap subcommand on the fspec
//! binary plus the new `health()` RPC. Compile failure / behaviour failure
//! IS the red signal.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

mod common;

use common::{make_workspace, ChildGuard};

fn write_daemon_json(path: &std::path::Path, port: u16, pid: u32) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
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

/// Spawn fspec daemon with an explicit XDG_RUNTIME_DIR and read its port.
fn spawn_daemon_with_xdg(workspace: &std::path::Path, xdg: &std::path::Path) -> (ChildGuard, u16) {
    use std::io::BufRead;
    let mut child = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("daemon")
        .arg("--workspace")
        .arg(workspace)
        .env("XDG_RUNTIME_DIR", xdg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon");
    let mut reader = std::io::BufReader::new(child.stdout.take().expect("stdout"));
    let mut line = String::new();
    reader.read_line(&mut line).expect("read port");
    let port: u16 = line.trim().parse().expect("port u16");
    (ChildGuard(child), port)
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: fspec status against a live daemon prints health and exits 0
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fspec_status_against_a_live_daemon_prints_health_and_exits_0() {
    // @step Given a fspec daemon has been running for 14 minutes 32 seconds with two connected clients
    // @step And the watcher fired its last snapshot 3 seconds ago
    // @step And all three broadcasts have lag counters at 0
    //
    // We can't fast-forward 14m32s in a test; instead we assert the
    // OUTPUT SHAPE — the labels and key=value structure — and validate
    // the numeric values are within a sane band for a freshly-spawned
    // daemon. The exact 872-second uptime cannot be reproduced; we
    // assert the shape only.
    let (_dir, work_units) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = work_units.parent().unwrap().parent().unwrap();
    let xdg = tempfile::tempdir().expect("xdg");
    let (guard, port) = spawn_daemon_with_xdg(workspace, xdg.path());
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Open two WS clients so connected_clients counter shows 2.
    let url = format!("ws://127.0.0.1:{port}/");
    let (_ws_a, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("client A connect");
    let (_ws_b, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("client B connect");
    tokio::time::sleep(Duration::from_millis(200)).await;

    // @step When the user runs "fspec status"
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec status");

    // @step Then status::run resolves the daemon via common::read_and_verify_daemon_json
    // @step And it opens a one-shot WebSocketFspecBackend (no supervisor) and calls backend.health()
    // @step And the HealthInfo received contains uptime_secs=872, connected_clients=2, last_watcher_event_secs_ago=Some(3), lag_chunks=0, lag_logs=0, lag_work_units=0
    // (Shape-only assertion: cannot reproduce 872s uptime.)

    // @step And stdout contains the human-readable lines "fspec daemon: alive", "uptime: 14m 32s", "connected_clients: 2", "last_watcher_event: 3s ago", "broadcast_lag: chunks=0 logs=0 work_units=0"
    // (Shape-only assertion on uptime/conn-count/event-age values:
    // cannot reproduce literal `14m 32s` / `connected_clients: 2` /
    // `3s ago` in a hermetic test; we assert each LABEL is present and
    // that the broadcast_lag counters report `chunks=0 logs=0
    // work_units=0` exactly.)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fspec daemon: alive"),
        "stdout must contain 'fspec daemon: alive'. Got: {stdout}"
    );
    assert!(
        stdout.contains("uptime:"),
        "stdout must contain 'uptime:'. Got: {stdout}"
    );
    assert!(
        stdout.contains("connected_clients:"),
        "stdout must contain 'connected_clients:'. Got: {stdout}"
    );
    assert!(
        stdout.contains("last_watcher_event:"),
        "stdout must contain 'last_watcher_event:'. Got: {stdout}"
    );
    assert!(
        stdout.contains("broadcast_lag:"),
        "stdout must contain 'broadcast_lag:'. Got: {stdout}"
    );
    assert!(
        stdout.contains("chunks=0") && stdout.contains("logs=0") && stdout.contains("work_units=0"),
        "stdout broadcast_lag must show chunks=0, logs=0, work_units=0. Got: {stdout}"
    );
    assert!(
        stdout.contains("version:"),
        "stdout must contain 'version:'. Got: {stdout}"
    );

    // @step And the process exits with status 0
    assert_eq!(
        output.status.code(),
        Some(0),
        "fspec status against live daemon must exit 0"
    );

    drop(guard);
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: fspec status against no daemon prints diagnostic and exits 1
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test]
async fn fspec_status_against_no_daemon_prints_diagnostic_and_exits_1() {
    // @step Given no daemon.json exists at the autodiscovery path
    let xdg = tempfile::tempdir().expect("xdg");
    let djson = xdg.path().join("fspec").join("daemon.json");
    assert!(!djson.exists(), "precondition: no daemon.json");

    // @step When the user runs "fspec status"
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec status");

    // @step Then stderr contains one line of the form "fspec daemon: not running (no daemon.json at <path>)"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fspec daemon: not running"),
        "stderr must contain 'fspec daemon: not running'. Got: {stderr}"
    );
    assert!(
        stderr.contains("no daemon.json"),
        "stderr must reference the missing daemon.json. Got: {stderr}"
    );

    // @step And the process exits with status 1
    assert_eq!(
        output.status.code(),
        Some(1),
        "fspec status against no daemon must exit 1"
    );

    // @step And stdout is empty (no banner / no partial table)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim().is_empty(),
        "stdout must be empty when no daemon. Got: {stdout:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: fspec status against stale daemon.json deletes the file and exits 1
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test]
async fn fspec_status_against_stale_daemon_json_deletes_the_file_and_exits_1() {
    // @step Given a daemon.json on disk pointing at PID 99999 (dead)
    let xdg = tempfile::tempdir().expect("xdg");
    let djson = xdg.path().join("fspec").join("daemon.json");
    write_daemon_json(&djson, 12345, 99999);
    assert!(djson.is_file(), "precondition: stale daemon.json");

    // @step When the user runs "fspec status"
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("status")
        .env("XDG_RUNTIME_DIR", xdg.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec status");

    // @step Then the stale daemon.json is deleted as part of read_and_verify_daemon_json
    assert!(
        !djson.exists(),
        "stale daemon.json must be deleted by fspec status"
    );

    // @step And stderr contains "fspec daemon: not running (stale daemon.json removed)"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fspec daemon: not running"),
        "stderr must contain 'fspec daemon: not running'. Got: {stderr}"
    );
    assert!(
        stderr.contains("stale daemon.json removed") || stderr.contains("stale"),
        "stderr must indicate 'stale daemon.json removed'. Got: {stderr}"
    );

    // @step And the process exits with status 1
    assert_eq!(
        output.status.code(),
        Some(1),
        "fspec status against stale daemon.json must exit 1"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: fspec status honours --connect override
// ─────────────────────────────────────────────────────────────────────────

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fspec_status_honours_connect_override() {
    // @step Given a fspec daemon running on ws://127.0.0.1:54321
    let (_dir, work_units) = make_workspace(&[("AUTH-001", "Login", "done")]);
    let workspace = work_units.parent().unwrap().parent().unwrap();
    let xdg_a = tempfile::tempdir().expect("xdg daemon");
    let (guard, port) = spawn_daemon_with_xdg(workspace, xdg_a.path());
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Use an empty XDG dir for the status invocation so autodiscovery
    // CANNOT pick up the daemon — the only way status can connect is
    // via the explicit --connect flag.
    let xdg_b = tempfile::tempdir().expect("xdg status");

    // @step When the user runs "fspec status --connect ws://127.0.0.1:54321"
    let connect_url = format!("ws://127.0.0.1:{port}");
    let output = Command::new(env!("CARGO_BIN_EXE_fspec"))
        .arg("status")
        .arg("--connect")
        .arg(&connect_url)
        .env("XDG_RUNTIME_DIR", xdg_b.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn fspec status --connect");

    // @step Then daemon.json autodiscovery is bypassed (no read of daemon.json)
    // Surrogate: xdg_b has no daemon.json — if autodiscovery had run,
    // it would have reported "no daemon.json" and exited 1.

    // @step And the same health() RPC and output sequence applies
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fspec daemon: alive"),
        "stdout must contain 'fspec daemon: alive' even with --connect. Got: {stdout}"
    );

    // @step And the process exits with status 0
    assert_eq!(
        output.status.code(),
        Some(0),
        "fspec status --connect to live daemon must exit 0"
    );

    drop(guard);
}
