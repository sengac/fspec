@done
@workflow-automation
@websocket
@tarpc
@rust
@multiple
@rpc
@RPC-011
Feature: Multi-client broadcast capacity and tracing
  """
  Two-client integration: spawn daemon, attach TWO WebSocketFspecBackends, exercise create_session
  + send_input from client A, assert client B receives the SAME chunk stream in order. Broadcast
  channel capacities are explicit: chunks 1024, work-units 256, logs 4096. Tracing spans on
  per-connection handler tasks carry client_id (peer addr) and session_id so a `grep client_id=…`
  filters one client's traffic.
  """

  Background: User Story
    As a power-user driving the rust binary
    I want the daemon to fan out broadcasts identically to every client and tag log lines by client
    So that multi-client attach is reliable and debuggable

  Scenario: Two clients attached simultaneously see the same chunk stream
    Given a daemon listening on 127.0.0.1:0
    When the test opens two WebSocketFspecBackends (WS-A and WS-B) against that daemon
    And WS-A calls create_session(None) returning session_id S
    And WS-A calls send_input(S, "hi")
    Then both WS-A.chunks_rx() and WS-B.chunks_rx() yield the SAME sequence of (S, chunk) frames in the SAME order
    And ServerStats.connected_clients reads 2 throughout the test
    And no chunk is delivered to one client and not the other

  Scenario: Broadcast capacities are explicit and tuned
    Given codelet/rpc/src/lib.rs
    When inspecting the broadcast capacity constants
    Then DEFAULT_CHUNKS_CAPACITY equals 1024
    And DEFAULT_LOGS_CAPACITY equals 4096
    And DEFAULT_WORK_UNITS_CAPACITY equals 256
    And the constants are used as the third argument of broadcast::channel(...) in SharedFspecService::new and SharedFspecService::with_session_manager

  Scenario: Tracing spans carry client_id on per-connection handler tasks
    Given a daemon with two simultaneous clients on peer addrs 127.0.0.1:54321 and 127.0.0.1:54322
    When both clients call list_work_units once each
    Then the daemon's tracing output contains at least two records with field client_id=127.0.0.1:54321
    And at least two records with field client_id=127.0.0.1:54322
    And grepping the log for client_id=127.0.0.1:54321 yields ONLY records originating from that connection's task
