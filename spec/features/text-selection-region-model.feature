@done
@rust
@text-selection
@tui
@COPY-002
Feature: Text selection region model
  """
  New module rust/fspec-tui/src/mouse/selection.rs. Types: Cell { row: u16, col: u16 } and Selection { anchor: Cell, cursor: Cell }. A RowSpan { row: u16, start_col: u16, end_col: u16 } (end exclusive, half-open to match the existing rect_contains half-open convention).
  For multi-row spans the caller supplies a row width (content width) so the first/middle rows can extend to the row end. Method signature: fn spans(&self, row_width: u16) -> Vec<RowSpan>. Normalization: order (row, col) pairs lexicographically; if start==end return empty vec.
  Pure module: no crossterm, ratatui, or io imports beyond primitive types. Depends on nothing else in COPY. Consumed by COPY-004 (text reconstruction), COPY-005 (highlight), and held live by COPY-006. Unit tests assert the exact Vec<RowSpan> for each example; no I/O.
  half-open columns: a span cols 2..6 covers columns 2,3,4,5. Middle-row full spans are 0..row_width. This matches how COPY-004 will slice span.content and COPY-005 will style cells.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A selection is defined by an anchor cell (row, col) and a cursor cell (row, col)
  #   2. The region is normalized so start is the earlier cell in reading order (top-to-bottom, then left-to-right) regardless of whether the anchor or cursor came first
  #   3. A single-row selection yields exactly one span: (row, start_col, end_col)
  #   4. A multi-row selection yields: first row from start_col to row end, full middle rows, last row from row start to end_col (linewise semantics)
  #   5. A collapsed selection (anchor == cursor) yields an empty region (no spans)
  #   6. The model is pure geometry: it contains no mouse, rendering, clipboard, or scrollback knowledge
  #
  # EXAMPLES:
  #   1. Anchor (1,2) and cursor (1,6) on the same row yields one span (row 1, cols 2..6)
  #   2. Backwards drag: anchor (3,4) and cursor (1,2) normalizes to start (1,2), end (3,4)
  #   3. Anchor (1,3) to cursor (3,5) over rows of width 10 yields spans: (1, 3..10), (2, 0..10), (3, 0..5)
  #   4. Anchor equals cursor (2,4)==(2,4) yields zero spans (empty selection)
  #   5. Same-row backwards drag: anchor (1,6) cursor (1,2) normalizes to one span (row 1, cols 2..6)
  #
  # ========================================
  Background: User Story
    As a developer
    I want to get a normalized, ordered set of (row, start_col, end_col) spans from a raw anchor/cursor cell pair
    So that I can reconstruct and highlight exactly the cells the user selected regardless of drag direction

  Scenario: Single-row forward selection yields one span
    Given a selection with anchor at row 1 column 2 and cursor at row 1 column 6
    When I request the spans for a row width of 10
    Then I get one span: row 1 columns 2 to 6

  Scenario: Backwards multi-row drag normalizes start before end
    Given a selection with anchor at row 3 column 4 and cursor at row 1 column 2
    When I request the spans for a row width of 10
    Then the normalized start is row 1 column 2 and the normalized end is row 3 column 4

  Scenario: Multi-row selection extends first and middle rows to the row width
    Given a selection with anchor at row 1 column 3 and cursor at row 3 column 5
    When I request the spans for a row width of 10
    Then I get the spans: row 1 columns 3 to 10, row 2 columns 0 to 10, and row 3 columns 0 to 5

  Scenario: Collapsed selection yields no spans
    Given a selection with anchor at row 2 column 4 and cursor at row 2 column 4
    When I request the spans for a row width of 10
    Then I get zero spans

  Scenario: Same-row backwards drag normalizes to one ordered span
    Given a selection with anchor at row 1 column 6 and cursor at row 1 column 2
    When I request the spans for a row width of 10
    Then I get one span: row 1 columns 2 to 6
