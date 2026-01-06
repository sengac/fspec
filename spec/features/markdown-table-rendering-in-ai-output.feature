@done
@agent-modal
@tui
@TUI-044
Feature: Markdown Table Rendering in AI Output

  """
  Table rendering utility function formatMarkdownTables() processes content and replaces raw table markdown with aligned box-drawing output
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Table rendering occurs only after the assistant turn is complete (isStreaming: false)
  #   2. Must detect and parse standard GFM markdown tables with pipe delimiters and header separator row
  #   3. Must respect column alignment specifiers (:--- left, :---: center, ---: right)
  #   4. Column widths calculated based on max content width in each column
  #   5. Use the marked library (already a dependency) to parse markdown and extract table tokens
  #   6. Tables rendered with box-drawing characters for borders (┌─┬─┐ │ ├─┼─┤ └─┴─┘)
  #
  # EXAMPLES:
  #   1. Input: '| Name | Age |\n|---|---|\n| Alice | 30 |' renders as aligned table with box borders
  #   2. Input: '| Left | Center | Right |\n|:---|:---:|---:|\n| A | B | C |' renders with correct alignment in each column
  #   3. Table with varying content widths: column widths adjust to fit longest cell in each column
  #   4. Tables mixed with other text: only table portions get rendered, surrounding text stays as-is
  #   5. Table header row rendered in bold
  #
  # ========================================

  Background: User Story
    As a user viewing AI agent output in the TUI
    I want to see markdown tables rendered with proper column alignment
    So that I can read tabular data clearly without misaligned columns

  Scenario: Simple table with two columns renders with box borders
    Given the AI assistant has completed a response containing a markdown table
    When the streaming is marked complete (isStreaming: false)
    Then the table is rendered with box-drawing characters
    And columns are aligned based on content width
    And the header row is rendered in bold


  Scenario: Table with alignment specifiers renders with correct alignment
    Given the AI response contains a table with alignment specifiers
    When the streaming completes
    Then columns with :--- are left-aligned
    And columns with :---: are center-aligned
    And columns with ---: are right-aligned


  Scenario: Tables mixed with other content preserves surrounding text
    Given the AI response contains text before and after a markdown table
    When the streaming completes
    Then only the table portion is rendered with box borders
    And the surrounding text remains unchanged

