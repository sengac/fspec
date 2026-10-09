@done
@BUG-203
@bug-203
@bug
@board-view
@menu-bar
@tui-component
@keyboard-navigation
@mouse-events
@regression
Feature: Board 'New Agent' gesture always prompts the CreateSessionDialog
  """
  BUG-203 — the board's 'New Agent' gesture (the '.' key, the '[ New Agent ]'
  Zone C button, the Kanban dropdown 'New Agent' row, the 'u' menu-bar-help
  'New Agent' row, and the mux top bar's Kanban 'New Agent' row) used to reuse
  the Shift+Right CYCLE path (Action::OpenAgentView(selected_session) →
  App::handle_open_agent_view): with the selected unit's attached session it
  jumped straight into that session, and with none attached it silently
  resumed the FIRST open session (the RPC-097 reopen #2 probe — correct for a
  cycle gesture, wrong for a 'New Agent' gesture). The user experienced it as
  "attaching to a running agent" instead of being asked to start a new one.

  Fix: the board 'New Agent' gesture becomes an EXACT mirror of the agent
  bar's 'New Agent' button (BUG-199 R1): every board entry point (the '.'
  key, the '[ New Agent ]' Zone C button, the Kanban dropdown 'New Agent'
  row, the 'u' menu-bar-help 'New Agent' row, and the mux top bar's Kanban
  'New Agent' row) resolves through the registry's
  MenuAction::NewAgent → Action::OpenCreateSessionDialog { preselect: None }
  (the shared RPC-060 helper, handle_open_create_session_dialog) and NEVER
  through Action::OpenAgentView (the Shift+Right CYCLE path). It never
  resumes or attaches to an existing session and never binds the fresh
  session to a work unit (the R8 current-session / selected-unit target
  substitution is removed from the registry mapping — BUG-199 superseded it
  for the agent surface; BUG-203 removes it for the board surface too).
  Shift+Right (single view AND mux) stays the CYCLE gesture and is
  untouched: attached-session fast path → first open session (RPC-097
  reopen #2) → dialog only when zero sessions are open.

  Contract:
  - Confirming Yes / Yes - Isolated from a BOARD surface flips to the Agent
    view on the fresh session (the existing handle_create_session_submitted
    board→agent flip) and does NOT bind the session to the selected work
    unit (the store's work-unit slots are untouched; no extra AttachSession
    fires for the selected unit beyond the pre-existing current_work_unit_id
    auto-attach, which is unchanged behavior).
  - Cancel / Esc keeps the user on the board and leaves every open session
    untouched (the RPC-097 reopen #1 overlay contract).
  - The dialog title follows the shared helper's documented contract
    (RPC-060 / RPC-097 R6): context-aware from the store's current session's
    WorkUnitContext ('Work on <id>?' when the current session is bound to a
    unit, else 'Start New Agent?').
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The board 'New Agent' gesture (the '.' key, the '[ New Agent ]'
  #      Zone C button, the Kanban dropdown 'New Agent' row, the 'u'
  #      menu-bar-help 'New Agent' row, and the mux top bar's Kanban
  #      'New Agent' row) ALWAYS mounts the CreateSessionDialog (BUG-199
  #      agent-bar parity via the shared RPC-060 helper) — it never jumps
  #      into / resumes an existing open session, even when the selected
  #      work unit has an attached session or other sessions are open.
  #   2. R2: Shift+Right (single view AND mux) is a CYCLE gesture and is
  #      unchanged — it still cycles/resumes sessions: attached-session
  #      fast path, then the first open session (RPC-097 reopen #2), then
  #      the create dialog only when zero sessions are open.
  #   3. R3: When the 'New Agent' prompt is confirmed (Yes / Yes - Isolated)
  #      from a BOARD surface, the fresh session is created and the view
  #      flips to the Agent view on it — the session is NOT bound to the
  #      board's selected work unit (current_work_unit_id is untouched and
  #      no AttachSession fires for it). Cancel / Esc keeps the user on the
  #      board and leaves every open session untouched.
  #   4. R4: The dialog follows the shared helper's documented title
  #      contract (RPC-060 / RPC-097 R6): context-aware from the store's
  #      CURRENT session's WorkUnitContext — 'Work on <id>?' + 'Start an AI
  #      session for this task' when the current session is bound to a unit,
  #      else 'Start New Agent?' + 'Begin a fresh AI conversation, not
  #      linked to any task.'
  #
  # EXAMPLES:
  #   1. Board with one open session "s-1" attached to AUTH-001; user selects
  #      AUTH-001 and presses '.' → the CreateSessionDialog mounts over the
  #      board; active view stays Board; no session is focused/switched; the
  #      dialog id is present exactly once
  #   2. Board with one open session "s-1" (attached to no unit); user selects
  #      an unattached unit AUTH-002 and presses '.' → CreateSessionDialog
  #      mounts (the first open session is NOT resumed — the user's exact
  #      report); confirming Yes creates a fresh session and flips to the
  #      Agent view WITHOUT binding it to AUTH-002
  #   3. The user's exact report: an agent is running on s-1; from the board
  #      the user clicks the '[ New Agent ]' button (or presses '.') → the
  #      CreateSessionDialog mounts instead of switching into the running
  #      agent s-1; current session stays s-1; the board bar's ring focus
  #      clears
  #   4. Regression guard: with session s-1 open and the user on the board,
  #      Shift+Right still resumes s-1 (active view flips to Agent, no
  #      dialog) — the cycle gesture is untouched; only the 'New Agent'
  #      gesture changed
  #
  # ========================================

  Background: User Story
    As a TUI user on the Kanban board
    I want to start a new agent from the 'New Agent' gesture (the '.' key, the '[ New Agent ]' Zone C button, the Kanban dropdown row, or the 'u' menu-bar-help row)
    So that I am always prompted with the CreateSessionDialog instead of being silently attached to an already-running agent

  @keyboard-navigation
  @regression
  Scenario: Pressing '.' with the selected unit's session attached mounts the CreateSessionDialog instead of jumping into it
    Given the board has one open session "s-1" attached to work unit "AUTH-001"
    And "AUTH-001" is selected in the focused column
    When the user presses the '.' key
    Then the CreateSessionDialog mounts over the board exactly once
    And the active view is still the board (the dialog overlays; the view switch is deferred to confirm)
    And no existing session is focused or switched (the navigation target is not set to "s-1")

  @keyboard-navigation
  @regression
  Scenario: Pressing '.' with an open session but no attachment on the selected unit still mounts the CreateSessionDialog (no silent resume)
    Given the board has one open session "s-1" attached to no work unit
    And the selected work unit "AUTH-002" has no attached session
    When the user presses the '.' key
    Then the CreateSessionDialog mounts over the board
    And the first open session "s-1" is NOT resumed (the active view is still the board)

  @mouse-events
  @regression
  Scenario: Clicking the board's '[ New Agent ]' button with a running agent mounts the CreateSessionDialog instead of switching into it
    Given an agent is running on session "s-1"
    And the board bar's '[ New Agent ]' Zone C button is painted
    When I left-click the '[ New Agent ]' button
    Then the CreateSessionDialog mounts over the board instead of switching into the running agent "s-1"
    And the current session remains "s-1" (untouched)
    And the board bar's ring focus clears

  @keyboard-navigation
  @regression
  Scenario: Confirming Yes on the board's New Agent prompt flips to the Agent view on the fresh session without binding it to a unit
    Given the board has one open session "s-1" attached to no work unit
    And the selected work unit "AUTH-002" has no attached session
    When the user presses the '.' key
    And the user presses Enter on the "Yes" option
    Then a fresh session is created (the backend create_session fires exactly once)
    And the active view is the Agent view on the fresh session
    And the fresh session is NOT bound to work unit "AUTH-002" (no attachment recorded; the store's current work unit is untouched)

  @keyboard-navigation
  Scenario: Cancel on the board's New Agent prompt leaves the user on the board with every open session untouched
    Given the board has one open session "s-1"
    And the selected work unit "AUTH-001" has no attached session
    When the user presses the '.' key
    And the user presses Esc on the dialog
    Then the CreateSessionDialog is dismissed
    And the active view is still the board
    And the current session remains "s-1"

  @keyboard-navigation
  @regression
  Scenario: Shift+Right from the board still resumes the first open session (the CYCLE gesture is unchanged)
    Given the board has one open session "s-1" attached to no work unit
    And the selected work unit "AUTH-002" has no attached session
    When the user presses Shift+Right
    Then the active view is the Agent view on session "s-1" (resumed)
    And the CreateSessionDialog does NOT mount

  @keyboard-navigation
  @regression
  Scenario: Shift+Right from the board with the selected unit's session attached still jumps into it
    Given the board has one open session "s-1" attached to work unit "AUTH-001"
    And "AUTH-001" is selected in the focused column
    When the user presses Shift+Right
    Then the active view is the Agent view on session "s-1"
    And the CreateSessionDialog does NOT mount

  @mouse-events
  Scenario: Executing the Kanban dropdown's 'New Agent' row mounts the CreateSessionDialog
    Given an agent is running on session "s-1"
    And the board's Kanban dropdown is open with the cursor on the 'New Agent' row
    When I click the 'New Agent' row (row 0 of the open dropdown)
    Then the CreateSessionDialog mounts over the board instead of switching into the running agent "s-1"
    And the dropdown closes and the bar de-selects exactly once (the execute path)

  @keyboard-navigation
  Scenario: Executing the 'u' menu-bar-help dialog's 'New Agent' row mounts the CreateSessionDialog
    Given an agent is running on session "s-1"
    And the 'u' menu-bar-help dialog is open with '. New Agent' (the first row) highlighted
    When I press Enter on the '. New Agent' row
    Then the menu-bar-help dialog is closed
    And the CreateSessionDialog mounts over the board instead of switching into the running agent "s-1"

  @mux
  Scenario: Executing the mux top bar's Kanban 'New Agent' row mounts the CreateSessionDialog over the mux grid
    Given mux mode is active with the board's top bar painted
    And an agent is running on session "s-1"
    And the mux top bar's Kanban dropdown is open with the cursor on the 'New Agent' row
    When I press Enter on the 'New Agent' row (row 0)
    Then the CreateSessionDialog mounts over the mux grid
    And the active view remains the mux (confirming the dialog later is what flips, not the row pick)
