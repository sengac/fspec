@done
@ui-enhancement
@tui
@medium
@TUI-007
Feature: Container Focus Indication with Headings

  """
  Modifies CheckpointViewer.tsx and ChangedFilesViewer.tsx to add container headings with focus-based styling. Uses Ink's Text component with backgroundColor and bold props to show focus state.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Container headings must show green background with black text when focused
  #   2. Container headings must show bold white text with no background when not focused
  #   3. Top-level page headings must be completely removed from both CheckpointViewer and ChangedFilesViewer
  #   4. Container headings must appear inside each container (checkpoint list, file list, diff pane)
  #   5. Tab key navigation must update heading background to show which container is currently focused
  #   6. Both CheckpointViewer and ChangedFilesViewer must have identical heading behavior and styling
  #
  # EXAMPLES:
  #   1. User opens CheckpointViewer - checkpoint list heading shows green background with black text 'Checkpoints', file list and diff headings show bold white text
  #   2. User presses Tab once in CheckpointViewer - checkpoint list heading changes to bold white text, file list heading shows green background with black text 'Files'
  #   3. User presses Tab twice in CheckpointViewer - diff pane heading shows green background with black text 'Diff', other headings show bold white text
  #   4. User opens ChangedFilesViewer - no top-level 'Changed Files: X staged, Y unstaged' heading, file list heading shows green background with black text 'Files'
  #   5. User opens CheckpointViewer - no top-level 'Checkpoints: X available' heading visible, checkpoint list has heading 'Checkpoints' with green background
  #
  # QUESTIONS (ANSWERED):
  #   Q: Which specific containers exist in the checkpoint view and change files view? (You mentioned file list and diff - are there others?)
  #   A: true
  #
  #   Q: What color should the heading background be when a container is focused? What about when it's not focused?
  #   A: true
  #
  #   Q: What text should each container heading display? (e.g., 'File List', 'Diff View', etc.)
  #   A: true
  #
  #   Q: You mentioned removing the top-level page heading completely - what heading is this currently and where does it appear? Should it be completely removed or replaced with something else?
  #   A: true
  #
  #   Q: Should the focused heading show any additional information besides the colored background? (e.g., keyboard shortcuts, help text, focus indicator icon?)
  #   A: true
  #
  #   Q: Should the checkpoint view and change files view have identical behavior and styling for the container headings, or are there any differences between them?
  #   A: true
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to see which container is currently focused when navigating with Tab
    So that I can navigate between containers more effectively

  Scenario: CheckpointViewer initial focus on checkpoint list
    Given I have opened the CheckpointViewer
    When the view is rendered
    Then the checkpoint list heading should display "Checkpoints" with green background and black text
    And the file list heading should display "Files" with bold white text and no background
    And the diff pane heading should display "Diff" with bold white text and no background
    And the top-level "Checkpoints: X available" heading should not be visible

  Scenario: Tab navigation to file list in CheckpointViewer
    Given I have opened the CheckpointViewer with focus on checkpoint list
    When I press the Tab key once
    Then the checkpoint list heading should display "Checkpoints" with bold white text and no background
    And the file list heading should display "Files" with green background and black text
    And the diff pane heading should display "Diff" with bold white text and no background

  Scenario: Tab navigation to diff pane in CheckpointViewer
    Given I have opened the CheckpointViewer with focus on checkpoint list
    When I press the Tab key twice
    Then the checkpoint list heading should display "Checkpoints" with bold white text and no background
    And the file list heading should display "Files" with bold white text and no background
    And the diff pane heading should display "Diff" with green background and black text

  Scenario: ChangedFilesViewer initial focus on file list
    Given I have opened the ChangedFilesViewer
    When the view is rendered
    Then the file list heading should display "Files" with green background and black text
    And the diff pane heading should display "Diff" with bold white text and no background
    And the top-level "Changed Files: X staged, Y unstaged" heading should not be visible

  Scenario: Tab navigation between file list and diff in ChangedFilesViewer
    Given I have opened the ChangedFilesViewer with focus on file list
    When I press the Tab key once
    Then the file list heading should display "Files" with bold white text and no background
    And the diff pane heading should display "Diff" with green background and black text
