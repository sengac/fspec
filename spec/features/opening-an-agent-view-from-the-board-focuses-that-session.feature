@done
@tui
@rust
@agent-view
@board
@multi-session
@session-switch
@BUG-553
Feature: Opening an agent view from the board focuses exactly the requested session
  """
  codelet-fspec-tui only. New App::open_agent_view_on(session) (app/dispatch_session_cycle.rs): when the session has an open tab, focus it with switch_to_session_index (BUG-377 refusal, BUG-549 draft sync, spawn_set_active_session, spawn_load_supervisors, probe_exec_stdin_for); otherwise attach it like the resume list (handle_attach_to_session: append, finish_session_attach_wiring, hydration, transcript resume). Then flip the view to Agent and drop the board ring focus. Callers: handle_open_agent_view Some(session) and the None branch's first open session, and the board-surface MenuChipActivate arm (dispatch_menu.rs), which used to call focus_session_index directly. The legacy navigation target slot keeps being recorded (RPC-012/RPC-097 pin it) but nothing reads it. Mux mode never emits OpenAgentView (Shift+arrows are mux pane focus; chips go through dispatch_menu_mux), so no mux branch is needed.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Opening the agent view for a given session from the board (Shift+Right on a work unit with an attached session) focuses exactly that session through the same focus path as every other route: the BUG-549 composer draft swap, the daemon's active-session pointer, the supervisor badge and the exec-stdin probe all follow the requested session
  #   2. A requested session that has no open tab is attached like the resume list does (tab appended and focused, draft hydrated, transcript resumed), never shown as some other session
  #   3. Shift+Right on a work unit with no attached session while sessions are open resumes the first open session (RPC-097 reopen 2, TS sessionGetFirst) and focuses it through the same path; with no open session the create-session dialog still overlays the board
  #   4. Activating a session chip in the board menu bar focuses that session through the same path (active-session pointer, supervisors, exec-stdin probe), as the agent-view chip already does (BUG-195)
  #   5. The agent view shows the focused session; the legacy navigation target slot is still recorded for the RPC-012 and RPC-097 contracts but never decides which session is shown
  #
  # EXAMPLES:
  #   1. s-1 and s-2 are open and s-2 is focused with the draft 's2 text'; s-1 holds 's1 text' and is attached to AUTH-001; on the board Shift+Right on AUTH-001 shows the agent view on s-1 with 's1 text' in the composer, s-2 keeps 's2 text', the daemon active session is s-1 and s-1's supervisors are loaded
  #   2. AUTH-001 is attached to s-3, which has no open tab, and the daemon holds the draft 'resume me' for s-3; Shift+Right on AUTH-001 opens a tab for s-3, focuses it, resumes its transcript and the composer shows 'resume me'
  #   3. s-1 and s-2 are open and s-2 is focused; on the board Shift+Right on an unattached work unit shows the agent view on s-1, the daemon active session is s-1
  #   4. s-1 and s-2 are open and s-2 is focused; on the board activating the s-1 chip shows the agent view on s-1, the daemon active session is s-1 and s-1's supervisors are loaded
  #
  # ========================================
  Background: User Story
    As a user opening an agent view from the board
    I want to have the agent view open on exactly the session I asked for, focused like every other route
    So that I type into and read the session I picked, with its own draft, chrome and supervisors, never the one that happened to be focused before

  # ─────────────────────────────────────────────────────────────────────
  # Shift+Right on a work unit with an attached session (rules 1, 2, 5)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Shift+Right on a work unit attached to an open session focuses that session
    Given sessions s-1 and s-2 are open
    And session s-1 holds the draft "s1 text" and is attached to work unit AUTH-001
    And session s-2 is focused with the draft "s2 text"
    And the user is on the board with AUTH-001 selected
    When the user presses Shift+Right
    Then the agent view shows session s-1
    And the composer shows "s1 text"
    And the in-memory draft of s-2 is "s2 text"
    And the daemon's active session is s-1
    And the supervisors of s-1 are loaded

  Scenario: Shift+Right on a work unit attached to a session with no open tab attaches it
    Given session s-1 is open and focused
    And work unit AUTH-001 is attached to session s-3, which has no open tab
    And the daemon holds the draft "resume me" for session s-3
    And the user is on the board with AUTH-001 selected
    When the user presses Shift+Right
    Then the agent view shows session s-3
    And session s-3 has an open tab
    And the transcript of s-3 is resumed
    And the composer shows "resume me"
    And the daemon's active session is s-3

  # ─────────────────────────────────────────────────────────────────────
  # Shift+Right on a work unit with no attached session (rule 3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Shift+Right on an unattached work unit resumes the first open session
    Given sessions s-1 and s-2 are open
    And session s-2 is focused with the draft "s2 text"
    And the user is on the board with an unattached work unit selected
    When the user presses Shift+Right
    Then the agent view shows session s-1
    And the composer is empty
    And the in-memory draft of s-2 is "s2 text"
    And the daemon's active session is s-1

  # ─────────────────────────────────────────────────────────────────────
  # Board menu-bar chip (rule 4)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Activating a session chip on the board focuses that session through the focus path
    Given sessions s-1 and s-2 are open
    And session s-2 is focused
    And the user is on the board
    When the user activates the s-1 chip in the board menu bar
    Then the agent view shows session s-1
    And the daemon's active session is s-1
    And the supervisors of s-1 are loaded
