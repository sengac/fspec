@done
@rust
@scrollback
@text-selection
@tui
@COPY-005
Feature: Render selection highlight overlay in scrollback
  """
  New pub(super) fn paint_selection_highlight(area, buf, spans_in_viewport, content_width) in scrollback_paint.rs, called from ScrollbackList::render_count_visited AFTER paint_chunk_rows and paint_selection_overlay (arrow bars). Iterates viewport RowSpans, and for each cell in area.x+col .. clamped to content_width sets buf[(x,y)].set_style(reversed).
  Style: Style::default().add_modifier(Modifier::REVERSED) applied via Cell::set_style so the underlying glyph is preserved and only fg/bg swap. This mirrors ratatui's standard selection-highlight approach and coexists with the DIM arrow bars from RPC-381.
  Input is viewport-space RowSpans (already offset-mapped and content-width-clamped) that COPY-006 derives from the live Selection. COPY-005 itself only paints; the mapping/clamping is shared with COPY-004 to keep a single source of truth for what 'selected' means.
  Testing: render into a TestBackend/ratatui Buffer, call the paint fn with hand-built viewport RowSpans, and assert the target cells' style contains REVERSED and gutter/out-of-region cells do not. Follows existing scrollback_paint render tests (buffer assertions).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When a selection is active, the cells inside the selected region are painted with a reversed/highlighted style
  #   2. Only cells within the content width are highlighted; the reserved scrollbar gutter column is never highlighted
  #   3. When there is no active selection, no highlight is painted
  #   4. The highlight is painted after the chunk text so it visually overlays existing content without erasing it
  #   5. The highlight overlay does not interfere with the turn-select arrow bars (they can coexist)
  #   6. The selection region is mapped from visual rows to viewport rows using the scroll offset; rows scrolled out of view are not highlighted
  #
  # EXAMPLES:
  #   1. With a single-row selection cols 0..5 on viewport row 3, the buffer cells (row 3, cols 0..5) carry the reversed style after paint
  #   2. A full-width selection on a row with a scrollbar gutter leaves the last (gutter) column un-highlighted
  #   3. With no active selection, painting leaves every cell's style unchanged (no reversed cells)
  #   4. A multi-row selection highlights the first row from start_col to content width, the middle row fully, and the last row up to end_col
  #   5. A selection whose top row is above the viewport (scrolled off) highlights only the visible portion
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to see the transcript text I am selecting highlighted as I drag
    So that I get visual feedback about exactly what will be copied

  Scenario: Selected cells are painted with the reversed style
    Given a rendered scrollback with an active single-row selection at viewport row 3 columns 0 to 5
    When the selection highlight is painted
    Then the buffer cells at row 3 columns 0 to 5 carry the reversed style

  Scenario: The scrollbar gutter column is never highlighted
    Given a rendered scrollback with a full-width selection on a row that has a scrollbar gutter
    When the selection highlight is painted
    Then the reserved gutter column is left un-highlighted

  Scenario: No selection paints no highlight
    Given a rendered scrollback with no active selection
    When the selection highlight is painted
    Then every cell keeps its original style and no cell is reversed

  Scenario: A multi-row selection highlights first, middle, and last rows correctly
    Given a rendered scrollback with an active selection spanning three viewport rows
    When the selection highlight is painted
    Then the first row is highlighted from its start column to the content width
    And the middle row is highlighted fully within the content width
    And the last row is highlighted up to its end column

  Scenario: A selection scrolled partly off-screen highlights only the visible portion
    Given a rendered scrollback whose selection top row is above the viewport
    When the selection highlight is painted
    Then only the visible rows of the selection carry the reversed style
