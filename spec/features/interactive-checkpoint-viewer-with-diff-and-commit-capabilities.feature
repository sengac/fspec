@tui
@ui
@checkpoint
@board-view
@GIT-004
Feature: Interactive checkpoint viewer with diff and commit capabilities

  """
  Two new React components: CheckpointViewer and ChangedFilesViewer, both using VirtualList for scrolling. Dual-pane flexbox layout (30% file list, 70% diff). Tab key switches focus between panes. Uses isomorphic-git for checkpoint file listing and diff generation. Integrates into BoardView as new view modes accessed via C and F keys.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Integrated into kanban board TUI using C key (checkpoints) and F key (changed files)
  #   2. View must use unified keybindings consistent with other TUI views (e.g., j/k for scrolling, q to quit)
  #   3. View must display two panes: file list (left) and diff view (right), both independently scrollable
  #   4. Layout must use flexbox with both file list and diff area using flex to grow proportionally
  #   5. Scrolling implementation must follow the same pattern as work unit details view in TUI
  #   6. C key opens checkpoint files view, F key opens changed files view
  #   7. No vim-style keybindings (j/k/g/G) - use only arrow keys, PgUp/PgDn, Home/End for navigation
  #
  # EXAMPLES:
  #   1. User presses C in kanban board → checkpoint files view opens with file list on left and diff on right
  #   2. User presses F in kanban board → changed files view opens with file list on left and diff on right
  #   3. User uses j/k to scroll through file list, selects file, then uses j/k to scroll through diff content
  #   4. User presses q in checkpoint/changed files view → returns to kanban board
  #   5. File list and diff view both resize proportionally when terminal window resizes (flexbox behavior)
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should users access the interactive checkpoint viewer? New command, integrated into kanban board, or integrated into show-work-unit?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to view and navigate checkpoint/changed files with diffs
    So that I can quickly understand what changed and restore work if needed

  Scenario: Open checkpoint files view with C key
    Given I am viewing the fspec kanban board
    And there are checkpoints available for the current work unit
    When I press the C key
    Then the checkpoint files view should open
    And I should see a file list pane on the left
    And I should see a diff pane on the right
    And the file list should be focused initially

  Scenario: Open changed files view with F key
    Given I am viewing the fspec kanban board
    And there are changed files in the working directory
    When I press the F key
    Then the changed files view should open
    And I should see a file list pane on the left showing staged and unstaged files
    And I should see a diff pane on the right
    And the file list should be focused initially

  Scenario: Navigate file list with arrow keys
    Given I am in the checkpoint files view
    And the file list contains multiple files
    And the file list pane is focused
    When I press the down arrow key
    Then the selection should move to the next file
    And the diff pane should update to show the selected file's diff

  Scenario: Scroll through diff content with arrow keys
    Given I am in the checkpoint files view
    And a file is selected in the file list
    And the diff pane is focused
    When I press the down arrow key
    Then the diff content should scroll down one line
    When I press PgDn
    Then the diff content should scroll down one page

  Scenario: Switch focus between panes with Tab key
    Given I am in the checkpoint files view
    And the file list pane is focused
    When I press the Tab key
    Then the diff pane should be focused
    And the focused pane should be visually indicated
    When I press Tab again
    Then the file list pane should be focused again

  Scenario: Return to kanban board with ESC key
    Given I am in the checkpoint files view
    When I press the ESC key
    Then I should return to the kanban board view

  Scenario: Dual-pane layout with flexbox sizing
    Given I am in the checkpoint files view
    When the terminal window is resized
    Then both the file list and diff panes should resize proportionally
    And the file list should maintain approximately 30% width
    And the diff pane should take the remaining space

  Scenario: Empty state for no checkpoints
    Given I am viewing the fspec kanban board
    And there are no checkpoints available
    When I press the C key
    Then the checkpoint files view should open
    And I should see "No checkpoints available" in the file list
    And the diff pane should show "Select a checkpoint to view files"

  Scenario: Empty state for no changed files
    Given I am viewing the fspec kanban board
    And there are no changed files in the working directory
    When I press the F key
    Then the changed files view should open
    And I should see "No changed files" in the file list
    And the diff pane should show "No changes to display"
