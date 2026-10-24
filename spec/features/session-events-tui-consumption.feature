@done
@rust
@tui
@session
@multi-session
@parity
@reconnect
@RPC-434
Feature: TUI consumes session lifecycle events on both transports (embedded + WS): created/removed fan-in + snapshot replay
  """
  FspecBackend: session_removed_rx() defaults to a closed receiver (RPC-037/PROV-109 convention); async session_snapshot() defaults to empty Vec. EmbeddedFspecBackend delegates session_removed_rx to the handle and session_snapshot to SharedFspecService::live_sessions_snapshot (in-memory only). WebSocketFspecBackend: session_created_rx (replacing the never-firing stub) + session_removed_rx are both fed by a per-subscriber splitter task over FspecWsClient::session_events_rx (RPC-433); session_snapshot reads the fresh client's snapshot frame via the client slot (read guard; disconnected => empty). Action::SessionRemoved handled by idempotent App::handle_session_removed (new dispatch_session_lifecycle.rs, <=300 lines): remove_session_if_open, mux_sync_window, board detach, clear current WU, focus fallback. bootstrap: 6th subscriber (session_removed) + replay_session_snapshot() helper (bootstrap AND reconnect) that folds each snapshot SessionInfo into Action::SessionCreated before spawn_session_restore.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: FspecBackend gains session_removed_rx() (default: closed receiver — existing convention) and async session_snapshot() (default: empty Vec); EmbeddedFspecBackend delegates both to the SharedFspecService/handle; WebSocketFspecBackend replaces its never-firing session_created_rx stub with a real receiver fed by the RPC-433 wire stream
  #   2. R2: The WebSocket backend splits the single RPC-433 session-event wire stream (SessionEvent::Created / Removed) onto separate session_created_rx and session_removed_rx broadcast receivers, preserving per-variant ordering; when the client slot is None (disconnected) both return a closed receiver so the bootstrap subscriber exits on RecvError::Closed and is respawned on Reconnected
  #   3. R3: New Action::SessionRemoved(SessionId); App::handle_session_removed is IDEMPOTENT (not-open tab => no-op, so the local close path + the remote event never double-remove) and mirrors the dispatch_agent_exit cleanup: remove_session_if_open, mux_sync_window, detach the session's board work-unit binding, and clear the current work unit when it pointed at the removed session; focus falls back to a surviving session or the board
  #   4. R4: Bootstrap seeds the tab list from backend.session_snapshot() by replaying each SessionInfo through the EXISTING Action::SessionCreated handler (idempotent append), synchronously BEFORE spawn_session_restore so the BUG-206 auto-restore re-asserts the correct focus; a shared App::replay_session_snapshot() helper is used by both bootstrap and the reconnect path
  #   5. R5: Bootstrap spawns the 8th subscriber task (session_removed_rx -> Action::SessionRemoved; the count this rule tracked was bumped 7 -> 8 by RPC-434 and 8 -> 9 by RPC-437) using the same Lagged-debug/Closed-break loop shape as the session-created subscriber; the Reconnected handler's abort+respawn of ALL subscriber tasks covers the new task automatically — no reconnect-specific wiring
  #
  # EXAMPLES:
  #   1. I start the TUI against a daemon that already has two sessions open (started from another machine); both sessions appear as tabs without me doing anything
  #   2. While my TUI is open, a session is created from another machine; a new tab appears in my session list without any poll or action on my side
  #   3. A session is closed from another machine; its tab disappears from my TUI, my focus moves to a surviving session (or the board if it was the last one), and the board's work-unit attachment for that session is released
  #
  # QUESTIONS (ANSWERED):
  #   Q: When the connect-time snapshot replay appends N session tabs, which session ends up focused? (Design decision per research doc: the last replayed session grabs focus, then the BUG-206 auto-restore re-asserts the daemon active pointer when one exists — confirm.)
  #   A: Chosen design (per attached research doc): replayed snapshot frames ride the existing Action::SessionCreated handler, so the last-replayed session temporarily grabs focus; the BUG-206 auto-restore (spawned AFTER the synchronous replay in bootstrap) re-asserts the daemon active pointer when one exists. If the pointer is empty, the last-created session stays focused (most-recent intuition, matches embedded behaviour). No replay flag on the wire — simpler, and restore runs deterministically after replay.
  #
  # ASSUMPTIONS:
  #   1. Chosen design (per attached research doc): replayed snapshot frames ride the existing Action::SessionCreated handler, so the last-replayed session temporarily grabs focus; the BUG-206 auto-restore (spawned AFTER the synchronous replay in bootstrap) re-asserts the daemon active pointer when one exists. If the pointer is empty, the last-created session stays focused (most-recent intuition, matches embedded behaviour). No replay flag on the wire — simpler, and restore runs deterministically after replay.
  #
  # ========================================
  Background: User Story
    As a operator
    I want to run the fspec TUI against a shared daemon over WebSocket (or embedded) and see session tabs appear/disappear as sessions are created or closed on ANY connected machine
    So that my client always reflects the daemon's live session set without polling, including when I connect late or reconnect

  Scenario: Starting the TUI against a daemon with open sessions seeds a tab for each of them
    Given a daemon whose session manager already holds two live sessions (created from another machine)
    When the TUI boots against that daemon (either transport)
    Then the TUI's session list contains a tab for each of those two sessions, seeded from the open-session snapshot before any session-creation RPC was issued by this client

  Scenario: A remote session creation adds a tab live (no poll)
    Given a TUI is connected to a daemon over WebSocket with no sessions open yet
    When a session is created from another machine connected to the same daemon
    Then a new session tab appears in the TUI's session list without any poll or user action

  Scenario: A remote session removal closes its tab with focus and board cleanup
    Given a TUI connected to a daemon shows two live session tabs, one of which is focused and has a work unit attached on the board
    When that session is closed from another machine connected to the same daemon
    Then the TUI removes that session's tab, the focused session becomes the surviving one, and the board's work-unit attachment for the removed session is released

  Scenario: Removal events are idempotent for already-closed tabs
    Given a TUI connected to a daemon and a session that has already been closed locally (its tab already removed)
    When the TUI processes a SessionRemoved event for that already-closed session
    Then the TUI ignores the event as a no-op (no panic, no duplicate cleanup, no effect on focus or the board)
