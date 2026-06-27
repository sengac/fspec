@done
@diff-viewer
@git-integration
@tui
@RPC-354
Feature: File Changes view in Rust TUI (port ChangedFilesViewer)

  """
  Reuses the existing codelet/git primitives and the established Navigator/Action transport pattern (mirrors checkpoint_counts RPC-015); does not reimplement git logic
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Pressing F (or f) on the Rust board opens a full-screen Changed Files view
  #   2. The Changed Files view shows the real staged and unstaged working-tree changes supplied by the TUI transport
  #   3. Pressing Esc in the Changed Files view returns to the board
  #
  # EXAMPLES:
  #   1. In a repo with one modified file, pressing F shows that file in the list and its diff in the diff pane
  #   2. After opening the Changed Files view, pressing Esc returns the user to the board
  #   3. In a clean repo with no changes, pressing F shows an empty-state message and Esc returns to the board
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to press F on the board to open a Changed Files view showing my staged and unstaged git changes with diffs
    So that I can review my working-tree changes without leaving the Rust TUI, matching the old TypeScript board

  @integration
  Scenario: Pressing F on the board opens the Changed Files view with the changed file and its diff
    Given a workspace whose git working tree has one modified file
    And the navigator is showing the board view
    When the user presses the "F" key
    Then the active view becomes the Changed Files view
    And the file list contains the modified file
    And the diff pane shows the unified diff for the modified file

  @integration
  Scenario: Pressing Esc in the Changed Files view returns to the board
    Given the navigator is showing the Changed Files view
    When the user presses the "Esc" key
    Then the active view becomes the board view

  @integration
  Scenario: Opening the Changed Files view in a clean repo shows an empty state
    Given a workspace whose git working tree has no changes
    And the navigator is showing the board view
    When the user presses the "F" key
    Then the active view becomes the Changed Files view
    And the view shows an empty-state message that there are no changed files
    When the user presses the "Esc" key
    Then the active view becomes the board view
