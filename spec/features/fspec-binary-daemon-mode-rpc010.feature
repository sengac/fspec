@critical
@infrastructure
@RPC-010 @rpc @rust @tarpc @websocket
Feature: fspec daemon mode — headless WS server with signal handling

  """
  RPC-010 (parent RPC-002, depends on RPC-009). Daemon mode
  (`fspec daemon`) starts the shared service + WebSocket server ONLY —
  no terminal UI, suitable for systemd / launchd / dev-container
  background process supervision. Replaces the existing
  `codelet-rpc-server` binary for production purposes; the original
  binary stays as a development helper with verbatim port-line contract.

  Key extensions over RPC-005:
    • Configurable `--bind <addr>` clap arg (default 127.0.0.1:0).
    • REJECTS non-loopback bind addresses at clap-arg validation
      (preserves RPC-005 architecture rule [13] explicitly).
    • Blocks on BOTH `tokio::signal::ctrl_c()` AND SIGTERM
      (RPC-005 was ctrl_c-only).
    • Writes `daemon.json` for `fspec client` autodiscovery.
    • Optional `--pidfile <path>` for systemd-style PID tracking.

  Source artifacts:
    • codelet/fspec/src/daemon.rs (NEW)
    • codelet/fspec/src/common.rs (NEW)
    • codelet/rpc-server/src/server.rs::bind_and_serve (existing — RPC-005)
  """

  Background:
    Given the fspec binary has been built via `cargo build -p fspec --release`
    And a temp workspace exists with a seeded spec/work-units.json

  @smoke @end-to-end @integration-test
  Scenario: Daemon mode emits the port on STDOUT (RPC-005 contract verbatim)
    """
    The existing `codelet/rpc-server/tests/websocket_transport.rs::spawn_rpc_server`
    test harness reads exactly ONE line from stdout via BufReader::read_line
    and parses it as a bare integer port. `fspec daemon` MUST preserve
    that contract verbatim so the harness works unmodified.
    """
    When the developer spawns `fspec daemon --workspace <temp-workspace>` as a subprocess
    And reads exactly one line from the child's STDOUT
    Then that line parses as a bare integer in the range 1024..=65535
    And no other line is emitted on STDOUT before the daemon is shut down
    When the test connects a `WebSocketFspecBackend` to `ws://127.0.0.1:<that-port>`
    And calls `list_work_units().await`
    Then the call returns a non-empty Vec<WorkUnitInfo>

  @smoke
  Scenario: Daemon mode does not call ratatui::init
    Given the file `codelet/fspec/src/daemon.rs` exists
    Then it contains no occurrence of `ratatui::init`
    And it contains no occurrence of `crossterm::execute!`
    And it contains no construction of `TerminalGuard`
    And the daemon process never enters alt-screen or raw mode at runtime

  @smoke
  Scenario: Daemon mode keeps stderr fmt tracing subscriber (RPC-005 pattern)
    Given the file `codelet/fspec/src/daemon.rs` exists
    Then it calls `common::init_tracing_daemon()` exactly once
    And the `init_tracing_daemon()` body in `common.rs` registers a `tracing_subscriber::fmt` layer that writes to `std::io::stderr`
    And the same body also registers the LogEvent broadcast layer from `codelet_rpc::register_log_layer`

  @smoke @integration-test
  Scenario: --bind defaults to 127.0.0.1:0 when omitted
    When the developer spawns `fspec daemon` with no `--bind` flag
    And reads the port line from STDOUT
    And the test connects to `ws://127.0.0.1:<that-port>` from the test process
    Then the connection succeeds
    And the daemon's listening SocketAddr's IP equals `127.0.0.1`

  @smoke
  Scenario: --bind 127.0.0.1:8080 succeeds (custom loopback port)
    When the developer spawns `fspec daemon --bind 127.0.0.1:8080`
    Then the daemon starts and emits `8080` on STDOUT
    And the test can connect a WebSocketFspecBackend to `ws://127.0.0.1:8080`

  @smoke
  Scenario: --bind ::1:0 succeeds (IPv6 loopback)
    When the developer spawns `fspec daemon --bind '[::1]:0'`
    Then the daemon starts and the listening IP equals `::1`

  @smoke @error
  Scenario: --bind 0.0.0.0:8080 is REJECTED at clap-arg validation
    When the developer spawns `fspec daemon --bind 0.0.0.0:8080`
    Then the child process exits with a non-zero code BEFORE binding any socket
    And the child's STDERR contains the substring `error: --bind must be a loopback address`
    And the child's STDERR contains a reference to `auth/TLS for external binds is out of scope`
    And no WebSocket listener was ever opened on 0.0.0.0

  @smoke @error
  Scenario: --bind with any non-loopback host is REJECTED
    When the developer spawns `fspec daemon --bind 192.168.1.5:0`
    Then the child process exits with a non-zero code BEFORE binding any socket
    And the child's STDERR contains the substring `error: --bind must be a loopback address`

  @smoke @end-to-end @integration-test
  Scenario: Daemon handles SIGTERM cleanly (extension over RPC-005)
    """
    RPC-005's `codelet-rpc-server` binary handled `tokio::signal::ctrl_c()`
    only. `fspec daemon` MUST also handle SIGTERM so it can be supervised
    by systemd / launchd.
    """
    Given the developer has spawned `fspec daemon --workspace <temp-workspace>` as a subprocess
    And the daemon is listening on the captured ephemeral port
    When the test sends SIGTERM to the child
    Then the child exits with code 0 within 5 seconds
    And the child's daemon.json file is gone after exit

  @smoke @end-to-end @integration-test
  Scenario: Daemon handles ctrl_c (SIGINT) cleanly
    Given the developer has spawned `fspec daemon` as a subprocess
    When the test sends SIGINT to the child
    Then the child exits with code 0 within 5 seconds
    And the child's daemon.json file is gone after exit

  @smoke @end-to-end @integration-test
  Scenario: --pidfile <path> writes pid + port on bootstrap
    Given a tempfile path `<P>`
    When the developer spawns `fspec daemon --pidfile <P>` as a subprocess
    And waits for the daemon to be listening
    Then the file at `<P>` exists
    And the file's content is parseable so that the pid token equals the child's process pid
    And the file's content is parseable so that the port token equals the listening port
    When the test sends SIGTERM to the child
    And the child exits cleanly within 5 seconds
    Then the file at `<P>` no longer exists

  @smoke
  Scenario: --pidfile is daemon-only (combined mode does not accept it)
    """
    Locked design decision: pidfile semantics are tied to supervised
    process lifecycles (systemd / launchd) which only apply to daemon
    mode. Combined mode writes only daemon.json.
    """
    When the developer spawns `fspec --pidfile /tmp/test.pid` (combined mode)
    Then clap argument parsing fails with a non-zero exit code
    And the STDERR mentions that `--pidfile` is not a valid argument for the default subcommand

  @end-to-end
  Scenario: Daemon mode writes daemon.json so `fspec client` can autodiscover it
    Given the developer has set HOME to a tempdir BEFORE spawning the child
    When the developer spawns `fspec daemon` as a subprocess
    And waits for the daemon to be listening
    Then the file at `<HOME>/.fspec/daemon.json` exists
    And the JSON contains `port`, `pid`, `workspace`, and `version` keys
    And the `port` value equals the integer parsed from STDOUT
    When the test sends SIGTERM to the child
    Then the file at `<HOME>/.fspec/daemon.json` no longer exists after exit
