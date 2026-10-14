@done
@rust
@session
@daemon
@in-memory
@clear
@rpc-436
Feature: Daemon-side session cleared broadcast (manager-level liveness gate)

  """
  SessionManager-level support for RPC-436's wire fan-out (the sibling
  feature session-events-cleared-fan-out): SessionManager gains
  session_cleared_tx (capacity 256, next to session_created_tx /
  session_removed_tx). clear_session_history resolves the session and
  fires the cleared broadcast ONLY when the clear actually succeeded —
  an unknown id emits nothing and returns the pre-existing
  Err("Session not found") contract (rule R2 of
  session-events-cleared-fanout's Example Mapping context, parity with
  RPC-433's removed-broadcast idempotency).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: SessionManager gains a session_cleared_tx broadcast sender (capacity 256, next to session_created_tx / session_removed_tx, sharing the supervisor broadcast capacity + lag-recovery contract); the event fires ONLY when a clear actually succeeded — a clear of an unknown id emits nothing and returns the existing Err("Session not found") contract (mirrors the R2 idempotency of RPC-433's removed-broadcast)
  #   2. R3: The event is fired INSIDE BackgroundSession::clear_history (the documented single source of truth for clear) AFTER the state clear + context-reinjection succeed, by resolving the owning manager (owning_manager() -> SessionManager::instance() fallback) and sending the SessionId on that manager's session_cleared_tx — so every clear entry point (RPC handle, NAPI session_clear_history, agent-loop bridge control "clear") emits exactly one event without touching each caller
  #
  # EXAMPLES:
  #   1. A session is cleared; the cleared broadcast carries exactly one event with that session's id
  #   2. clear_session_history is called for an unknown id: the Err("Session not found") contract holds and NO cleared event is emitted
  #
  # ========================================

  Background: User Story
    As a daemon operator
    I want session clears to be observable as an in-process broadcast
    So that the WebSocket fan-out layer (the sibling feature session-events-cleared-fan-out) has a correct cleared source that only fires for sessions that actually exist

  Scenario: Clearing an unknown session id pushes no cleared event
    Given a session manager with a subscriber on the cleared session broadcast
    When clear_session_history is called for a session id that is not in the manager's live map
    Then the clear returns the existing not-found error
    And no cleared event is observed on the cleared broadcast


  Scenario: Clearing a live session pushes exactly one cleared event carrying its id
    Given the session manager holds one live session
    When clear_session_history is called for that session
    Then exactly one cleared event is observed on the cleared broadcast, carrying the cleared session's id
    And a second clear of the same id pushes exactly one more cleared event (clears are repeatable — unlike destroy, a cleared session stays live)
