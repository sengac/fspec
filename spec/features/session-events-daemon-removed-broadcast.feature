@done
@rust
@session
@daemon
@in-memory
@RPC-433
Feature: Daemon-side session removal broadcast + in-memory live-session snapshot
  """
  SessionManager-level support for RPC-433's wire fan-out (rules R2/R3 of
  the wire-side feature session-events-daemon-fanout.feature):
  session_removed_tx (capacity 256) fired by destroy_session ONLY when the
  in-memory map actually lost a session (idempotent destroy of an unknown
  id emits nothing), and live_session_infos() returning the in-memory map
  ONLY (no persisted-manifest merge, no project filter — parity with the
  BUG-206 liveness gate; the destroyed session's manifest stays on disk,
  so a snapshot that merged disk state would leak closed sessions).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: SessionManager gains session_removed_tx (capacity 256); destroy_session fires SessionEvent::Removed ONLY when the shift_remove actually removed a session (idempotent destroy of an unknown id emits nothing, and the pre-existing Err("Session not found") return contract is unchanged)
  #   2. R3: SessionManager::live_session_infos() returns the in-memory session map ONLY (no persisted-manifest merge, no project filter) — parity with the BUG-206 liveness gate; it is the connect-time replay source the WS server pushes as Envelope::SessionSnapshot
  #
  # EXAMPLES:
  #   1. A session is destroyed; its manifest persists on disk (resume-able later), yet the next in-memory snapshot no longer lists it
  #   2. destroy_session is called twice for the same id (or once for an unknown id): only the live destroy emits a removal event
  #
  # ========================================
  Background: User Story
    As a daemon operator
    I want session removals to be observable as an in-process broadcast, and the live-session snapshot to reflect only in-memory sessions
    So that the WebSocket fan-out layer (the sibling feature session-events-daemon-fanout) has a correct removal source and a disk-clean connect-time replay source

  Scenario: Destroying an unknown session id pushes no removal event
    Given a session manager with a subscriber on the removed session broadcast
    When destroy_session is called for a session id that is not in the manager's live map
    Then no removal event is observed on the removed broadcast

  Scenario: The in-memory live-session snapshot excludes destroyed sessions
    Given the daemon's session manager holds two live sessions
    When one of those sessions is destroyed
    Then the in-memory live-session snapshot excludes the destroyed session and retains the other
