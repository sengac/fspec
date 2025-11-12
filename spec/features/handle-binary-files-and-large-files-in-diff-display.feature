@done
@git
@diff-display
@checkpoint
@tui
@high
@TUI-030
Feature: Handle binary files and large files in diff display

  """
  Uses isomorphic-git for file content access. Binary detection checks for null bytes in file content. Line counting uses newline character counting. Diff display uses react-diff-viewer. Truncation applies to diff lines, not file lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Binary files must not show diff content, only a message indicating it's binary
  #   2. Files with more than 20,000 lines must be truncated at 20,000 lines
  #   3. Truncated files must show a message with total line count
  #   4. Binary file detection must check file content, not just extension
  #   5. Normal text files under 20,000 lines must show complete diff
  #
  # EXAMPLES:
  #   1. User views checkpoint with image file (PNG), sees '[Binary file - no diff available]' instead of garbled text
  #   2. User views checkpoint with PDF file, sees binary file message
  #   3. User views checkpoint with 50,000-line log file, sees first 20,000 lines plus '[File truncated - showing first 20,000 of 50,000 lines]'
  #   4. User views checkpoint with normal 500-line source file, sees complete diff
  #   5. User views checkpoint with executable binary, sees binary file message
  #
  # ========================================

  Background: User Story
    As a developer using checkpoint viewer
    I want to view diffs for all file types without performance issues or display errors
    So that I can quickly understand changes in checkpoints regardless of file type or size

  Scenario: Display binary file message for PNG image
    Given I have a checkpoint that includes changes to a PNG image file
    When I view the diff for the PNG file in the checkpoint viewer
    Then I should see the message '[Binary file - no diff available]'
    And I should not see garbled binary content in the diff view


  Scenario: Truncate large file with over 20,000 lines
    Given I have a checkpoint that includes changes to a log file with 50,000 lines
    When I view the diff for the log file in the checkpoint viewer
    Then I should see the first 20,000 lines of the diff
    And I should see the message '[File truncated - showing first 20,000 of 50,000 lines]'


  Scenario: Display complete diff for normal-sized text file
    Given I have a checkpoint that includes changes to a source file with 500 lines
    When I view the diff for the source file in the checkpoint viewer
    Then I should see the complete diff without truncation
    And I should not see any truncation message


  Scenario: Display binary file message for executable binary
    Given I have a checkpoint that includes changes to an executable binary file
    When I view the diff for the executable in the checkpoint viewer
    Then I should see the message '[Binary file - no diff available]'
    And I should not see garbled binary content in the diff view

