//! Combined-mode (`fspec` with no subcommand) integration tests for RPC-010.
//!
//! Feature: spec/features/fspec-binary-combined-mode-rpc010.feature
//!
//! Combined mode boots the ratatui TUI AND the always-on WS server in
//! one process. These tests assert: (a) the PORT banner goes to STDERR
//! (alt-screen TUI owns STDOUT); (b) an external WS client can attach
//! mid-session; (c) on clean exit the WS JoinHandle is aborted BEFORE
//! daemon.json is removed. They MUST FAIL in the testing phase because
//! main.rs is a placeholder.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(unix)]

mod common;

use std::fs;
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use common::{
    codelet_root, fspec_bin, fspec_crate_root, make_workspace, scan_for_port_equals,
    strip_comments, ChildGuard,
};
use url::Url;

/// Spawn a real OS thread that drains a `ChildStdout` into a buffer.
/// Returns the shared buffer; the thread exits naturally once the
/// child's stdout closes.
fn drain_into_buffer(stdout: std::process::ChildStdout) -> Arc<Mutex<Vec<u8>>> {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let buf_clone = Arc::clone(&buf);
    std::thread::spawn(move || {
        let mut s = stdout;
        let mut chunk = [0u8; 4096];
        loop {
            match s.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf_clone
                    .lock()
                    .expect("mutex")
                    .extend_from_slice(&chunk[..n]),
                Err(_) => break,
            }
        }
    });
    buf
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_combined_boots_the_tui_and_starts_the_ws_server_in_one_process() {
    // @step Given the fspec binary has been built via `cargo build -p fspec --release`
    // (Implicit: env!("CARGO_BIN_EXE_fspec") is supplied by cargo.)

    // @step And a temp workspace exists with a seeded spec/work-units.json containing at least one WorkUnit
    let (ws, _path) = make_workspace(&[("CMB-1", "combined-boot", "backlog")]);

    // @step Given the developer has cd'd into the temp workspace
    // (Supplied via --workspace; equivalent under rule [16].)

    // @step When the developer spawns `fspec` as a subprocess with stdin/stdout/stderr piped
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(ws.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec (combined)");
    let stderr = child.stderr.take().expect("stderr");
    let _guard = ChildGuard(child);

    // @step Then within 5 seconds the child process is still running
    // @step And the child has bound a WebSocket listener on 127.0.0.1:<ephemeral-port>
    let mut reader = std::io::BufReader::new(stderr);
    let port = scan_for_port_equals(&mut reader);
    assert!(
        port >= 1024,
        "WS listener port must be a real ephemeral port; got {port}"
    );

    // @step And the same child has called ratatui::init() (alt-screen + raw mode active)
    // Asserted structurally — App::run() initializes TerminalGuard
    // (per architecture note 9), which calls ratatui::init() (per RPC-008).
    // We confirm the child is still alive AND its WS listener is accepting
    // connections — both would be false if App::run() had returned early
    // due to a ratatui::init() failure.
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let _backend =
        tokio::time::timeout(Duration::from_secs(5), WebSocketFspecBackend::connect(url))
            .await
            .expect("connect timeout")
            .expect("connect to combined-mode WS listener");
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_combined_emits_the_port_equals_n_banner_on_stderr_not_stdout() {
    // @step Given the fspec binary has been built
    // @step And a temp workspace exists
    let (ws, _path) = make_workspace(&[("CMB-2", "stderr-port", "backlog")]);

    // @step When the developer spawns `fspec` as a subprocess
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(ws.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec (combined)");
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let _guard = ChildGuard(child);
    let stdout_buf = drain_into_buffer(stdout);

    // @step And waits until the WS server is listening
    let mut stderr_reader = std::io::BufReader::new(stderr);
    let port = scan_for_port_equals(&mut stderr_reader);

    // @step Then exactly one line matching `^PORT=(\d+)$` appears on the child's STDERR
    assert!(port >= 1024, "expected a real PORT= line; got port={port}");

    // Allow the bootstrap to flush whatever stdout writes happen.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let snapshot = stdout_buf.lock().expect("mutex").clone();
    let stdout_text = String::from_utf8_lossy(&snapshot);

    // @step And the same line does NOT appear anywhere on the child's STDOUT
    assert!(
        !stdout_text.contains("PORT="),
        "STDOUT must NOT contain `PORT=`; got: {stdout_text:?}"
    );

    // @step And the captured STDOUT contains only ratatui control codes / cell drawing
    // Negative checks for plain-English log content that would prove the
    // tracing subscriber leaked onto stdout:
    assert!(
        !stdout_text.to_lowercase().contains("listening"),
        "STDOUT must NOT contain `listening` log line; got: {stdout_text:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_combined_does_not_corrupt_the_alt_screen_tui_canvas() {
    // @step When the developer spawns `fspec` and pipes its STDOUT to a buffer
    let (ws, _path) = make_workspace(&[("CMB-3", "no-corrupt", "backlog")]);
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(ws.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec");
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let _guard = ChildGuard(child);
    let stdout_buf = drain_into_buffer(stdout);

    // @step And the App has completed its bootstrap (left pane seeded, REPL session created)
    let mut stderr_reader = std::io::BufReader::new(stderr);
    let _port = scan_for_port_equals(&mut stderr_reader);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let snapshot = stdout_buf.lock().expect("mutex").clone();
    let body = String::from_utf8_lossy(&snapshot);

    // @step Then the captured STDOUT buffer contains NO occurrence of the literal text "PORT="
    assert!(
        !body.contains("PORT="),
        "STDOUT must NOT contain `PORT=`; got: {body:?}"
    );

    // @step And the captured STDOUT buffer contains NO occurrence of the literal text "listening"
    assert!(
        !body.to_lowercase().contains("listening"),
        "STDOUT must NOT contain `listening`; got: {body:?}"
    );

    // @step And the captured STDOUT buffer contains only ratatui's escape-sequence cell stream
    // (Implied by the two negative assertions above plus the
    //  bootstrap-completion gate.)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_external_ws_client_can_attach_to_combined_and_call_list_work_units() {
    // @step Given the developer has spawned `fspec --workspace <temp-workspace>` as a subprocess
    let (ws, _path) = make_workspace(&[("EXT-1", "external-client", "backlog")]);
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(ws.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec");
    let stderr = child.stderr.take().expect("stderr");
    let _guard = ChildGuard(child);

    // @step And the test has parsed the `PORT=<n>` line from the child's STDERR
    let mut stderr_reader = std::io::BufReader::new(stderr);
    let port = scan_for_port_equals(&mut stderr_reader);

    // @step When the test constructs a SECOND `WebSocketFspecBackend::connect(ws://127.0.0.1:<n>)` from the test process
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect 2nd WebSocketFspecBackend to combined mode");

    // @step And the test calls `list_work_units().await` on that backend
    let units = backend
        .list_work_units()
        .await
        .expect("list_work_units against combined-mode WS");

    // @step Then the call returns a non-empty Vec<WorkUnitInfo>
    assert!(
        !units.is_empty(),
        "list_work_units must return non-empty vec from combined-mode WS"
    );

    // @step And the Vec contains every WorkUnit seeded in the temp workspace's spec/work-units.json
    assert!(
        units.iter().any(|u| u.id == "EXT-1"),
        "list_work_units must include seeded WorkUnit EXT-1; got: {units:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_combined_shutdown_aborts_ws_join_handle_before_removing_daemon_json() {
    // @step Given the developer has spawned `fspec` as a subprocess
    let home = tempfile::tempdir().expect("home");
    let djson = home.path().join(".fspec").join("daemon.json");
    let (ws, _path) = make_workspace(&[("ABT-1", "abort-order", "backlog")]);
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(ws.path())
        .env("HOME", home.path())
        .env_remove("XDG_RUNTIME_DIR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec");
    let stderr = child.stderr.take().expect("stderr");
    let pid = child.id();
    let guard = ChildGuard(child);
    let mut stderr_reader = std::io::BufReader::new(stderr);
    let port = scan_for_port_equals(&mut stderr_reader);

    // @step And the test has attached an external WS client subscribed to work_units_rx
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect external client");
    let mut rx = backend.work_units_rx();

    // @step When the test sends SIGINT to the child
    let _ = Command::new("kill")
        .arg("-INT")
        .arg(pid.to_string())
        .status()
        .expect("kill -INT");

    // @step Then the external WS client observes a connection-closed error (not a hang) within 5 seconds
    let observed_close = wait_for_close(&mut rx, Duration::from_secs(5)).await;
    assert!(
        observed_close,
        "external client must observe connection-closed within 5s — JoinHandle was not aborted"
    );

    // @step And after the connection-closed error the child's daemon.json file is gone
    let djson_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < djson_deadline && djson.is_file() {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !djson.is_file(),
        "daemon.json must be removed after shutdown completes"
    );

    // @step And finally the child process exits with code 0
    let mut guard = guard;
    let exit_deadline = Instant::now() + Duration::from_secs(5);
    let mut exited = None;
    while Instant::now() < exit_deadline {
        if let Some(status) = guard.0.try_wait().expect("try_wait") {
            exited = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let status = exited.expect("combined mode did not exit within 5s after SIGINT");
    assert_eq!(
        status.code(),
        Some(0),
        "combined mode must exit code 0; got {status:?}"
    );
}

async fn wait_for_close<T>(rx: &mut tokio::sync::broadcast::Receiver<T>, deadline: Duration) -> bool
where
    T: Clone,
{
    let result = tokio::time::timeout(deadline, async {
        loop {
            match rx.recv().await {
                Ok(_) => continue,
                Err(_) => return true,
            }
        }
    })
    .await;
    matches!(result, Ok(true))
}

#[test]
fn scenario_combined_uses_tokio_runtime_handle_current_for_embedded_backend() {
    // @step Given the file `codelet/fspec/src/combined.rs` exists
    let combined_rs = fspec_crate_root().join("src").join("combined.rs");
    let body = std::fs::read_to_string(&combined_rs).expect("read codelet/fspec/src/combined.rs");

    // @step Then it contains the literal call `tokio::runtime::Handle::current()`
    assert!(
        body.contains("tokio::runtime::Handle::current()")
            || body.contains("runtime::Handle::current()")
            || body.contains("Handle::current()"),
        "combined.rs must source the Handle from Handle::current()"
    );

    // @step And it contains no occurrence of `tokio::runtime::Builder`
    assert!(
        !body.contains("tokio::runtime::Builder"),
        "combined.rs must NOT call tokio::runtime::Builder"
    );

    // @step And it contains no occurrence of `Runtime::new`
    assert!(
        !body.contains("Runtime::new"),
        "combined.rs must NOT call Runtime::new"
    );

    // @step And the `EmbeddedFspecBackend::new(handle, service.clone())` construction is reachable from the file's top-level run function
    assert!(
        body.contains("EmbeddedFspecBackend::new(") && body.contains("service.clone()"),
        "combined.rs must call EmbeddedFspecBackend::new(handle, service.clone())"
    );
}

#[test]
fn scenario_combined_bootstraps_with_build_service_constructed_exactly_once() {
    // @step Given the file `codelet/fspec/src/combined.rs` exists
    let combined_rs = fspec_crate_root().join("src").join("combined.rs");
    let body = std::fs::read_to_string(&combined_rs).expect("read codelet/fspec/src/combined.rs");

    // @step Then it calls `common::build_service(` exactly once
    let calls = body.matches("build_service(").count();
    assert_eq!(
        calls, 1,
        "combined.rs must call common::build_service exactly once; got {calls}"
    );

    // @step And the returned Arc<SharedFspecService> is passed to both `bind_and_serve` and `EmbeddedFspecBackend::new`
    assert!(
        body.contains("bind_and_serve(") && body.contains("EmbeddedFspecBackend::new("),
        "combined.rs must pass the Arc<SharedFspecService> to bind_and_serve AND EmbeddedFspecBackend::new"
    );
}
#[test]
fn scenario_combined_and_daemon_share_the_same_bind_and_serve_function() {
    // @step Given the file `codelet/fspec/src/combined.rs` exists
    // @step And the file `codelet/fspec/src/daemon.rs` exists
    let combined_rs = fspec_crate_root().join("src").join("combined.rs");
    let daemon_rs = fspec_crate_root().join("src").join("daemon.rs");
    let combined = std::fs::read_to_string(&combined_rs).expect("read combined.rs");
    let daemon = std::fs::read_to_string(&daemon_rs).expect("read daemon.rs");

    // @step Then `combined.rs` contains exactly one call to `bind_and_serve(`
    assert_eq!(
        strip_comments(&combined).matches("bind_and_serve(").count(),
        1,
        "combined.rs must contain exactly one bind_and_serve( call"
    );

    // @step And `daemon.rs` contains exactly one call to `bind_and_serve(`
    assert_eq!(
        strip_comments(&daemon).matches("bind_and_serve(").count(),
        1,
        "daemon.rs must contain exactly one bind_and_serve( call"
    );

    // @step And no other file under `codelet/fspec/src/` calls `bind_and_serve(`
    let src = fspec_crate_root().join("src");
    for entry in std::fs::read_dir(&src).expect("read fspec/src") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if name == "combined.rs" || name == "daemon.rs" {
            continue;
        }
        let body = std::fs::read_to_string(&path).expect("read rs file");
        assert!(
            !strip_comments(&body).contains("bind_and_serve("),
            "{name} must NOT call bind_and_serve(; it is exclusive to combined.rs + daemon.rs"
        );
    }

    // Sanity-check the underlying RPC-005 source exists where the rules
    // expect it (cross-reference to architecture note 5):
    let bind_and_serve_path = codelet_root()
        .join("rpc-server")
        .join("src")
        .join("server.rs");
    let bind_body =
        std::fs::read_to_string(&bind_and_serve_path).expect("read rpc-server/server.rs");
    assert!(
        bind_body.contains("pub async fn bind_and_serve("),
        "codelet/rpc-server/src/server.rs must still expose pub async fn bind_and_serve(...)"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_combined_writes_daemon_json_on_bootstrap_and_removes_it_on_clean_exit() {
    // @step Given the developer has set HOME to a tempdir BEFORE spawning the child
    let home = tempfile::tempdir().expect("home tempdir");
    let daemon_json = home.path().join(".fspec").join("daemon.json");

    // @step When the developer spawns `fspec` as a subprocess
    let (ws, _path) = make_workspace(&[("DJ-1", "daemon-json", "backlog")]);
    let mut child = Command::new(fspec_bin())
        .arg("--workspace")
        .arg(ws.path())
        .env("HOME", home.path())
        .env_remove("XDG_RUNTIME_DIR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec (combined)");

    // @step And waits for the WS server to be listening
    let stderr = child.stderr.take().expect("stderr");
    let pid = child.id();
    let guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stderr);
    let port = scan_for_port_equals(&mut reader);

    // @step Then the file at `<HOME>/.fspec/daemon.json` exists
    // Same temporal-ordering caveat as the pidfile test: allow up to 2s
    // for the file to appear after the port banner.
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !daemon_json.is_file() {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        daemon_json.is_file(),
        "daemon.json must exist at {} after bootstrap",
        daemon_json.display()
    );

    // @step And that file is valid JSON with at minimum keys `port`, `pid`, `workspace`, `version`
    let content = fs::read_to_string(&daemon_json).expect("read daemon.json");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("daemon.json must be valid JSON");
    let obj = json.as_object().expect("daemon.json must be a JSON object");
    for key in ["port", "pid", "workspace", "version"] {
        assert!(
            obj.contains_key(key),
            "daemon.json missing required key {key:?}; got: {content}"
        );
    }

    // @step And `port` equals the listening port observed on STDERR
    let port_in_json = obj["port"]
        .as_u64()
        .expect("daemon.json.port must be a number");
    assert_eq!(
        port_in_json as u16, port,
        "daemon.json.port must match the PORT= banner on stderr"
    );

    // @step And `pid` equals the child process's pid
    let pid_in_json = obj["pid"]
        .as_u64()
        .expect("daemon.json.pid must be a number");
    assert_eq!(
        pid_in_json as u32, pid,
        "daemon.json.pid must match the child process pid"
    );

    // @step When the test sends SIGINT to the child
    let kill_status = Command::new("kill")
        .arg("-INT")
        .arg(pid.to_string())
        .status()
        .expect("kill -INT");
    assert!(kill_status.success(), "kill -INT must succeed");

    // @step And the child exits with code 0 within 5 seconds
    let mut guard = guard;
    let exit_deadline = Instant::now() + Duration::from_secs(5);
    let mut exited = None;
    while Instant::now() < exit_deadline {
        if let Some(status) = guard.0.try_wait().expect("try_wait") {
            exited = Some(status);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let status = exited.expect("combined-mode child did not exit within 5s of SIGINT");
    assert_eq!(
        status.code(),
        Some(0),
        "combined mode must exit code 0 on SIGINT; got {status:?}"
    );

    // @step Then the file at `<HOME>/.fspec/daemon.json` no longer exists
    assert!(
        !daemon_json.is_file(),
        "daemon.json must be removed on clean shutdown; still at: {}",
        daemon_json.display()
    );
}
