//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
use super::*;

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_fspec_version_prints_the_workspace_version() {
    // @step When the developer runs `rust/target/release/fspec --version`
    let output = Command::new(fspec_bin())
        .arg("--version")
        .output()
        .expect("spawn fspec --version");

    // @step Then the command exits with code 0
    assert!(
        output.status.success(),
        "fspec --version must exit with code 0; got {:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And STDOUT contains the workspace version string declared in `rust/Cargo.toml [workspace.package].version`
    // The workspace package version is supplied to clap via the `version`
    // attribute, which `Cli::parse` populates from CARGO_PKG_VERSION.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = env!("CARGO_PKG_VERSION");
    assert!(
        stdout.contains(expected),
        "fspec --version stdout must contain {expected:?}; got {stdout:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_fspec_help_shows_three_subcommands() {
    // @step When the developer runs `rust/target/release/fspec --help`
    let output = Command::new(fspec_bin())
        .arg("--help")
        .output()
        .expect("spawn fspec --help");

    // @step Then the command exits with code 0
    assert!(
        output.status.success(),
        "fspec --help must exit with code 0; got {:?}",
        output.status
    );
    let help = String::from_utf8_lossy(&output.stdout);

    // @step And the help text mentions exactly the subcommands `daemon` and `client`
    assert!(
        help.contains("daemon"),
        "--help must mention `daemon` subcommand; got:\n{help}"
    );
    assert!(
        help.contains("client"),
        "--help must mention `client` subcommand; got:\n{help}"
    );

    // @step And the help text describes the no-subcommand default as combined mode
    let lc = help.to_lowercase();
    assert!(
        lc.contains("combined"),
        "--help must describe the no-subcommand default as combined mode; got:\n{help}"
    );

    // @step And the help text mentions the `--workspace` flag
    assert!(
        help.contains("--workspace"),
        "--help must mention `--workspace`; got:\n{help}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_workspace_defaults_to_cwd_when_omitted() {
    // @step Given a tempdir `<W>` containing a seeded spec/work-units.json
    let (w, _path) = make_workspace(&[("WS-CWD-1", "cwd-default", "backlog")]);

    // @step When the developer runs `cd <W> && fspec daemon` (no --workspace flag)
    // BUG-627: the daemon gets its own HOME and XDG_RUNTIME_DIR.
    let daemon_home = common::TestHome::new();
    let mut child = common::IsolatedCommand::fspec(&daemon_home)
        .arg("daemon")
        .current_dir(w.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon (cwd-rooted)");
    let stdout = child.stdout.take().expect("stdout");
    let _guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port not a u16: {line:?} ({e})"));

    // @step Then the daemon's WorkUnitsWatcher is rooted at `<W>`
    // (Indirectly asserted by the next step: only a watcher rooted at
    //  the CWD-passed workspace can return the seeded WorkUnitInfo.)
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect WebSocketFspecBackend");

    // @step And calls to `list_work_units` return the work units seeded in `<W>/spec/work-units.json`
    let units = backend.list_work_units().await.expect("list_work_units");
    assert!(
        units.iter().any(|u| u.id == "WS-CWD-1"),
        "list_work_units must return the WS-CWD-1 unit seeded into CWD; got: {units:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_workspace_path_overrides_cwd_for_the_work_units_watcher_root() {
    // @step Given a tempdir `<A>` containing seeded work-units (id "A-1")
    let (a, _path_a) = make_workspace(&[("A-1", "tempdir-A", "backlog")]);
    // @step And a different tempdir `<B>` containing seeded work-units (id "B-1")
    let (b, _path_b) = make_workspace(&[("B-1", "tempdir-B", "backlog")]);

    // @step When the developer runs `cd <A> && fspec --workspace <B> daemon`
    // (top-level `--workspace` MUST precede the `daemon` subcommand —
    // see common/mod.rs::spawn_fspec_daemon docs and main.rs::Cli::workspace
    // for the non-`global = true` rationale)
    // BUG-627: the daemon gets its own HOME and XDG_RUNTIME_DIR.
    let daemon_home = common::TestHome::new();
    let mut child = common::IsolatedCommand::fspec(&daemon_home)
        .arg("--workspace")
        .arg(b.path())
        .arg("daemon")
        .current_dir(a.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec daemon (workspace override)");
    let stdout = child.stdout.take().expect("stdout");
    let _guard = ChildGuard(child);
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    use std::io::BufRead;
    reader.read_line(&mut line).expect("read port line");
    let port: u16 = line
        .trim()
        .parse()
        .unwrap_or_else(|e| panic!("port not a u16: {line:?} ({e})"));

    // @step Then the daemon's WorkUnitsWatcher is rooted at `<B>`
    let url = Url::parse(&format!("ws://127.0.0.1:{port}")).unwrap();
    let backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect WebSocketFspecBackend");

    // @step And `list_work_units` returns the work units from `<B>` (NOT `<A>`)
    let units = backend.list_work_units().await.expect("list_work_units");
    assert!(
        units.iter().any(|u| u.id == "B-1"),
        "list_work_units must return B-1 (from --workspace override); got: {units:?}"
    );
    assert!(
        !units.iter().any(|u| u.id == "A-1"),
        "list_work_units must NOT return A-1 (CWD must be ignored); got: {units:?}"
    );
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scenario_both_combined_and_daemon_honour_the_same_workspace_resolution() {
    // @step Given a tempdir `<W>` containing seeded work-units
    let (w, _path) = make_workspace(&[("PARITY-1", "ws-parity", "backlog")]);

    // @step When the developer runs `fspec --workspace <W>` (combined)
    // BUG-627: the daemon gets its own HOME and XDG_RUNTIME_DIR.
    let daemon_home = common::TestHome::new();
    let mut combined_child = common::IsolatedCommand::fspec(&daemon_home)
        .arg("--workspace")
        .arg(w.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fspec (combined --workspace)");
    let stderr = combined_child.stderr.take().expect("stderr");
    let _combined_guard = ChildGuard(combined_child);
    let mut stderr_reader = std::io::BufReader::new(stderr);
    let combined_port = common::scan_for_port_equals(&mut stderr_reader);

    // @step Then the embedded backend's `list_work_units` returns the seeded units
    // External-WS view of the embedded backend's shared service:
    let url = Url::parse(&format!("ws://127.0.0.1:{combined_port}")).unwrap();
    let combined_backend = WebSocketFspecBackend::connect(url)
        .await
        .expect("connect to combined-mode WS");
    let combined_units = combined_backend
        .list_work_units()
        .await
        .expect("combined list_work_units");
    assert!(
        combined_units.iter().any(|u| u.id == "PARITY-1"),
        "combined --workspace must surface PARITY-1; got: {combined_units:?}"
    );

    // @step When the developer runs `fspec --workspace <W> daemon`
    let daemon_home = common::TestHome::new();
    let (_daemon_guard, daemon_port) = spawn_fspec_daemon(w.path(), &daemon_home);

    // @step Then the WebSocket-attached client's `list_work_units` returns the same seeded units
    let daemon_url = Url::parse(&format!("ws://127.0.0.1:{daemon_port}")).unwrap();
    let daemon_backend = WebSocketFspecBackend::connect(daemon_url)
        .await
        .expect("connect to daemon-mode WS");
    let daemon_units = daemon_backend
        .list_work_units()
        .await
        .expect("daemon list_work_units");
    assert!(
        daemon_units.iter().any(|u| u.id == "PARITY-1"),
        "daemon --workspace must surface the same PARITY-1; got: {daemon_units:?}"
    );
}
