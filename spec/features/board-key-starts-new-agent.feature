@done
@navigation
@rpc
@tui
@RPC-395
Feature: Board '.' key starts new agent
  """
  Uses crossterm KeyCode::Char('.') arm in board.rs handle_event, emitting Action::OpenAgentView(self.selected_session(store)) mirroring the Shift+Right handler at board.rs:114-117
  Update keybinding_shortcuts.rs line 32 string + doc comments (lines 8, 10-11), and update snapshot .snap files (help_dialog_dismissed, repl_bootstrap_rpc012, help_dialog_visible) plus view_board_unit_rpc015.rs assertion from '/ New Agent' to '. New Agent'
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing '.' on the board opens the AgentView using the selected work unit's session (same behavior as Shift+Right)
  #   2. The '.' key handler is modifier-free (no Ctrl/Shift) so it does not conflict with reserved chords
  #   3. The board header hint text reads '. New Agent' instead of '/ New Agent'
  #
  # EXAMPLES:
  #   1. User has a work unit selected on the board and presses '.', the AgentView opens for that work unit's session
  #   2. The board header row is rendered and displays '. New Agent' as the last chord segment
  #   3. User presses '.' while no work unit is selected, the AgentView still opens with no attached session (mirrors Shift+Right)
  #
  # ========================================
  Background: User Story
    As a fspec TUI user on the Kanban board
    I want to press the '.' (period) key to start a new agent
    So that I have a fast single-key shortcut for opening the agent view, matching the TypeScript board's '/' behavior

  Scenario: Pressing '.' with a selected work unit opens the AgentView for its session
    Given a BoardStore containing AUTH-001 in backlog with the focused column "backlog" and selected index 0
    When the user presses the key '.'
    Then BoardView emits an Action::OpenAgentView for the selected work unit's session

  Scenario: Pressing '.' with no work unit selected still opens the AgentView with no attached session
    Given an empty BoardStore with no work units
    When the user presses the key '.'
    Then BoardView emits an Action::OpenAgentView with no attached session

  Scenario: The board header hint row displays '. New Agent'
    Given a BoardStore with any selection state
    When the App renders BoardView against a 120x24 TestBackend
    Then the rendered buffer contains the substring ". New Agent"
    And the rendered buffer does not contain the substring "/ New Agent"
