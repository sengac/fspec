@done
@agent-integration
@tui
@input-components
@TUI-039
Feature: Multi-line text input with cursor navigation for AgentView

  """
  Create new MultiLineInput.tsx component in src/tui/components/, AgentView.tsx imports and uses MultiLineInput replacing SafeTextInput, State managed via useMultiLineInput hook in src/tui/hooks/
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Cursor position must be tracked as (row, column) tuple within the text
  #   2. Left/Right arrow keys move cursor by one character, wrapping across lines
  #   3. Up/Down arrow keys move cursor to same column on adjacent line, clamping to line length
  #   4. Home/End keys move cursor to start/end of current line
  #   5. Alt+Left/Right moves cursor by word boundaries
  #   6. Backspace deletes character before cursor; at line start, merges with previous line
  #   7. Delete key removes character at cursor; at line end, merges with next line
  #   8. Alt+Backspace deletes the word before cursor
  #   9. Character insertion happens at cursor position, not at end of text
  #   10. The cursor must be visually rendered using inverse text style on the character under cursor
  #   11. Shift+Enter inserts a newline; Enter submits the message
  #   12. The component must be separate from AgentView.tsx - create MultiLineInput.tsx
  #   13. Maximum 5 visible lines; if content exceeds this, scroll to keep cursor visible
  #   14. Shift+Up/Down must be preserved for history navigation (passed through to parent)
  #
  # EXAMPLES:
  #   1. User types 'hello world', cursor is at end (row 0, col 11); pressing Left moves cursor to (0, 10) before 'd'
  #   2. User has 'hello|world' (cursor after 'o'), presses Right 5 times, cursor moves through 'world' and stops at end
  #   3. User has two lines: 'first line' and 'second', cursor at (0, 8); pressing Down moves to (1, 6) - end of 'second'
  #   4. User has cursor at start of line 2; pressing Backspace merges line 2 onto end of line 1
  #   5. User has 'hello world' with cursor after 'o'; pressing Alt+Backspace deletes 'hello ' leaving 'world'
  #   6. User has 'hello world' with cursor in middle; types 'X', and 'X' appears at cursor position, not at end
  #   7. User presses Shift+Enter in middle of line; line splits at cursor into two lines
  #   8. User presses Home on 'hello world' with cursor at end; cursor jumps to start of line
  #   9. User presses Alt+Right on 'hello world' with cursor at start; cursor jumps to end of 'hello'
  #   10. Cursor at end of line 1; pressing Right moves to start of line 2 (wrapping)
  #   11. Cursor at start of line 2; pressing Left moves to end of line 1 (wrapping)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should Ctrl+A/Ctrl+E also work as Home/End (Emacs-style keybindings)?
  #   A: No - do not use Emacs or Vim style bindings. Only Home/End and arrow keys for navigation.
  #
  #   Q: Should we support Ctrl+U (delete to line start) and Ctrl+K (delete to line end)?
  #   A: No - do not support Ctrl+U or Ctrl+K. Keep keybindings simple.
  #
  #   Q: Does the existing Shift+Up/Down for history navigation need to be preserved in the new component?
  #   A: Yes - preserve Shift+Up/Down for history navigation in the new component.
  #
  #   Q: What's the maximum number of lines allowed in the input before we need scrolling?
  #   A: Maximum 5 visible lines. If content exceeds 5 lines, implement scrolling to keep cursor visible.
  #
  # ========================================

  Background: User Story
    As a developer using fspec's AgentView
    I want to edit multi-line messages with full cursor navigation
    So that I can efficiently compose and modify complex prompts without retyping

  Scenario: Move cursor left within a line
    Given the input contains "hello world" with cursor at the end
    When I press the Left arrow key
    Then the cursor moves one position to the left


  Scenario: Move cursor right and stop at end
    Given the input contains "hello" with cursor after the 'o'
    When I press the Right arrow key
    Then the cursor remains at the end of the text


  Scenario: Move cursor down to shorter line clamps column
    Given the input contains two lines "first line" and "second" with cursor at column 8 on line 1
    When I press the Down arrow key
    Then the cursor moves to the end of line 2 at column 6


  Scenario: Backspace at line start merges lines
    Given the input contains two lines with cursor at the start of line 2
    When I press the Backspace key
    Then line 2 is merged onto the end of line 1
    And the cursor is positioned at the join point


  Scenario: Alt+Backspace deletes word before cursor
    Given the input contains "hello world" with cursor after "hello "
    When I press Alt+Backspace
    Then the input contains "world" with cursor at the start


  Scenario: Insert character at cursor position
    Given the input contains "helloworld" with cursor between "hello" and "world"
    When I type "X"
    Then the input contains "helloXworld"
    And the cursor is after the "X"


  Scenario: Shift+Enter inserts newline and splits line
    Given the input contains "helloworld" with cursor between "hello" and "world"
    When I press Shift+Enter
    Then the input contains two lines "hello" and "world"
    And the cursor is at the start of line 2


  Scenario: Home key moves cursor to line start
    Given the input contains "hello world" with cursor at the end
    When I press the Home key
    Then the cursor is at the start of the line


  Scenario: Alt+Right moves cursor to end of word
    Given the input contains "hello world" with cursor at the start
    When I press Alt+Right
    Then the cursor is at the end of "hello"


  Scenario: Right arrow at end of line wraps to next line
    Given the input contains two lines with cursor at the end of line 1
    When I press the Right arrow key
    Then the cursor moves to the start of line 2


  Scenario: Left arrow at start of line wraps to previous line
    Given the input contains two lines with cursor at the start of line 2
    When I press the Left arrow key
    Then the cursor moves to the end of line 1


  Scenario: Scroll viewport when content exceeds 5 lines
    Given the input contains 7 lines with cursor on line 1
    When I move the cursor to line 7
    Then the viewport scrolls to keep the cursor visible
    And only 5 lines are displayed at a time


  Scenario: Shift+Up triggers history navigation callback
    Given the multi-line input is active with history callback configured
    When I press Shift+Up
    Then the onHistoryPrev callback is invoked

