@RPC-426
@rust
@agent-view
@tui
Feature: Agent input newline bindings — Ctrl+J universal fallback with Shift+Enter best-effort
  """
  Ctrl+J (Emacs-style) is the universal newline binding — works on every terminal because it uses character codes, not modifier detection. Shift+Enter is best-effort (only on terminals with kitty keyboard enhancement). Alt+Enter is legacy fallback.
  """

  Background: User Story
    As a TUI user
    I want to insert newlines in the agent input field
    So that I can compose multi-line messages reliably on any terminal

  Scenario: Ctrl+J inserts a newline and grows the input area
    Given the agent input contains "hello" with the cursor at the end
    When I press Ctrl+J
    Then the input buffer contains "hello" followed by a newline
    And the cursor is at the start of the second line
    And the input area reports 2 visible rows

  Scenario: Ctrl+J mid-word splits the line at cursor position
    Given the agent input contains "hello world" with the cursor between "hello " and "world"
    When I press Ctrl+J
    Then the input buffer contains "hello " on the first line and "world" on the second line
    And the cursor is at the start of the second line
    And the input area reports 2 visible rows

  Scenario: Plain Enter submits the multi-line buffer and resets the input
    Given the agent input contains 3 lines composed with Ctrl+J
    When I press plain Enter with no modifiers
    Then the submitted value is the 3 lines joined by newline characters
    And the input buffer is empty
    And the input area reports 1 visible row

  Scenario: Shift+Enter inserts a newline on enhanced terminals
    Given the agent input contains "first line" with the cursor at the end
    And the terminal supports keyboard enhancement
    When I press Shift+Enter
    Then a newline is inserted at the cursor
    And the buffer is not submitted
    And the input area reports 2 visible rows

  Scenario: Shift+Enter submits on non-enhanced terminals (modifier eaten)
    Given the agent input contains "hello" with the cursor at the end
    And the terminal does not support keyboard enhancement
    When I press Shift+Enter
    Then the buffer is submitted as "hello"
    And the input buffer is empty

  Scenario: Alt+Enter inserts a newline as legacy fallback
    Given the agent input contains "first line" with the cursor at the end
    When I press Alt+Enter
    Then a newline is inserted at the cursor
    And the buffer is not submitted
    And the input area reports 2 visible rows

  Scenario: Ctrl+J is swallowed while the session is compacting
    Given the agent input contains "draft"
    And the session is compacting
    When I press Ctrl+J
    Then the key is consumed without modifying the buffer
    And the input buffer still contains "draft"
    And the input area reports 1 visible row

  Scenario: Input placeholder shows Ctrl+J as the primary newline hint
    Given the agent input is empty
    When the placeholder is rendered
    Then the placeholder text contains "Ctrl+J"
