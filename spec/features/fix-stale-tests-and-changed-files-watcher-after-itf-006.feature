@tui
@board-visualization
@high
@ITF-007
Feature: Fix stale tests and changed files watcher after ITF-006

  """
  ITF-006 replaced GitStashesPanel with CheckpointPanel, changing display format from 'Git Stashes (X)' to 'Checkpoints: X Manual, Y Auto'. This broke 11 existing tests that expect old text. Tests must be updated to expect new checkpoint format. Changed files watcher uses getStagedFiles/getUnstagedFiles utilities which need to be called after git operations to refresh counts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All tests expecting 'Git Stashes' text must be updated to expect 'Checkpoints' instead
  #   2. Changed files watcher must refresh counts immediately after git commit operations
  #   3. Failing tests: BoardView-git-watcher-fix (4), BoardView-interactive-kanban (2), BoardView-realtime-updates (2), BoardView-table-layout (2), BoardView-git-context-work-unit-details (1) - total 11 tests
  #
  # EXAMPLES:
  #   1. After git commit, changed files panel shows '14 staged, 0 unstaged' but should show '0 staged, 0 unstaged' (clean working directory)
  #   2. Test expects 'Git Stashes (2)' but component now displays 'Checkpoints: X Manual, Y Auto'
  #   3. Test expects keybinding 'S View Stashes' but panel now shows 'C View Checkpoints'
  #
  # ========================================

  Background: User Story
    As a developer running tests
    I want to have all tests pass after ITF-006 checkpoint integration
    So that I can verify the TUI works correctly with the new checkpoint system

  Scenario: Update tests expecting 'Git Stashes' text to expect 'Checkpoints'
    Given 11 tests fail after ITF-006 integration
    And tests expect old text format "Git Stashes (2)"
    When I update tests to expect new format "Checkpoints: X Manual, Y Auto"
    Then all 11 tests should pass with updated expectations
    And tests should verify checkpoint panel displays correct format

  Scenario: Update tests expecting 'S View Stashes' keybinding to expect 'C View Checkpoints'
    Given tests check for keybinding text "S View Stashes"
    And ITF-006 changed keybindings to "C View Checkpoints ◆ F View Changed Files"
    When I update test expectations to match new keybinding text
    Then tests should verify correct keybinding display
    And "S View Stashes" should not appear in output

  Scenario: Fix changed files watcher to refresh counts after git commit
    Given changed files panel shows "14 staged, 0 unstaged"
    And a git commit was just made (working directory is clean)
    When changed files watcher detects git operation
    Then panel should refresh and show "0 staged, 0 unstaged"
    And counts should update immediately without manual refresh
