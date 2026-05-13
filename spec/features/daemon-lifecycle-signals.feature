@done
@workflow-automation
@websocket
@tarpc
@rust
@resilience
@cli
@rpc
@RPC-011
Feature: Daemon lifecycle hardening (SIGTERM/SIGHUP/SIGINT + graceful drain + ConnectedClientGuard)

  """
  fspec daemon handles SIGTERM (drain + exit), SIGHUP (ArcSwap watcher rebuild, no exit), SIGINT
  (drain + exit). Drain protocol is Option B: stats.shutdown_signal Notify + each
  handle_connection sends WS Close{1001 going_away} then break + abort handle. ConnectedClientGuard
  RAII increments ServerStats.connected_clients on upgrade and decrements via Drop on ANY exit
  path. daemon.json gains pid + started_at + version fields and is removed on every clean shutdown
  path (SIGINT, SIGTERM, panic — via install_panic_hook from RPC-010).
  """

  Background: User Story
    As a fspec power-user driving the rust binary
    I want to have the fspec / fspec daemon / fspec client trio survive realistic interruptions (daemon restart, SIGTERM, multi-client attach, stale daemon.json) and tell me what's going on via fspec status + a health RPC
    So that I can day-drive the rust frontend without the rough edges flagged by RPC-010 review (CR-1 baseline + reconnect supervisor)

  Scenario: SIGTERM triggers graceful drain with going_away Close frame
    Given a fspec daemon serving two connected clients
    When an external observer sends SIGTERM to the daemon process
    Then codelet_rpc_server::request_shutdown fires ServerStats.shutdown_signal.notify_waiters
    And each handle_connection's tokio::select! takes the shutdown_signal arm
    And each per-connection task sends a WebSocket Close frame with code 1001 and reason "going_away" on its ws_sink
    And the daemon awaits the bind_and_serve JoinHandle for up to 500ms before aborting it
    And daemon.json is removed from disk AFTER the join.await completes (or aborts)
    And the daemon process exits with status 0

  Scenario: SIGHUP rebuilds the workspace watcher without exiting
    Given a fspec daemon with SharedFspecService.watcher = ArcSwap holding W_old
    When the daemon receives SIGHUP
    Then build_shutdown_future yields ShutdownReason::Sighup
    And the daemon logs "SIGHUP: re-reading workspace" at info level
    And it constructs a fresh WorkUnitsWatcher W_new against the same workspace path
    And service.watcher.store(Arc::new(W_new)) replaces the old watcher atomically
    And the daemon does NOT exit (its top-level signal-loop re-arms and continues)
    And subsequent list_work_units calls observe snapshots from W_new

  Scenario: SIGINT continues to trigger immediate shutdown
    Given a fspec daemon currently serving zero clients
    When SIGINT is delivered
    Then build_shutdown_future yields ShutdownReason::Sigint
    And the daemon executes the same drain protocol as SIGTERM (going_away Close + abort + remove daemon.json)
    And the process exits with status 0

  Scenario: ConnectedClientGuard increments and decrements on each connection
    Given a fspec daemon with ServerStats.connected_clients = 0
    When a client opens a WebSocket connection
    Then ServerStats.connected_clients reads 1 immediately after the upgrade succeeds
    When the client closes its connection (clean WS Close OR TCP RST)
    Then ServerStats.connected_clients reads 0 via the ConnectedClientGuard Drop impl
    And the counter is correct even if handle_connection returns Err mid-way

  Scenario: daemon.json schema upgrade carries pid + started_at + version
    Given the fspec daemon is bootstrapping
    When common::write_daemon_json runs at the autodiscovery path
    Then the JSON file contains all of: "port" (u16), "pid" (u32), "workspace" (absolute path), "started_at" (ISO 8601 string), "version" (CARGO_PKG_VERSION string)
    And the write is atomic (temp + rename)

  Scenario: daemon.json is removed on every clean shutdown path (SIGINT, SIGTERM)
    Given a fspec daemon with daemon.json on disk
    When the process receives SIGTERM
    Then daemon.json is removed from disk before exit
    Given a fresh fspec daemon with daemon.json on disk
    When the process receives SIGINT
    Then daemon.json is removed from disk before exit
