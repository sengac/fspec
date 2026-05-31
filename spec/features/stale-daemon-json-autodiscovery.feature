@done
@workflow-automation
@rust
@resilience
@cli
@rpc
@RPC-011
Feature: Stale daemon.json autodiscovery hardening (verify_daemon_alive + stale-delete)
  """
  Clients (fspec client AND fspec status) call common::read_and_verify_daemon_json BEFORE trusting
  the URL. The function reads daemon.json, parses pid, and uses nix::sys::signal::kill(pid, None)
  on unix / GetExitCodeProcess on windows to verify the daemon is alive. On ESRCH/INVALID it
  deletes the stale file and returns Err with stable text "no daemon.json found …" so callers can
  match. Lives in codelet/fspec/src/common.rs alongside daemon_json_path() and
  read_daemon_json_port(). Reused by client.rs and status.rs.
  """

  Background: User Story
    As a fspec power-user driving the rust binary
    I want to have the fspec / fspec daemon / fspec client trio survive realistic interruptions (daemon restart, SIGTERM, multi-client attach, stale daemon.json) and tell me what's going on via fspec status + a health RPC
    So that I can day-drive the rust frontend without the rough edges flagged by RPC-010 review (CR-1 baseline + reconnect supervisor)

  Scenario: read_and_verify_daemon_json deletes stale file when pid is dead
    Given a daemon.json on disk pointing at PID 99999 (guaranteed dead) on port 12345
    When any caller invokes common::read_and_verify_daemon_json
    Then it parses the JSON and extracts pid=99999
    And it calls nix::sys::signal::kill(Pid::from_raw(99999), None) which returns Err(ESRCH)
    And it deletes the file from disk
    And it returns Err containing the stable text "no daemon.json found"

  Scenario: read_and_verify_daemon_json accepts a live pid
    Given a daemon.json on disk pointing at the test process's own PID and an arbitrary port
    When common::read_and_verify_daemon_json runs
    Then kill(pid, None) returns Ok(())
    And it returns Ok(DaemonHandshake { port, pid, started_at, version }) — file is NOT deleted

  Scenario: fspec client falls back gracefully on stale daemon.json
    Given a stale daemon.json on disk (dead pid) and NO running daemon
    When the user runs "fspec client" (no --connect flag)
    Then resolve_connect_url calls read_and_verify_daemon_json
    And the stale file is deleted as part of the verify step
    And the client prints to stderr a single line containing "no daemon.json found" AND "fspec daemon"
    And the client exits with status 1
