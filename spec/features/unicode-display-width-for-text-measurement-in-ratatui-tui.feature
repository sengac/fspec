@done
@RPC-431
Feature: Unicode display width for text measurement in ratatui TUI
  """
  Replace .chars().count() with unicode_width::UnicodeWidthStr::width() in all display-width measurement functions across rust/fspec-tui/src/. The unicode-width crate is already in the workspace. Text selection/copy, animation, and secret masking intentionally keep .chars().count() since they operate on character counts, not display width.
  """

  Background: 
    Given the fspec ratatui TUI is running
    And the terminal supports wide Unicode characters

  Scenario: Markdown table column width accounts for wide Unicode characters
    Given a markdown table with a cell containing the emoji "✅" (display width 2)
    When the table is rendered by push_table_block
    Then the column width for that cell must be 2 (display width) not 1 (char count)
    And the cell padding must align correctly with other width-2 content like "OK"

  Scenario: Dialog title centering accounts for wide Unicode characters
    Given a dialog with a title containing the CJK character "中" (display width 2)
    When the dialog is rendered by dialog_theme inner_content_width
    Then the dialog must center correctly using display width 2 not char count 1

  Scenario: Text wrapping respects Unicode display width
    Given a line of emoji characters "✅✅✅✅" (display width 8)
    When the line is wrapped by wrap_to_width with a width of 4
    Then the line must wrap after 2 emoji characters (display width 4)
    And the first wrapped row must contain exactly 2 emoji characters

  Scenario: Dialog button row centering accounts for wide Unicode characters
    Given a confirmation dialog with button spans containing wide characters
    When the button row is rendered
    Then the button row must center using display width not char count

  Scenario: Text truncation respects Unicode display width
    Given a string "中文字" (display width 6) that needs to fit in a width of 4
    When the string is truncated by truncate_to
    Then the truncated output must fit within display width 4
    And the truncation must not split wide characters mid-way

  Scenario: Text selection copy preserves character count not display width
    Given a selected text region containing wide Unicode characters
    When the text is copied via slice_chars
    Then the copied text must contain the correct characters
    And the character count boundary must be used (not display width)

  Scenario: Secret masking uses character count not display width
    Given a secret string containing wide Unicode characters
    When the secret is masked by mask_secret
    Then each character must be replaced by exactly one bullet point
    And the masked output must have the same character count as the input

  Scenario: Animation frame counting uses character count not display width
    Given captured text containing wide Unicode characters
    When the input transition animation advances
    Then each frame must reveal or consume one character
    And the frame count must equal the character count not display width
