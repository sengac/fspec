@TUI-104
Feature: Sanitize diff output in Changed Files and Checkpoint views
  """
  Move sanitize_for_terminal() from store/agent_view/sanitize.rs to a shared location (e.g., utils/sanitize.rs or crate root) so views/diff_common can import it. Apply sanitization inside diff_line() in views/diff_common/diff_render.rs before creating the Span.
  """

  Background: User Story
    As a TUI user
    I want to view file diffs in the Changed Files and Checkpoint views
    So that see clean terminal output without ANSI escape sequences trashing the display

  Scenario: Diff lines with ANSI color codes display cleanly in the Changed Files view
    Given I have a changed file whose diff contains ANSI escape sequences like "\x1b[31m" for colored content
    When I open the Changed Files view and select that file
    Then the diff pane displays the text content without ANSI escape sequences
    And the terminal display is not corrupted by escape sequences

  Scenario: Diff lines with tab characters display with consistent spacing in the Checkpoint view
    Given I have a checkpoint with a file diff that contains tab characters
    When I open the Checkpoint view and select the file
    Then the diff pane displays two spaces instead of each tab character
    And the terminal display maintains consistent visual width

  Scenario: Diff lines with carriage returns display without line overwriting
    Given I have a changed file whose diff contains carriage return characters
    When I open the Changed Files view and select that file
    Then the diff pane displays the content without line overwriting
    And each line appears on its own row in the terminal

  Scenario: Diff lines with control characters display without corrupted rendering
    Given I have a checkpoint with a file diff that contains control characters like NUL or backspace
    When I open the Checkpoint view and select the file
    Then the diff pane displays the content with control characters removed
    And the terminal display is not corrupted
