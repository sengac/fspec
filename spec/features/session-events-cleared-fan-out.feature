@done
@rust
@rpc
@session
@websocket
@clear
@rpc-436
Feature: Session cleared event fan-out (SessionEvent::Cleared) over the RPC-433 wire stream
  """
  Wire-side half of RPC-436: completes the RPC-433 session lifecycle event
  coverage with the third lifecycle transition: clear. SessionEvent gains a
  Cleared(SessionId) variant (APPENDED after Removed so existing bincode
  variant indices stay stable). clear_history must fire
  Envelope::SessionEvent(SessionEvent::Cleared(id)) to EVERY connected
  client (no per-client filtering), fed by a per-connection
  session_cleared_fanout task that drains
  SharedFspecService::session_cleared_rx and shares the
  lag_session_events counter with the created/removed fanouts.

  Manager-level liveness gate (unknown id fires no event) lives in the
  sibling feature session-events-daemon-cleared-broadcast — one test file
  per feature, ACDD 1:1 mapping (mirrors the RPC-433 split between
  session-events-daemon-fanout and
  session-events-daemon-removed-broadcast).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: codelet-rpc-types SessionEvent gains a third variant Cleared(SessionId), APPENDED after Removed so existing bincode variant indices (Created=0, Removed=1) stay stable; the id is the only payload a connected client needs (mirrors Removed)
  #   2. R2: The WS server's handle_connection spawns a per-connection session_cleared_fanout task draining SharedFspecService::session_cleared_rx and wrapping each id in Envelope::SessionEvent(SessionEvent::Cleared(id)), fanning out to EVERY connected client (no per-client filtering) and sharing the lag_session_events counter with the created/removed fanouts — mirroring session_created_fanout / session_removed_fanout exactly
  #
  # EXAMPLES:
  #   1. I clear a session from machine A (TUI /clear); the TUI on machine B is told about the clear over the wire — no poll, no chunk-stream dependence for lifecycle tracking
  #   2. The client that issued clear_history also receives the pushed Cleared event for its own session (no per-client filtering, like the Created/Removed fan-out)
  #
  # SCOPE NOTE:
  #   TUI-side consumption of the new event (FspecBackend::session_cleared_rx,
  #   the WS-transport splitter, the App subscriber + Action) is a follow-up
  #   work unit (mirrors the RPC-433 wire-side / RPC-434 TUI-side split).
  #   The TUI's reactive UI reset for /clear continues to ride the existing
  #   StreamChunk::SessionStateChange { state: Cleared } chunks stream.
  #
  # ========================================
  Background: User Story
    As a operator connected to the fspec daemon
    I want to clear a session's history from any machine
    So that every connected client is told about the clear via the RPC-433 session-events wire stream, completing the session lifecycle event coverage (Created/Cleared/Removed)

  Scenario: Session cleared on any client is pushed to all connected clients
    Given a codelet-rpc-server is serving a SharedFspecService with an attached session manager
    And WebSocket clients A and B are connected, client A holds a live session, and both are subscribed to the session-event stream
    When client A calls clear_history for that session
    Then both client A and client B observe a SessionEvent::Cleared carrying the session id on their session-event stream
