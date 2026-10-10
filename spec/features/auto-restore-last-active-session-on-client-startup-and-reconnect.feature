@done
@tui-113
@tui
@session-resume
@reconnect
@startup
@TUI-113
Feature: Auto-restore last active session on client startup and reconnect

  """
  Architecture:
  - New module rust/fspec-tui/src/app/session_restore.rs (impl App::resolve_restore_target +
    App::spawn_session_restore) — pure resolution logic (focused-session override →
    get_active_session → list_sessions(daemon_cwd).first()) returning Option<SessionId>,
    kept under the 300-LoC ceiling.
  - bootstrap.rs calls spawn_session_restore() after the existing bootstrap RPCs;
    dispatch_reconnect.rs calls it with the focused-session override and replaces the
    unconditional create_session(None) with a resolve-then-attach (fallback create only
    when resolution is empty).
  - dispatch_attach.rs / dispatch_session_cycle.rs / session_creation.rs gain
    set_active_session fire-and-forget calls. The daemon pointer is STICKY — BackToBoard
    does not clear it (user decision 2026-10-10), so the last-viewed session restores
    after a restart even if the user was on the board.
  - All backend calls go through the existing FspecBackend trait (get_active_session /
    set_active_session / list_sessions / get_workspace_info / resume_session /
    get_buffered_output) so both embedded + websocket transports work unchanged
    (RPC-037 cross-transport parity).
  - The restore target is attached via the EXISTING Action::AttachToSession path
    (RPC-049 durable restore: resume_session + get_buffered_output replay) — no new
    transport RPCs are introduced.
  - Supersedes RPC-011 scenario 'Reconnect re-issues create_session(None)':
    re-attach replaces create-session; the create_session fallback only fires on an
    empty daemon (see R4/R5).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R4: The restore task never creates a second session if it attaches — it only dispatches Action::AttachToSession(target). create_session(None) fires ONLY as a fallback when all resolution steps yield nothing: reconnect with no focused session AND get_active_session() == None AND list_sessions(daemon_cwd) is empty. This preserves the RPC-011/012 safety net (a genuinely empty daemon still gets a session when the user first enters a work unit) while preventing the 'phantom empty session' symptom.
  #   2. R1: On App::bootstrap, a detached tokio task resolves the restore target and dispatches the existing Action::AttachToSession on success; bootstrap never awaits the resolution, so the Board first frame renders immediately (mirrors the lazy create_session spawn pattern).
  #   3. R2: Resolution order — (a) reconnect path (Action::Reconnected): the client's currently focused session wins (tabs survive transient WS drops; another client may have moved the daemon pointer meanwhile); (b) get_active_session() — the daemon's in-memory active-session pointer (RPC-037); (c) most-recent entry of list_sessions(daemon_cwd) where daemon_cwd comes from get_workspace_info().cwd (RPC-018) with fallback to empty string (no project filter) — NEVER the client-side std::env::current_dir(), because on a remote client machine that path matches no daemon session (the RPC-427 filter would empty the list).
  #   4. R5: Supersedes RPC-011 scenario 'Reconnect re-issues create_session(None)': on Action::Reconnected the re-bootstrap re-ATTACHES the focused/existing session (Action::AttachToSession → resume_session + get_buffered_output replay, which recovers chunks streamed while the WS was down) instead of creating a new one. create_session(None) runs only when no session is restorable (R4). The list_work_units refetch + subscriber respawn on reconnect is unchanged. The RPC-011 feature file keeps the old scenario with a SUPERSEDED-BY-TUI-113 annotation.
  #   5. R3: The TUI keeps the daemon's active-session pointer fresh by calling set_active_session(sid) on every attach (Action::AttachToSession handler), on session focus switch (switch_to_session_index — RPC-024 cycle, mux focus follow, chip activation), and on session creation (route_bootstrap_create_session / handle_session_created). All three are fire-and-forget (backend errors are logged at debug and never block the UI). NOTE: BackToBoard does NOT clear the pointer (user decision 2026-10-10: the daemon pointer is sticky; 'I was on the board' is not an explicit state) — a later restart therefore restores the last-viewed session, which is the desired behaviour.
  #
  # EXAMPLES:
  #   1. Transient WS drop while watching a running agent: on reconnect the SAME session tab re-attaches, the '✓ Reconnected' line lands in it, and chunks streamed while the connection was down are replayed — no new session is created
  #   2. Remote client reconnects via Tailscale (fresh process) to a daemon whose session 'Session be4bd683' is still running: the TUI lands back in the Agent view on that session with its scrollback replayed via get_buffered_output, not a blank new session
  #   3. User's most-recent session is "S-recent" (the daemon restarted so its in-memory active pointer is lost), closed the client, and restarted it: "S-recent" re-opens from the persisted list, while a daemon with zero sessions still boots to the board with NO session created (lazy create-on-enter-work-unit path untouched)
  #   4. Daemon process restarts (SIGTERM + relaunch) while a client stays connected: the reconnect handler finds no focused session on the new daemon's fresh in-memory map, falls back to the persisted most-recent session for the workspace, and re-attaches it via resume-from-disk
  #   5. Fresh client, fresh daemon (no sessions on disk, no active pointer): reconnect re-bootstrap still creates a session exactly as RPC-011 required — the safety net for a genuinely empty workspace survives
  #   6. Client machine cwd (/Users/remote/code) differs from the daemon workspace (/home/rquast/projects/fspec): the restore path queries sessions with the daemon's workspace path (from workspace info), so the RPC-427 project filter never empties the list; if the client cwd were used instead the list would be empty and restore would silently fail
  #   7. User cycles between three open agent panes then closes the client: the daemon's active pointer tracks the last-focused session (set on each focus switch), so on the next connect exactly that last-focused session restores — not the first or most-recent-by-time one
  #   8. Reconnect fires twice in quick succession (WS flap): the second re-attach targets the already-open session tab, focuses it, and does not spawn a second session or a second resume_session round-trip — the append path is idempotent
  #
  # ========================================

  Background: User Story
    As a developer running fspec in daemon/client mode
    I want to start or reconnect a client while the daemon already has sessions
    So that the session I was last viewing is restored automatically — agent state (scrollback, pending input, work-unit binding) re-attaches without hunting through /resume or losing the thread

  Scenario: Reconnect re-attaches the focused session instead of creating a new one
    Given a client is viewing session "S-focus" and the WebSocket connection drops
    When the supervisor reconnects and Action::Reconnected is dispatched
    Then the focused session "S-focus" is re-attached via the existing attach path
    And no create_session RPC is issued
    And the inline reconnect notice lands in the "S-focus" session's scrollback

  Scenario: Reconnect replays output streamed while the connection was down
    Given a client is viewing session "S-focus" and the WebSocket connection drops
    And the daemon streamed 3 assistant text chunks for "S-focus" while the connection was down
    When the supervisor reconnects and the re-attach completes
    Then the 3 chunks are replayed into "S-focus"'s scrollback via the buffered-output round-trip
    And live streaming chunks continue after the replay

  Scenario: Fresh client process restores the daemon's active session on bootstrap
    Given a daemon has a live session "S-active" set as its active session
    When a new client process starts and App::bootstrap runs
    Then bootstrap returns without awaiting the restore
    And the client re-attaches "S-active" via the existing durable-restore path
    And the TUI lands in the Agent view on "S-active" with its scrollback replayed

  Scenario: Fresh client process restores the most-recent workspace session when no active pointer exists
    Given a daemon's active-session pointer is unset (e.g. no client has attached yet after a daemon restart)
    And the daemon's workspace has persisted sessions ["S-older", "S-recent"] by recency
    When a new client process starts and App::bootstrap runs
    Then the client restores "S-recent" — the most-recent entry of list_sessions(daemon_cwd)
    And "S-older" is not restored

  Scenario: Daemon restart restores the persisted most-recent session via resume-from-disk
    Given a daemon was killed and relaunched so its in-memory session map is empty
    And a client stays connected and its supervisor reconnects
    And the re-attach finds no focused session on the fresh daemon
    When the restore falls back to the persisted sessions for the daemon's workspace
    Then the most-recent persisted session is re-attached via the resume_session round-trip
    And its conversation history replays from disk

  Scenario: Empty daemon still creates a session on reconnect (RPC-011 safety net preserved)
    Given a fresh daemon with no active session and no persisted sessions for its workspace
    And a client's supervisor reconnects with no focused session
    When the restore resolution yields no candidate
    Then create_session(None) is issued as the fallback
    And the new session becomes the active session exactly as RPC-011 specified

  Scenario: Restore queries sessions by the daemon workspace path, not the client cwd
    Given the client machine's cwd is "/Users/remote/code"
    And the daemon workspace is "/home/rquast/projects/fspec"
    And a session "S-ds" exists for the daemon workspace but NOT for the client cwd
    When a fresh client process resolves the restore target
    Then list_sessions is called with "/home/rquast/projects/fspec" (from get_workspace_info)
    And "S-ds" is a candidate and gets restored

  Scenario: Focus switches track the daemon active pointer
    Given a client has three open sessions "S-a", "S-b", "S-c"
    When the user cycles focus S-a → S-b → S-c
    Then set_active_session is called with "S-c" last
    And on the next client start "S-c" restores — not "S-a" and not the most-recent-by-time session

  Scenario: Repeated reconnects re-attach idempotently
    Given a client is viewing session "S-focus"
    When the WebSocket flaps and Action::Reconnected fires twice in quick succession
    Then both re-attach targets resolve to the already-open "S-focus" tab
    And exactly one session exists for "S-focus" in the open-session list
    And no duplicate session tab is appended

  Scenario: Reconnect does not recreate a destroyed session
    Given a client whose last-viewed session "S-x" was destroyed and closed while the client was running
    And the daemon has no sessions and no active pointer for its workspace
    When the client reconnects
    Then the R4 create_session fallback fires exactly once (RPC-011 safety net)
    And the closed "S-x" tab is not re-opened
    And the fallback-created session becomes the active session

  Scenario: Esc back to board keeps the active pointer sticky so restart restores the last-viewed session
    Given a client is viewing session "S-focus"
    When the user presses Esc to exit to the board view
    Then the daemon's active-session pointer still references "S-focus" (it is sticky — not cleared on board exit)
    And on the next client start "S-focus" restores as the last-viewed session
