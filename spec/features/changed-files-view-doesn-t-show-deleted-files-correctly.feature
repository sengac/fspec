@done
@high
@tui
@git-integration
@diff-viewer
@BUG-069
Feature: Changed files view doesn't show deleted files correctly

  """
  This bug fix modifies the TUI changed files view component to correctly display deleted files with proper status indicators and diff panel messages. The fix involves: (1) Color-coding the 'D' status indicator in red following git conventions, (2) Displaying 'File was deleted' message in the diff panel instead of 'No changes to display' when a deleted file is selected. Implementation will touch the file status display component and the diff panel rendering logic.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Deleted files should show 'D' status indicator in red color
  #   2. Diff panel should show 'File was deleted' message for deleted files instead of 'No changes to display'
  #   3. Status indicators should follow git conventions: A (added/green), M (modified/yellow), D (deleted/red), R (renamed/cyan)
  #
  # EXAMPLES:
  #   1. User deletes a file (rm somefile.txt), opens changed files view (F key), sees 'D somefile.txt' in red
  #   2. User deletes and stages file (git rm somefile.txt), changed files view shows staged 'D somefile.txt' in red
  #   3. User selects deleted file in changed files view, diff panel shows 'File was deleted' instead of 'No changes to display'
  #
  # ========================================

  Background: User Story
    As a developer using fspec
    I want to view deleted files in the changed files view
    So that I can see which files were deleted and review the deletion in the diff panel

  Scenario: View unstaged deleted file in changed files view
    Given I have deleted a file "example.txt" without staging it
    When I open the changed files view with the F key
    Then I should see "D example.txt" displayed in red color
    And the status indicator should be "D" for deleted

  Scenario: View staged deleted file in changed files view
    Given I have deleted and staged a file "test.ts" using "git rm test.ts"
    When I open the changed files view with the F key
    Then I should see "D test.ts" displayed in red color under staged changes
    And the status indicator should be "D" for deleted

  Scenario: View diff panel for deleted file
    Given I have deleted a file "config.json"
    And I have opened the changed files view with the F key
    When I select the deleted file "config.json"
    Then the diff panel should display "File was deleted"
    And the diff panel should not display "No changes to display"
