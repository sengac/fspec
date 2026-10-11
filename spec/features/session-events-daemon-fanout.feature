@done
@rust
@rpc
@session
@websocket
@connect
@multiple
@push
@RPC-433
Feature: Daemon-side session lifecycle event fan-out (created/removed) + connect-time snapshot over WebSocket

  """
  New SessionEvent { Created(SessionInfo), Removed(SessionId) } payload in codelet-rpc-types. SessionManager gains session_removed_tx (capacity 256, next to session_created_tx); destroy_session fires Removed only when shift_remove actually removed a session. SessionManager::live_session_infos() returns the in-memory map ONLY (no disk merge, no project filter) — the connect-time replay source, parity with the BUG-206 liveness gate. SessionManagerHandle gains session_removed_rx (default closed) + live_session_infos (default empty); StubSessionManagerHandle fires created/removed on create/destroy so multi-client fan-out tests can drive events. SharedFspecService::session_removed_rx + live_sessions_snapshot delegate to the handle (degenerate when no manager).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: New SessionEvent { Created(SessionInfo), Removed(SessionId) } payload type in codelet-rpc-types; new Envelope variants SessionEvent(SessionEvent) and SessionSnapshot(Vec<SessionInfo>) APPENDED after CmdRes so existing bincode variant indices stay stable
  #   2. R2: SessionManager gains session_removed_tx (capacity 256); destroy_session fires SessionEvent::Removed ONLY when the shift_remove actually removed a session (idempotent destroy of an unknown id emits nothing)
  #   3. R3: SessionManager::live_session_infos() returns the in-memory session map ONLY (no persisted-manifest merge, no project filter) — parity with the BUG-206 liveness gate; it is the connect-time replay source
  #   4. R4: WS server handle_connection spawns per-connection fan-out tasks for session_created_rx (Envelope::SessionEvent(Created)) and session_removed_rx (Envelope::SessionEvent(Removed)) mirroring status_changes_fanout, plus a lag_session_events stat; ServerStats exposes it
  #   5. R5: On connect, the server pushes Envelope::SessionSnapshot(live_sessions_snapshot()) right after the initial WorkUnitsUpdate — ALWAYS sent (even when empty) so a late-joining client deterministically re-broadcasts every open session
  #   6. R6: FspecWsClient gains session_events_rx() (broadcast template, sender owned by the pump task — drop-on-disconnect propagation per RPC-010 rule [23]) and session_snapshot_rx() (watch-backed forwarder replaying the cached snapshot, mirroring work_units_rx); ws_client_connect wires both into ClientInbound
  #   7. R7: SessionEvent / SessionSnapshot are server-to-client only: a client-pushed frame of either variant is counted in ServerStats.rejected_envelopes, named in rejected_variants, and traced (same treatment as StatusUpdate)
  #
  # EXAMPLES:
  #   1. I open the TUI on machine B while a session already runs on the daemon (created from machine A); when machine B connects it learns about that open session without me having to poll
  #   2. I close a session on machine A; the TUI on machine B is told about the removal over the wire (not by polling list_live_sessions)
  #   3. The client that issued the create_session call also receives the pushed Created event for its own session (no per-client filtering, like the RPC-007 chunk fan-out)
  #   4. I connect a TUI to a daemon that has no open sessions; the connection handshake still delivers an (empty) open-session snapshot so I can distinguish 'checked, none open' from 'never asked'
  #
  # ========================================

  Background: User Story
    As a operator
    I want to create or destroy a session on any machine connected to the fspec daemon
    So that every other connected client is told about the add/removal over the wire, and a connecting client is re-told about the sessions that are already open

  Scenario: Session created on any client is pushed to all connected clients
    Given a codelet-rpc-server is serving a SharedFspecService with an attached session manager
    And WebSocket clients A and B are connected and subscribed to the session-event stream
    When client A calls create_session
    Then both client A and client B observe a SessionEvent::Created carrying the new session's SessionInfo on their session-event stream


  Scenario: Session destroyed on any client is pushed to all connected clients
    Given a codelet-rpc-server is serving a SharedFspecService with an attached session manager
    And WebSocket clients A and B are connected, client A holds a live session, and both are subscribed to the session-event stream
    When client A calls destroy_session for that session
    Then both client A and client B observe a SessionEvent::Removed carrying the session id on their session-event stream


  # (Manager-level scenario "Destroying an unknown session id pushes no
  # removal event" lives in the sibling feature
  # session-events-daemon-removed-broadcast — one test file per feature,
  # ACDD 1:1 mapping.)

  Scenario: A connecting client is re-told about the sessions that are already open
    Given client A has created a session on a daemon while no other client is connected
    And the daemon's in-memory session map holds that session
    When client B connects to the daemon
    Then client B receives an open-session snapshot containing that session as part of the connection handshake, with no RPC poll from the client


  Scenario: A connecting client always receives an open-session snapshot even when no sessions are open
    Given a daemon whose session manager has no live sessions
    When a client connects to the daemon
    Then the client receives an EMPTY open-session snapshot during the connection handshake


  Scenario: Client-pushed session event frames are rejected as reserved
    Given a WebSocket client connected to the daemon
    When the client sends a raw Envelope::SessionEvent frame and a raw Envelope::SessionSnapshot frame
    Then the server counts both frames in rejected_envelopes, records the variant names SessionEvent and SessionSnapshot in rejected_variants, and the connection stays healthy

