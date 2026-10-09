@done
@navigation
@rpc
@tui
@RPC-395
Feature: Board '.' key starts new agent
  """
  Uses crossterm KeyCode::Char('.') arm in board.rs handle_event, emitting
  Action::OpenCreateSessionDialog { preselect: None } (BUG-203 supersedes the
  original RPC-395 emission — Action::OpenAgentView(selected_session) —
  which mirrored the Shift+Right handler: the '.' key is a 'New Agent'
  gesture that ALWAYS mounts the CreateSessionDialog; resuming / re-entering
  a session is the Shift+Right CYCLE gesture's job and is unchanged).
  Update keybinding_shortcuts.rs line 32 string + doc comments (lines 8,
  10-11), and update snapshot .snap files (help_dialog_dismissed,
  repl_bootstrap_rpc012, help_dialog_visible) plus view_board_unit_rpc015.rs
  assertion from '/ New Agent' to '. New Agent'
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing '.' on the board mounts the CreateSessionDialog (BUG-203:
  #      the 'New Agent' gesture ALWAYS starts a new agent — it never jumps
  #      into / resumes the selected work unit's session or the first open
  #      session; that is the Shift+Right CYCLE gesture's unchanged job)
  #   2. The '.' key handler is modifier-free (no Ctrl/Shift) so it does not conflict with reserved chords
  #   3. The board header hint text reads '. New Agent' instead of '/ New Agent'
  #
  # EXAMPLES:
  #   1. User has a work unit selected on the board and presses '.', the CreateSessionDialog mounts over the board (the AgentView is NOT opened for that work unit's session)
  #   2. The board header row is rendered and displays '. New Agent' as the last chord segment
  #   3. User presses '.' while no work unit is selected, the CreateSessionDialog still mounts (the first open session is NOT resumed — BUG-203)
  #
  # ========================================
  Background: User Story
    As a fspec TUI user on the Kanban board
    I want to press the '.' (period) key to start a new agent
    So that I have a fast single-key shortcut for opening the agent view, matching the TypeScript board's '/' behavior

  # NOTE (BUG-203 supersedes the original RPC-395 scenarios 1 & 2): the '.'
  # gesture now ALWAYS mounts the CreateSessionDialog (the R8/RPC-097
  # OpenAgentView emission is gone — session resume is the Shift+Right CYCLE
  # gesture's unchanged job). See the BUG-203 feature.

  Scenario: Pressing '.' with a selected work unit mounts the CreateSessionDialog
    Given a BoardStore containing AUTH-001 in backlog with the focused column "backlog" and selected index 0
    When the user presses the key '.'
    Then BoardView emits Action::OpenCreateSessionDialog { preselect: None }

  Scenario: Pressing '.' with no work unit selected still mounts the CreateSessionDialog
    Given an empty BoardStore with no work units
    When the user presses the key '.'
    Then BoardView emits Action::OpenCreateSessionDialog { preselect: None } (the first open session is NOT resumed — BUG-203)

  Scenario: The board header hint row displays '. New Agent'
    Given a BoardStore with any selection state
    When the App renders BoardView against a 120x24 TestBackend
    Then the header row 3 shows the 2-zone menu bar Zone A items (Actions Help)
    And the rendered buffer does not contain the substring "/ New Agent"
    # MENU-002 R1 supersedes: the header chord (and the later u Actions hint)
    # became the live 2-zone menu bar; New Agent lives in the Actions dropdown.
