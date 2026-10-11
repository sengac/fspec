@done
@BUG-206
@tui-113
@tui
@session-resume
@reconnect
@startup
@TUI-113
Feature: Session auto-restore only restores sessions live in the daemon

  """
  Architecture:
  - MEMORY-ONLY restore (supersedes the TUI-113 disk-aware fallbacks):
    rust/fspec-tui/src/app/session_restore.rs `resolve_restore_target`
    builds the candidate list — (a) on the reconnect path, the client's
    focused session (tabs survive transient WS drops); (b) the daemon's
    in-memory active-session pointer (get_active_session) — and verifies
    each candidate via `list_live_sessions()`, a NEW RPC that returns
    ONLY the daemon's in-memory session map (SessionManager →
    SessionManagerHandle → FspecService → FspecBackend; embedded +
    websocket + MockBackend transports). Persisted manifests (disk)
    are NEVER candidates: no `list_sessions` disk scan, no
    `resume_session` (RPC-049 durable restore), no `create_session`
    fallback. The RPC-012 lazy create-on-enter-work-unit path is the
    only session-creation path on startup/reconnect.
  - Nothing live → NOTHING happens: the client lands on the board
    (user decision 2026-10-11: "Land on the board"). No fallback of
    any kind — the auto-restore path must not resurrect sessions the
    user closed, nor create sessions on a genuinely empty daemon.
  - The daemon's active-session pointer is invalidated when the
    session it references is destroyed: `SessionManager::destroy_session`
    clears the pointer when it pointed at the destroyed session
    (single source of truth — no extra client-side clear call). The
    pointer remains STICKY for board-exit (BackToBoard does not
    clear it) — but a closed session is never live, so the gate
    protects even a stale pointer.
  - Re-attach is LIVE and MEMORY-ONLY (Action::ReattachSession →
    `App::handle_reattach_session` in dispatch_attach.rs): an
    already-open tab (transient WS drop) is focused in place; a
    closed/absent tab (fresh client start) is re-opened. NO disk
    `resume_session` round-trip (it would double-inject a live
    session's persisted conversation into the LLM context and
    re-broadcast every chunk). Catch-up: the daemon's in-memory
    `get_buffered_output` (RPC-037) chunks are replayed through the
    chunk pipeline so the view catches up to whatever the daemon has
    — a fresh tab lands on the live conversation, a tab that
    survived a WS drop picks up the chunks streamed while the
    connection was down.
  - bootstrap.rs calls `spawn_session_restore(None)`; dispatch_reconnect.rs
    calls it with the focused-session override after the RPC-011/0415
    re-bootstrap (work-unit refetch + subscriber respawn unchanged).
  - All backend calls go through the existing FspecBackend trait so
    both embedded + websocket transports work unchanged (RPC-037
    cross-transport parity).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The auto-restore resolution (bootstrap + reconnect) considers
  #     ONLY sessions running in the daemon's in-memory session map.
  #     Resolution order: (a) on reconnect, the client's focused session;
  #     (b) the daemon's active-session pointer (get_active_session). The
  #     candidate must be verified LIVE via list_live_sessions() before any
  #     restore dispatch. No candidate, or a candidate that is not live →
  #     NOTHING happens: land on the board. NO list_sessions disk scan, NO
  #     resume-from-disk, NO create_session fallback. The RPC-012 lazy
  #     create-on-enter-work-unit path is the only session-creation path on
  #     startup/reconnect.
  #   2. R3: A new RPC list_live_sessions() returns ONLY the session IDs of
  #     the daemon's in-memory session map (SessionManager::all_in_memory_session_ids
  #     surfaced via SessionManagerHandle → FspecService → FspecBackend,
  #     embedded + websocket + MockBackend). It NEVER reads persisted
  #     manifests. The auto-restore path uses it exclusively for candidate
  #     verification; list_sessions (the disk-merging /resume list) is
  #     removed from the auto-restore path entirely.
  #   3. R4: The daemon's in-memory active-session pointer is invalidated
  #     when the session it references is destroyed:
  #     SessionManager::destroy_session clears the pointer when it pointed
  #     at the destroyed session (single source of truth — the client's
  #     close path needs no extra clear call). A pointer must never
  #     reference a closed session.
  #   4. R5: The re-attach is live-gated and MEMORY-ONLY: no disk
  #     resume_session round-trip (a live session's persisted conversation
  #     would be double-injected into its LLM context); the in-memory
  #     get_buffered_output catch-up is how a fresh/re-opened tab lands on
  #     the live conversation and a tab that survived a WS drop picks up
  #     chunks streamed while the connection was down.
  #
  # EXAMPLES:
  #   1. Fresh client start while the daemon is running session 'S-live'
  #     (in its in-memory map) and a long-closed session 'S-closed' exists
  #     on disk: the TUI restores 'S-live' (the daemon's live session) and
  #     NEVER re-opens 'S-closed'
  #   2. Transient WS drop while the user watches session 'S-focus' and
  #     'S-focus' is still running in the daemon: on reconnect the SAME
  #     'S-focus' tab re-attaches and no new session is created
  #   3. Fresh client start with a daemon that has ZERO in-memory sessions
  #     (only persisted manifests on disk): nothing restores — the TUI
  #     lands on the board and creates no session (the lazy
  #     create-on-enter-work-unit path is the only creation path)
  #   4. Client reconnects (WS flap) but the daemon's active pointer and
  #     the client's focused session both reference a session that was
  #     CLOSED since the drop: nothing re-attaches — the closed tab stays
  #     closed and the TUI does not resurrect it from disk
  #
  # ========================================

  Background: User Story
    As a developer running fspec in daemon/client mode
    I want to start or reconnect a client and have it re-attach a session that is actually running in the daemon
    So that I never get resurrected sessions I closed, and no fallbacks of any kind

  @BUG-206
  Scenario: Reconnect re-attaches the focused session instead of creating a new one
    Given a client is viewing session "S-focus" and the WebSocket connection drops
    When the supervisor reconnects and Action::Reconnected is dispatched
    Then the focused session "S-focus" is re-attached via the live re-attach path
    And no create_session RPC is issued
    And the re-attach NEVER runs the disk resume_session round-trip (BUG-206: the candidate is live — the disk durable-restore path would double-inject its LLM context)
    And the inline reconnect notice lands in the "S-focus" session's scrollback

  @BUG-206
  Scenario: Reconnect replays output streamed while the connection was down
    Given a client is viewing session "S-focus" and the WebSocket connection drops
    And the daemon streamed 3 assistant text chunks for "S-focus" while the connection was down
    When the supervisor reconnects and the re-attach completes
    Then the 3 chunks are replayed into "S-focus"'s scrollback via the in-memory buffered-output catch-up
    And the catch-up came from the IN-MEMORY buffer — no disk resume_session
    And live streaming chunks continue after the replay

  @BUG-206
  Scenario: Fresh client process restores the daemon's active session on bootstrap
    Given a daemon has a live session "S-active" set as its active session
    When a new client process starts and App::bootstrap runs
    Then bootstrap returns without awaiting the restore
    And the client re-attaches "S-active" via the live re-attach path
    And the re-attach is memory-only — NO disk resume_session round-trip
    And the TUI lands in the Agent view on "S-active" with its scrollback replayed

  @BUG-206
  Scenario: Fresh client process lands on the board when no session is live
    Given a daemon's active-session pointer is unset (e.g. no client has attached yet after a daemon restart)
    And the daemon's workspace has ONLY persisted sessions on disk (["S-older", "S-recent"]) — none of them is in the daemon's in-memory map
    When a new client process starts and App::bootstrap runs
    Then nothing is restored — the TUI stays on the board with NO open session
    And the restore path never touched the disk — no resume_session and no list_sessions call

  @BUG-206
  Scenario: Daemon restart lands the client on the board (the pointer is gone)
    Given a daemon was killed and relaunched so its in-memory session map is empty
    And a client stays connected and its supervisor reconnects
    And the re-attach finds no focused session on the fresh daemon
    Then nothing is re-attached — the persisted sessions are NOT resumed from disk

  @BUG-206
  Scenario: Empty daemon lands on the board on reconnect (NO create fallback)
    Given a fresh daemon with no active session and no live sessions
    And a client's supervisor reconnects with no focused session
    When the restore resolution yields no candidate
    Then NO create_session is issued — the client lands on the board

  @BUG-206
  Scenario: A closed candidate is never restored — the liveness gate drops it
    Given the daemon's active pointer still references session "S-closed" but "S-closed" is NOT in the daemon's in-memory map (it was closed)
    When a new client process starts and App::bootstrap runs
    Then the closed "S-closed" candidate is dropped by the liveness gate — nothing restores

  @BUG-206
  Scenario: Reconnect does not recreate a destroyed session
    Given a client whose last-viewed session "S-x" was destroyed and closed while the client was running
    And the daemon has no sessions and no active pointer for its workspace
    When the client reconnects
    Then the closed "S-x" tab is NOT re-opened and NO session is created

  @BUG-206
  Scenario: Focus switches track the daemon active pointer
    Given a client has three open sessions "S-a", "S-b", "S-c"
    When the user cycles focus S-a → S-b → S-c
    Then set_active_session is called with "S-c" last
    And on the next client start "S-c" restores (because it is LIVE) — not "S-a" and not a disk entry

  @BUG-206
  Scenario: Repeated reconnects re-attach idempotently
    Given a client is viewing session "S-focus"
    When the WebSocket flaps and Action::Reconnected fires twice in quick succession
    Then both re-attach targets resolve to the already-open "S-focus" tab
    And exactly one session exists for "S-focus" in the open-session list
    And no duplicate session tab is appended

  @BUG-206
  Scenario: Esc back to board keeps the active pointer sticky so restart restores the last-viewed session
    Given a client is viewing session "S-focus"
    When the user presses Esc to exit to the board view
    Then the daemon's active-session pointer still references "S-focus" (it is sticky — not cleared on board exit)
    And on the next client start "S-focus" restores as the last-viewed session (while it is still live)

  @BUG-206
  Scenario: A stale pointer is skipped — the daemon's live sessions win
    Given a client reconnects while focused on session "S-focus" (the tab survived the WS drop) and the daemon pointer still references the CLOSED "S-closed" session
    When the supervisor reconnects and Action::Reconnected is dispatched
    Then the client re-attaches the focused LIVE session "S-focus"
    And the closed pointer target is never restored
