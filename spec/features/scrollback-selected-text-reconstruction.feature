@done
@rust
@scrollback
@text-selection
@tui
@COPY-004
Feature: Reconstruct selected text from scrollback, excluding scrollbar gutter
  """
  New function on ScrollbackList (or a free fn in a new scrollback_copy.rs, extracted to respect the 300-LoC ceiling like scrollback_select.rs). Signature: fn selected_text(&self, region: &[RowSpan]) -> String taking the RowSpans from COPY-002's Selection::spans(content_width). Reads self.chunks[].lines (Vec<Line<'static>>) and self.scroll_state.offset.
  Content width = viewport_width minus reserved gutter (reserve_gutter = 2 when overflow, per scrollback.rs render_count_visited). The caller (COPY-006) computes content_width and passes it to Selection::spans so end cols are already clamped; COPY-004 additionally clamps to each row's real char length. This double-guards the scrollbar-glyph exclusion (the TS Ink bug where │/■ were copied).
  Row flattening: a visual row is one Line; build its plain string by concatenating span.content across line.spans, then char-slice [start_col..min(end_col, char_len)] using chars().skip().take(). visual_row_index = scroll_state.offset + viewport_row; total visual rows walk chunks in order summing lines.len() (same iteration as total_visual_rows()).
  Testing: build a ScrollbackList with known chunks/lines (test helper already used by scrollback tests), set scroll_state.offset, call selected_text with hand-built RowSpans, assert the exact String. Cover single-row, multi-row, gutter-exclusion, offset mapping, over-length clamp, emoji, and empty cases. Pure, no I/O.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Selected viewport rows are mapped back to scrollback visual rows using the current scroll offset (offset + viewport_row = visual_row_index)
  #   2. Each row's text is the concatenation of its Line spans' content, sliced by the span's start_col..end_col
  #   3. Every row's end column is clamped to the content width (viewport width minus reserved scrollbar gutter) so scrollbar glyphs are never included
  #   4. Rows are joined with a single newline (\n) in top-to-bottom order
  #   5. A column beyond a row's actual content length is treated as end-of-row (no padding spaces are added past real content)
  #   6. Column slicing operates on characters (unicode scalar boundaries), not raw bytes, so multi-byte glyphs are not split
  #   7. An empty selection region produces an empty string
  #
  # EXAMPLES:
  #   1. Single-row selection of visual row "Hello world" cols 0..5 reconstructs "Hello"
  #   2. Selection spanning rows "foo" and "bar" (full width) reconstructs "foo\nbar"
  #   3. A row rendered as "answer text  │" with a scrollbar glyph at the gutter, selected full-width, reconstructs "answer text" (gutter │ excluded via content-width clamp)
  #   4. With scroll offset 10, selecting viewport row 2 reads scrollback visual row 12
  #   5. Selecting cols 0..20 on the row "hi" (only 2 chars) reconstructs "hi" (end clamped to content length, no trailing spaces)
  #   6. Selecting an emoji row "a😀b" cols 0..2 reconstructs "a😀" without splitting the emoji's bytes
  #   7. An empty region (collapsed selection) reconstructs "" (empty string)
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to copy exactly the transcript text I selected, with no scrollbar characters mixed in
    So that I can paste clean text elsewhere without stray scrollbar glyphs like │ or ■

  Scenario: Reconstruct a single-row partial selection
    Given a scrollback whose visual row reads "Hello world"
    When I reconstruct the selection of that row from column 0 to column 5
    Then the reconstructed text is "Hello"

  Scenario: Reconstruct a multi-row selection joined by newline
    Given a scrollback with consecutive visual rows "foo" and "bar"
    When I reconstruct the full-width selection spanning both rows
    Then the reconstructed text is "foo\nbar"

  Scenario: The scrollbar gutter glyph is excluded from copied text
    Given a scrollback row rendered as "answer text  │" where the │ sits in the reserved scrollbar gutter
    When I reconstruct the full-width selection of that row clamped to the content width
    Then the reconstructed text is "answer text"

  Scenario: Scroll offset maps viewport rows to visual rows
    Given a scrollback with scroll offset 10
    When I reconstruct the selection of viewport row 2
    Then the text comes from scrollback visual row 12

  Scenario: End column beyond content length is clamped without padding
    Given a scrollback whose visual row reads "hi"
    When I reconstruct the selection of that row from column 0 to column 20
    Then the reconstructed text is "hi" with no trailing spaces

  Scenario: Multi-byte glyphs are sliced on character boundaries
    Given a scrollback whose visual row reads "a😀b"
    When I reconstruct the selection of that row from column 0 to column 2
    Then the reconstructed text is "a😀" and the emoji bytes are not split

  Scenario: An empty selection reconstructs an empty string
    Given a scrollback with a collapsed (empty) selection region
    When I reconstruct the selected text
    Then the reconstructed text is the empty string
