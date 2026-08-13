@done
@tui-component
@scrollback
@markdown
@agent-view
@RPC-370
Feature: Render markdown tables with box-drawing characters in Rust chat view
  """
  Column width uses char-count as the visual-width proxy, consistent with the existing text_wrap.rs approach in the Rust port (full East-Asian-width handling deferred, matching current scrollback behavior)
  Implementation modifies format_markdown_tables in rust/fspec-tui/src/store/agent_view/markdown_tables.rs, which is already called from chunk_processor.rs::handle_done at Done finalization — no new call site needed, the box-drawing output replaces the existing pipe-padding output
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A markdown pipe-table (header row, separator row of dashes, zero or more data rows) is rendered as a Unicode box-drawing grid with top border ┌─┬─┐, header row, header separator ├─┼─┤, data rows, and bottom border └─┴─┘
  #   2. Each column is padded to the visual width of its widest cell (header or data), measured by character count, so all borders align vertically
  #   3. Per-column alignment is derived from the separator row colons: ':---' is left, ':---:' is center, '---:' is right, and '---' defaults to left; cell text is padded accordingly within its column
  #   4. Data rows with fewer cells than the header are padded with empty cells; extra cells beyond the header count are ignored
  #   5. Non-table text passes through unchanged, and text surrounding a table (before/after) is preserved with the table rendered in place
  #   6. A pipe block that has no dash separator row (not a real table) is left unchanged rather than being drawn as a grid
  #
  # EXAMPLES:
  #   1. Input '| col1 | col2 |\n|---|---|\n| a | bb |' renders a 2-column box-drawing grid where the col2 column is wide enough for 'bb' and both border rows align
  #   2. Input with separator '|:---|:---:|---:|' left-aligns column 1, center-aligns column 2, and right-aligns column 3 within the padded cells
  #   3. A data row with fewer cells than the header (e.g. '| a |' under a 2-column header) renders with the missing cell shown as blank, keeping the grid rectangular
  #   4. Plain prose 'hello world\nnot a table' is returned byte-for-byte unchanged with no box-drawing characters added
  #   5. A table embedded in prose ('Here:\n| a | b |\n|---|---|\n| 1 | 2 |\nDone.') renders the grid in place while keeping the 'Here:' and 'Done.' lines
  #
  # ASSUMPTIONS:
  #   1. Bold/ANSI styling of header cells is out of scope because the Rust scrollback wrap path renders chunk text as single-color plain spans without an embedded-ANSI parser
  #   2. Code-fence-wrapped tables (the TS looksLikeTable / code-token path) are out of scope because the Rust Done-finalization path has no markdown lexer; only contiguous pipe-table blocks are converted
  #
  # ========================================
  Background: User Story
    As a user reading AI responses in the Rust chat view
    I want to see markdown tables rendered as aligned box-drawing grids
    So that I can read tabular data as easily as in the TypeScript chat view

  Scenario: Simple two-column table renders as an aligned box-drawing grid
    Given an AI response containing the markdown table "| col1 | col2 |\n|---|---|\n| a | bb |"
    When the response is finalized and formatted for the chat view
    Then the output contains a top border line starting with "┌" and ending with "┐"
    And the output contains a header separator line starting with "├" and ending with "┤"
    And the output contains a bottom border line starting with "└" and ending with "┘"
    And every box-drawing border row has the same display width

  Scenario: Colon separators set per-column left, center, and right alignment
    Given an AI response containing the markdown table "| a | b | c |\n|:---|:---:|---:|\n| x | y | z |"
    When the response is finalized and formatted for the chat view
    Then column 1 cells are left-aligned within their padded width
    And column 2 cells are center-aligned within their padded width
    And column 3 cells are right-aligned within their padded width

  Scenario: Data row with fewer cells than the header keeps the grid rectangular
    Given an AI response containing the markdown table "| h1 | h2 |\n|---|---|\n| a |"
    When the response is finalized and formatted for the chat view
    Then the missing second cell is rendered as a blank padded cell
    And every rendered data row has the same display width as the header row

  Scenario: Non-table prose passes through unchanged
    Given an AI response containing the text "hello world\nnot a table"
    When the response is finalized and formatted for the chat view
    Then the output equals the input byte-for-byte
    And no box-drawing characters are added

  Scenario: A table embedded in prose is rendered in place with surrounding lines kept
    Given an AI response containing the text "Here:\n| a | b |\n|---|---|\n| 1 | 2 |\nDone."
    When the response is finalized and formatted for the chat view
    Then the line "Here:" is preserved before the grid
    And the line "Done." is preserved after the grid
    And the output contains a box-drawing top border line

  Scenario: A pipe block with no separator row is left unchanged
    Given an AI response containing the text "| a | b |\n| c | d |"
    When the response is finalized and formatted for the chat view
    Then the output equals the input byte-for-byte
    And no box-drawing characters are added

  Scenario: Rendered table grid survives the scrollback wrap path with padding preserved
    given a rendered grid row "│ Name  │ Role     │ Location  │" that fits within the viewport width
    When the chat view wraps the row to the viewport width
    Then the wrapped row equals the input with every internal column padding space preserved
    Then no column padding is collapsed to a single space
