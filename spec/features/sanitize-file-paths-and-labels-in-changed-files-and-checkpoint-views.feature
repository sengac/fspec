@TUI-105
Feature: Sanitize file paths and labels in Changed Files and Checkpoint views
  """
  Apply sanitization in file_row() for file.path and file.change_type before truncate_path() and Span::styled(). Apply sanitization in checkpoint_line() for the checkpoint label before truncate_path(). Both depend on sanitize_for_terminal() being accessible from the views/ module (same shared module as TUI-104).
  """

  Background: User Story
    As a TUI user
    I want to view file paths and checkpoint labels in the Changed Files and Checkpoint views
    So that see clean terminal output without corrupted display from unusual characters

  Scenario: File paths with unusual characters display cleanly in the Changed Files view
    Given I have a changed file with a path containing control characters or ANSI sequences
    When I open the Changed Files view
    Then the file list displays the path without control characters
    And the terminal display is not corrupted

  Scenario: Checkpoint labels with special characters display cleanly in the Checkpoint view
    Given I have a checkpoint with a label containing control characters or ANSI sequences
    When I open the Checkpoint view
    Then the checkpoint list displays the label without control characters
    And the terminal display is not corrupted
