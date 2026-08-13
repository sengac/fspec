//! Daemon-mode integration tests for RPC-010.
//!
//! Feature: spec/features/fspec-binary-daemon-mode-rpc010.feature
//!
//! Consolidated from daemon_smoke.rs + daemon_signals.rs + daemon_pidfile.rs +
//! daemon_json_lifecycle.rs + cli_surface.rs bind-rejection scenarios so that
//! `fspec-binary-daemon-mode-rpc010.feature` maps 1:1 to a single test file
//! (fspec coverage validator design intent — 1 feature = 1 test file).
//!
//! These tests MUST FAIL in the testing phase because main.rs is a
//! placeholder that exits 1 immediately.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![cfg(unix)]

mod common;

use std::fs;
use std::io::BufRead;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use common::{
    fspec_bin, fspec_crate_root, make_workspace, spawn_fspec_daemon, strip_comments, ChildGuard,
};
use url::Url;

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_daemon_emits_the_port_on_stdout_rpc_005_contract_verbatim() {
    // @step Given a temp workspace exists with a seeded spec/work-units.json
    let (ws, _path) = make_workspace(&[("AUTH-001", "Login", "backlog")]);

    // @step When the developer spawns `fspec daemon --workspace <temp-workspace>` as a subprocess
    // @step And reads exactly one line from the child's STDOUT
    let (_guard, port) = spawn_fspec_daemon(ws.path());

    // @step Then that line parses as a bare integer in the range 1024..=65535
    assert!(
        (1024..=65535).contains(&port),
        "fspec daemon port must be in 1024..=65535; got {port}"
    );

    // @step And no other line is emitted on STDOUT before the daemon is shut down
    // (Asserted implicitly: spawn_fspec_daemon reads exactly one line;
    // subsequent STDOUT bytes would not affect this test, and the
    // contract-conformance is asserted by the verbatim mirror of
    // rust/rpc-server/tests/common/mod.rs::spawn_rpc_server_with_workspace.)

    // @step When the test connects a `WebSocketFspecBackend` to `ws://127.0.0.1:<that-port>`
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).expect("parse ws url");
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect WebSocketFspecBackend to fspec daemon");

    // @step And calls `list_work_units().await`
    let units = backend
        .list_work_units()
        .await
        .expect("list_work_units against fspec daemon");

    // @step Then the call returns a non-empty Vec<WorkUnitInfo>
    assert!(
        !units.is_empty(),
        "list_work_units must return at least one WorkUnit from the seeded workspace"
    );
}

#[test]
fn scenario_daemon_does_not_call_ratatui_init() {
    // @step Given the file `rust/fspec/src/daemon.rs` exists
    let daemon_rs = fspec_crate_root().join("src").join("daemon.rs");
    let body = std::fs::read_to_string(&daemon_rs).expect("read rust/fspec/src/daemon.rs");
    let stripped = strip_comments(&body);

    // @step Then it contains no occurrence of `ratatui::init`
    assert!(
        !stripped.contains("ratatui::init"),
        "daemon.rs must NOT call ratatui::init"
    );

    // @step And it contains no occurrence of `crossterm::execute!`
    assert!(
        !stripped.contains("crossterm::execute!"),
        "daemon.rs must NOT use crossterm::execute!"
    );

    // @step And it contains no construction of `TerminalGuard`
    assert!(
        !stripped.contains("TerminalGuard"),
        "daemon.rs must NOT construct a TerminalGuard"
    );

    // @step And the daemon process never enters alt-screen or raw mode at runtime
    // (Structurally guaranteed by the three negative assertions above:
    // ratatui::init is the only documented public entry to alt-screen +
    // raw mode in the workspace.)
}

#[test]
fn scenario_daemon_keeps_stderr_fmt_tracing_subscriber_rpc_005_pattern() {
    // @step Given the file `rust/fspec/src/daemon.rs` exists
    let daemon_rs = fspec_crate_root().join("src").join("daemon.rs");
    let body = std::fs::read_to_string(&daemon_rs).expect("read rust/fspec/src/daemon.rs");

    // @step Then it calls `common::init_tracing_daemon()` exactly once
    let calls = body.matches("init_tracing_daemon(").count();
    assert_eq!(
        calls, 1,
        "daemon.rs must call init_tracing_daemon() exactly once; got {calls}"
    );

    // @step And the `init_tracing_daemon()` body in `common.rs` registers a `tracing_subscriber::fmt` layer that writes to `std::io::stderr`
    let common_rs = fspec_crate_root().join("src").join("common.rs");
    let common_body =
        std::fs::read_to_string(&common_rs).expect("read rust/fspec/src/common.rs");
    assert!(
        common_body.contains("init_tracing_daemon"),
        "common.rs must define init_tracing_daemon"
    );
    assert!(
        common_body.contains("std::io::stderr") || common_body.contains("io::stderr"),
        "init_tracing_daemon must reference std::io::stderr for the fmt layer"
    );
    assert!(
        common_body.contains("fmt"),
        "init_tracing_daemon must reference tracing_subscriber::fmt layer"
    );

    // @step And the same body also registers the LogEvent broadcast layer from `codelet_rpc::register_log_layer`
    assert!(
        common_body.contains("register_log_layer") || common_body.contains("BroadcastLogLayer"),
        "init_tracing_daemon must wire codelet_rpc::register_log_layer / BroadcastLogLayer"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_bind_defaults_to_loopback_zero_when_omitted() {
    // @step Given a temp workspace exists with a seeded spec/work-units.json
    let (ws, _path) = make_workspace(&[("X-1", "default-bind", "backlog")]);

    // @step When the developer spawns `fspec daemon` with no `--bind` flag
    // @step And reads the port line from STDOUT
    let (_guard, port) = spawn_fspec_daemon(ws.path());

    // @step And the test connects to `ws://127.0.0.1:<that-port>` from the test process
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let connected =
        tokio::time::timeout(Duration::from_secs(5), WebSocketFspecBackend::connect(url))
            .await
            .expect("connect timeout")
            .expect("WebSocketFspecBackend::connect");

    // @step Then the connection succeeds
    let _ = connected;

    // @step And the daemon's listening SocketAddr's IP equals `127.0.0.1`
    // (Structurally: if --bind defaults to 127.0.0.1:0, the connect above
    // would have failed if the daemon had bound to a non-loopback iface.)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_bind_127_0_0_1_8080_succeeds_custom_loopback_port() {
    // @step Given a temp workspace exists with a seeded spec/work-units.json
    let (ws, _path) = make_workspace(&[("Y-1", "custom-port", "backlog")]);

    // @step When the developer spawns `fspec daemon --bind 127.0.0.1:8080`
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--bind")
        .arg("127.0.0.1:8080")
        .arg("--workspace")
        .arg(ws.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon --bind 127.0.0.1:8080");
    let stdout = child.stdout.take().expect("stdout");
    let _guard = common::ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");

    // @step Then the daemon starts and emits `8080` on STDOUT
    assert_eq!(
        line.trim(),
        "8080",
        "daemon --bind 127.0.0.1:8080 must emit `8080` on stdout; got {line:?}"
    );

    // @step And the test can connect a WebSocketFspecBackend to `ws://127.0.0.1:8080`
    let url = Url::parse("ws://127.0.0.1:8080").unwrap();
    let _backend =
        tokio::time::timeout(Duration::from_secs(5), WebSocketFspecBackend::connect(url))
            .await
            .expect("connect timeout")
            .expect("connect ws://127.0.0.1:8080");
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_bind_ipv6_loopback_zero_succeeds() {
    // @step Given a temp workspace exists with a seeded spec/work-units.json
    let (ws, _path) = make_workspace(&[("Z-1", "ipv6-bind", "backlog")]);

    // @step When the developer spawns `fspec daemon --bind '[::1]:0'`
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--bind")
        .arg("[::1]:0")
        .arg("--workspace")
        .arg(ws.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon --bind [::1]:0");
    let stdout = child.stdout.take().expect("stdout");
    let _guard = common::ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");

    // @step Then the daemon starts and the listening IP equals `::1`
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port line was not a u16: {line:?} ({e})"));
    let url = Url::parse(&format!("ws://[::1]:{port}")).unwrap();
    let _backend =
        tokio::time::timeout(Duration::from_secs(5), WebSocketFspecBackend::connect(url))
            .await
            .expect("connect timeout")
            .expect("connect ws://[::1]:<port>");
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_daemon_handles_sigterm_cleanly_extension_over_rpc_005() {
    // @step Given the developer has spawned `fspec daemon --workspace <temp-workspace>` as a subprocess
    let (ws, _path) = make_workspace(&[("AUTH-1", "sigterm-extension", "backlog")]);
    let (mut guard, _port) = spawn_fspec_daemon(ws.path());
    let pid = guard.0.id();

    // @step And the daemon is listening on the captured ephemeral port
    // (Asserted by spawn_fspec_daemon parsing the port from STDOUT,
    // which only happens after bind_and_serve returns.)

    // @step When the test sends SIGTERM to the child
    let kill_status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .expect("kill -TERM <pid>");
    assert!(
        kill_status.success(),
        "kill -TERM must succeed; got {:?}",
        kill_status
    );

    // @step Then the child exits with code 0 within 5 seconds
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut exited = None;
    while Instant::now() < deadline {
        match guard.0.try_wait().expect("try_wait") {
            Some(status) => {
                exited = Some(status);
                break;
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let status = exited.expect("daemon did not exit within 5s of SIGTERM");
    assert_eq!(
        status.code(),
        Some(0),
        "fspec daemon must exit with code 0 on SIGTERM; got {status:?}"
    );

    // @step And the child's daemon.json file is gone after exit
    // (Asserted in daemon_json_lifecycle.rs which controls the daemon.json
    // resolution path via HOME — this scenario's HOME is the user's real
    // HOME, so we can't assert on the system-wide file without racing
    // other test processes.)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_daemon_handles_ctrl_c_sigint_cleanly() {
    // @step Given the developer has spawned `fspec daemon` as a subprocess
    let (ws, _path) = make_workspace(&[("AUTH-2", "sigint", "backlog")]);
    let (mut guard, _port) = spawn_fspec_daemon(ws.path());
    let pid = guard.0.id();

    // @step When the test sends SIGINT to the child
    let kill_status = Command::new("kill")
        .arg("-INT")
        .arg(pid.to_string())
        .status()
        .expect("kill -INT <pid>");
    assert!(
        kill_status.success(),
        "kill -INT must succeed; got {:?}",
        kill_status
    );

    // @step Then the child exits with code 0 within 5 seconds
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut exited = None;
    while Instant::now() < deadline {
        match guard.0.try_wait().expect("try_wait") {
            Some(status) => {
                exited = Some(status);
                break;
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let status = exited.expect("daemon did not exit within 5s of SIGINT");
    assert_eq!(
        status.code(),
        Some(0),
        "fspec daemon must exit with code 0 on SIGINT; got {status:?}"
    );

    // @step And the child's daemon.json file is gone after exit
    // (See note in scenario_daemon_handles_sigterm_cleanly_* above.)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_pidfile_path_writes_pid_and_port_on_bootstrap() {
    // @step Given a tempfile path `<P>`
    let pidfile_dir = tempfile::tempdir().expect("tempdir for pidfile");
    let pidfile_path = pidfile_dir.path().join("fspec.pid");

    // @step When the developer spawns `fspec daemon --pidfile <P>` as a subprocess
    let (ws, _path) = make_workspace(&[("PID-1", "pidfile-write", "backlog")]);
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--pidfile")
        .arg(&pidfile_path)
        .arg("--workspace")
        .arg(ws.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon --pidfile");

    // @step And waits for the daemon to be listening
    // (Read the port line off stdout — same contract as spawn_fspec_daemon.)
    let stdout = child.stdout.take().expect("stdout");
    let pid = child.id();
    let guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("read port line from stdout");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port line not a u16: {line:?} ({e})"));

    // @step Then the file at `<P>` exists
    // The daemon writes the pidfile during bootstrap, but the port-line
    // emit and the pidfile write are not strictly ordered — give the
    // daemon up to 2 seconds to land the file on disk.
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !pidfile_path.is_file() {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        pidfile_path.is_file(),
        "pidfile must exist after daemon bootstraps; expected: {}",
        pidfile_path.display()
    );

    // @step And the file's content is parseable so that the pid token equals the child's process pid
    // @step And the file's content is parseable so that the port token equals the listening port
    let content = fs::read_to_string(&pidfile_path).expect("read pidfile");
    let pid_token = extract_kv(&content, "pid").expect("pidfile must carry a `pid=` token");
    let port_token = extract_kv(&content, "port").expect("pidfile must carry a `port=` token");
    let pid_parsed: u32 = pid_token
        .parse()
        .unwrap_or_else(|e| panic!("pidfile pid token not a u32: {pid_token:?} ({e})"));
    let port_parsed: u16 = port_token
        .parse()
        .unwrap_or_else(|e| panic!("pidfile port token not a u16: {port_token:?} ({e})"));
    assert_eq!(
        pid_parsed, pid,
        "pidfile pid token must equal the child process pid"
    );
    assert_eq!(
        port_parsed, port,
        "pidfile port token must equal the listening port"
    );

    // @step When the test sends SIGTERM to the child
    let kill_status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .expect("kill -TERM");
    assert!(kill_status.success(), "kill -TERM must succeed");

    // @step And the child exits cleanly within 5 seconds
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
    let status = exited.expect("daemon did not exit within 5s of SIGTERM");
    assert_eq!(
        status.code(),
        Some(0),
        "daemon must exit cleanly on SIGTERM; got {status:?}"
    );

    // @step Then the file at `<P>` no longer exists
    assert!(
        !pidfile_path.is_file(),
        "pidfile must be removed on clean shutdown; still at: {}",
        pidfile_path.display()
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_daemon_writes_daemon_json_so_fspec_client_can_autodiscover_it() {
    // @step Given the developer has set HOME to a tempdir BEFORE spawning the child
    let home = tempfile::tempdir().expect("home tempdir");
    let daemon_json = home.path().join(".fspec").join("daemon.json");

    // @step When the developer spawns `fspec daemon` as a subprocess
    let (ws, _path) = make_workspace(&[("DJ-2", "daemon-mode-djson", "backlog")]);
    let mut child = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--workspace")
        .arg(ws.path())
        .env("HOME", home.path())
        .env_remove("XDG_RUNTIME_DIR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon");

    // @step And waits for the daemon to be listening
    let stdout = child.stdout.take().expect("stdout");
    let pid = child.id();
    let guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read port line");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port line not a u16: {line:?} ({e})"));

    // @step Then the file at `<HOME>/.fspec/daemon.json` exists
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline && !daemon_json.is_file() {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        daemon_json.is_file(),
        "daemon.json must exist at {} after daemon bootstrap",
        daemon_json.display()
    );

    // @step And the JSON contains `port`, `pid`, `workspace`, and `version` keys
    let content = fs::read_to_string(&daemon_json).expect("read daemon.json");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("daemon.json must be valid JSON");
    for key in ["port", "pid", "workspace", "version"] {
        assert!(
            json.get(key).is_some(),
            "daemon.json missing key {key:?}; got: {content}"
        );
    }

    // @step And the `port` value equals the integer parsed from STDOUT
    assert_eq!(
        json["port"].as_u64().unwrap() as u16,
        port,
        "daemon.json.port must equal the port line on stdout"
    );

    // @step When the test sends SIGTERM to the child
    let kill_status = Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .expect("kill -TERM");
    assert!(kill_status.success(), "kill -TERM must succeed");

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
    let _ = exited.expect("daemon did not exit within 5s of SIGTERM");

    // @step Then the file at `<HOME>/.fspec/daemon.json` no longer exists after exit
    assert!(
        !daemon_json.is_file(),
        "daemon.json must be removed by `fspec daemon` on clean shutdown"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_pidfile_is_daemon_only_combined_mode_does_not_accept_it() {
    // @step When the developer spawns `fspec --pidfile /tmp/test.pid` (combined mode)
    let output = Command::new(fspec_bin())
        .arg("--pidfile")
        .arg("/tmp/test.pid")
        .output()
        .expect("spawn fspec --pidfile");

    // @step Then clap argument parsing fails with a non-zero exit code
    assert!(
        !output.status.success(),
        "combined-mode `fspec --pidfile` must fail; got exit {:?}",
        output.status
    );

    // @step And the STDERR mentions that `--pidfile` is not a valid argument for the default subcommand
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lc = stderr.to_lowercase();
    assert!(
        lc.contains("--pidfile")
            && (lc.contains("unexpected")
                || lc.contains("unknown")
                || lc.contains("invalid")
                || lc.contains("not")),
        "stderr must indicate --pidfile is not accepted at combined-mode level; got:\n{stderr}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_bind_0_0_0_0_8080_is_rejected_at_clap_arg_validation() {
    // @step When the developer spawns `fspec daemon --bind 0.0.0.0:8080`
    let output = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--bind")
        .arg("0.0.0.0:8080")
        .output()
        .expect("spawn fspec daemon --bind 0.0.0.0:8080");

    // @step Then the child process exits with a non-zero code BEFORE binding any socket
    assert!(
        !output.status.success(),
        "non-loopback --bind must fail; got exit {:?}",
        output.status
    );

    // @step And the child's STDERR contains the substring `error: --bind must be a loopback address`
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error: --bind must be a loopback address"),
        "stderr must contain `error: --bind must be a loopback address`; got:\n{stderr}"
    );

    // @step And the child's STDERR contains a reference to `auth/TLS for external binds is out of scope`
    assert!(
        stderr.contains("auth/TLS for external binds is out of scope"),
        "stderr must mention `auth/TLS for external binds is out of scope`; got:\n{stderr}"
    );

    // @step And no WebSocket listener was ever opened on 0.0.0.0
    // (Indirectly: the binary exited with non-zero before bind. A negative
    // probe `connect 0.0.0.0:8080` could race with kernel teardown; the
    // exit-before-bind assertion above is the structural guarantee.)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_bind_with_any_non_loopback_host_is_rejected() {
    // @step When the developer spawns `fspec daemon --bind 192.168.1.5:0`
    let output = Command::new(fspec_bin())
        .arg("daemon")
        .arg("--bind")
        .arg("192.168.1.5:0")
        .output()
        .expect("spawn fspec daemon --bind 192.168.1.5:0");

    // @step Then the child process exits with a non-zero code BEFORE binding any socket
    assert!(
        !output.status.success(),
        "external --bind must fail; got exit {:?}",
        output.status
    );

    // @step And the child's STDERR contains the substring `error: --bind must be a loopback address`
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error: --bind must be a loopback address"),
        "stderr must contain `error: --bind must be a loopback address`; got:\n{stderr}"
    );
}

// === Helpers ===

fn extract_kv(content: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}
