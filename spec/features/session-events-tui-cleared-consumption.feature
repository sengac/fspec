@done
@rust
@tui
@session
@multi-session
@clear
@parity
@websocket
@RPC-437
Feature: TUI consumes the SessionEvent::Cleared lifecycle event (both transports)
  """
  TUI-side half of the RPC-433/RPC-436 session lifecycle event stream:
  completes the RPC-434 created/removed consumption with the third
  transition, clear. FspecBackend gains session_cleared_rx (default:
  closed receiver — RPC-037/PROV-109 convention); EmbeddedFspecBackend
  delegates to the SessionManagerHandle's session_cleared_rx (zero-cost
  direct read, same match pattern as session_created_rx /
  session_removed_rx); WebSocketFspecBackend splits the RPC-433
  SessionEvent wire stream onto a per-subscriber Cleared-only broadcast
  (sibling of session_created_rx / session_removed_rx; closed receiver
  when the client slot is None, respawned on Reconnected). New
  Action::SessionCleared(SessionId) folded by an IDEMPOTENT
  App::handle_session_cleared that resets the cleared session's local
  scrollback + input draft + RPC-100 chrome (status → Cleared, token
  state, compaction reduction) and — only when that session is focused —
  additionally resets the AgentView input buffer + popups through the
  SAME shared AgentView::reset_local_scrollback the local /clear step
  uses (DRY). Bootstrap spawns a 9th subscriber task
  (session_cleared_rx -> Action::SessionCleared) in its own extracted
  module (app/bootstrap_session_cleared.rs, 300-LoC ceiling); the
  Reconnected handler's abort+respawn covers it automatically. The
  local /clear continues to ride the existing
  StreamChunk::SessionStateChange { state: Cleared } chunk for its
  reactive reset; the echoed Cleared event fold is idempotent on top.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: FspecBackend gains session_cleared_rx() -> broadcast::Receiver<SessionId> with a default closed receiver; EmbeddedFspecBackend overrides with the zero-cost direct read of the attached SessionManagerHandle::session_cleared_rx() (closed receiver when no session manager is attached); WebSocketFspecBackend overrides with a per-subscriber splitter task over FspecWsClient::session_events_rx that forwards ONLY the SessionEvent::Cleared variant (capacity 16, shared SESSION_EVENT_SPLITS_CAPACITY; closed receiver when the client slot is None so the subscriber exits on RecvError::Closed and is respawned on Reconnected)
  #   2. R2: New Action::SessionCleared(codelet_rpc_types::SessionId) variant in components/mod.rs (appended after the RPC-434 SessionRemoved so existing variant order is untouched); App::dispatch routes it to the idempotent handle_session_cleared
  #   3. R3: handle_session_cleared is IDEMPOTENT and per-session (never touches focus): it (a) resets the cleared session's local SessionContext scrollback + in-flight slots via ctx.reset_scrollback() and blanks that session's input draft, (b) when the cleared session is the FOCUSED one, additionally resets the AgentView input buffer + closes slash/file popups (the local /clear's AgentView-local step, extracted to a shared AgentView::reset_local_scrollback so /clear and the remote fold share ONE code path — DRY), and (c) mirrors the chunk-side Cleared state reset (RPC-100): AgentViewStore::set_session_status(sid, Cleared) + reset_token_state(sid) + clear_compaction_reduction(sid), so the SessionHeader badge returns to [0%] even when the chunk stream lags the lifecycle event. A Cleared for a session whose tab is not open is a complete no-op (no panic, no tab creation, no effect on other sessions)
  #   4. R4: Bootstrap spawns a 9th subscriber task (session_cleared_rx -> Action::SessionCleared) using the SAME Lagged-debug/Closed-break loop shape as the session-removed subscriber (app/bootstrap_session_cleared.rs, extracted so bootstrap.rs stays under the 300-LoC ceiling; spawned alongside spawn_session_removed_subscriber). The Reconnected handler's abort+respawn of ALL subscriber tasks covers the new task automatically — no reconnect-specific wiring
  #   5. R5: The LOCAL /clear path (handle_slash_clear) is unchanged in observable behaviour: it still resets the focused session's local scrollback + input synchronously and then spawns backend.clear_history(session_id). Its local reset step now delegates to the SAME shared AgentView::reset_local_scrollback used by handle_session_cleared (DRY; RPC-074 TS parity preserved — no scrollback notice line, errors to tracing::error! only). A LOCAL /clear fires BOTH the existing StreamChunk::SessionStateChange { state: Cleared } chunk (reactive UI reset contract) AND the RPC-436 SessionEvent::Cleared lifecycle event — the event's fold is idempotent on top of the chunk reset (double-reset is a no-op)
  #
  # EXAMPLES:
  #   1. A SessionCleared event arrives for a session whose tab is not open in this TUI (closed locally, or never opened). The App treats it as a complete no-op: no tab is created, focus and the board are untouched, no panic
  #   2. I have two TUI clients (A over embedded, B over WebSocket) attached to the same daemon, and both show session S's tab with scrollback. I type /clear in client A. Client B's session S tab empties (scrollback + input) and its SessionHeader badge returns to [0%] — without any poll, driven by the SessionEvent::Cleared frame over the RPC-433 wire stream. Client A's tab is reset by its own local /clear step AND by the echoed Cleared event (idempotent)
  #   3. Two consecutive /clear of the same live session from any machine produce exactly two SessionEvent::Cleared frames (clear is REPEATABLE — the session stays live), and each fold is a no-op on the second if the first already emptied the tab; no duplicate tabs, no state drift
  #   4. A session S is cleared from the Telegram bridge (NAPI clear entry point) while my TUI has S's tab open in the background (a different session is focused). The TUI resets S's local scrollback + input draft without touching my focused session's input buffer or focus, and S's per-session token/compaction badge state resets
  #
  # SCOPE NOTE:
  #   The wire/daemon side (SessionEvent::Cleared variant, manager
  #   liveness-gated broadcast, per-connection WS fan-out) landed in
  #   RPC-436 (spec/features/session-events-cleared-fan-out.feature +
  #   session-events-daemon-cleared-broadcast.feature). The WS live-wire
  #   counterpart (client A clears, client B's transport delivers the
  #   frame) is covered at the wire level by
  #   rpc-server/tests/rpc436_session_cleared_fan_out.rs; these
  #   scenarios pin the App-level behaviour transport-agnostically via
  #   the MockBackend (the App's bus is transport-agnostic).
  #
  # ========================================
  Background: User Story
    As a operator connected to the fspec daemon
    I want to have the TUI reset a session's local UI when that session is cleared on ANY machine
    So that every connected TUI reflects the clear over the wire (no poll, no chunk-stream dependence for the lifecycle)

  Scenario: A session cleared from another client resets its local tab live (no poll)
    Given a TUI connected to a daemon over WebSocket with one session S whose tab is focused and has scrollback content
    When session S is cleared from another machine connected to the same daemon
    Then S's local scrollback and input draft are reset without any poll or user action
    And S's per-session SessionHeader chrome (status badge, token counters, compaction reduction) returns to the cleared state

  Scenario: A clear event for a background session resets only that session
    Given a TUI with two open sessions S-1 (background) and S-2 (focused), each with scrollback, and S-2's input buffer holding a typed draft
    When a SessionCleared event for S-1 is processed
    Then S-1's local scrollback and input draft are reset
    And S-2's scrollback, input buffer and focus are untouched
    And S-1's per-session token and compaction badge state is reset

  Scenario: Clear events for a session without an open tab are a no-op
    Given a TUI where session S was already closed locally (its tab already removed) or was never opened in this client
    When the TUI processes a SessionCleared event for S (fired twice to pin the double-fire no-op)
    Then the TUI ignores the events as no-ops — no tab is created, no panic, and the surviving session's focus and state are untouched

  Scenario: Consecutive clear events for the same live session are idempotent
    Given a TUI with one open session S whose tab was already reset by a first SessionCleared event
    When a second SessionCleared event for S arrives (clear is repeatable — the session stays live)
    Then the TUI holds exactly one tab for S with no duplicate cleanup, no scrollback notice lines, and no state drift

  Scenario: Bootstrap spawns the session-cleared subscriber and degenerate defaults leak nothing
    Given an App bootstrapped against a backend whose session-cleared receiver is the degenerate default (closed)
    When the App's bootstrap runs
    Then bootstrap spawns nine subscriber tasks — the existing eight plus the new session_cleared_rx subscriber
    And the degenerate (closed) session-cleared receiver must not leak SessionCleared events onto the action bus
