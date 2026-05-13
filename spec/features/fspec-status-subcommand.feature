@done
@workflow-automation
@websocket
@tarpc
@rust
@cli
@rpc
@RPC-011
Feature: fspec status subcommand (one-shot health RPC against autodiscovered or --connect'd daemon)

  """
  New subcommand `fspec status` on the existing fspec binary's Cli enum (Mode::Status { connect:
  Option<String> }). Lives in codelet/fspec/src/status.rs (~80 lines). Mirrors client.rs's
  resolve+connect path but uses common::read_and_verify_daemon_json. Opens a one-shot
  WebSocketFspecBackend::connect (no supervisor), calls backend.health(), pretty-prints multi-line
  human-readable output, exits 0 on success / 1 on any failure (no daemon.json, stale, connect
  failure). --connect override bypasses daemon.json autodiscovery entirely.
  """

  Background: User Story
    As a fspec power-user driving the rust binary
    I want to have the fspec / fspec daemon / fspec client trio survive realistic interruptions (daemon restart, SIGTERM, multi-client attach, stale daemon.json) and tell me what's going on via fspec status + a health RPC
    So that I can day-drive the rust frontend without the rough edges flagged by RPC-010 review (CR-1 baseline + reconnect supervisor)

  Scenario: fspec status against a live daemon prints health and exits 0
    Given a fspec daemon has been running for 14 minutes 32 seconds with two connected clients
    And the watcher fired its last snapshot 3 seconds ago
    And all three broadcasts have lag counters at 0
    When the user runs "fspec status"
    Then status::run resolves the daemon via common::read_and_verify_daemon_json
    And it opens a one-shot WebSocketFspecBackend (no supervisor) and calls backend.health()
    And the HealthInfo received contains uptime_secs=872, connected_clients=2, last_watcher_event_secs_ago=Some(3), lag_chunks=0, lag_logs=0, lag_work_units=0
    And stdout contains the human-readable lines "fspec daemon: alive", "uptime: 14m 32s", "connected_clients: 2", "last_watcher_event: 3s ago", "broadcast_lag: chunks=0 logs=0 work_units=0"
    And the process exits with status 0

  Scenario: fspec status against no daemon prints diagnostic and exits 1
    Given no daemon.json exists at the autodiscovery path
    When the user runs "fspec status"
    Then stderr contains one line of the form "fspec daemon: not running (no daemon.json at <path>)"
    And the process exits with status 1
    And stdout is empty (no banner / no partial table)

  Scenario: fspec status against stale daemon.json deletes the file and exits 1
    Given a daemon.json on disk pointing at PID 99999 (dead)
    When the user runs "fspec status"
    Then the stale daemon.json is deleted as part of read_and_verify_daemon_json
    And stderr contains "fspec daemon: not running (stale daemon.json removed)"
    And the process exits with status 1

  Scenario: fspec status honours --connect override
    Given a fspec daemon running on ws://127.0.0.1:54321
    When the user runs "fspec status --connect ws://127.0.0.1:54321"
    Then daemon.json autodiscovery is bypassed (no read of daemon.json)
    And the same health() RPC and output sequence applies
    And the process exits with status 0
