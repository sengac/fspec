@done
@rust
@agent-view
@tui
@RPC-393
Feature: Refactor Edit/Write diff formatting to a clean structured-row model with consistent gutter coloring
  """
  Chosen gutter-consistency rule: line-number gutter is ALWAYS rendered with a dim/gray style OUTSIDE the colored background; the red/green bar fills from the marker column to the right edge. Applied uniformly to Removed/Added/Context. This fixes defect A (no per-row-type flip) while preserving RPC-392 full-width bars.
  Option 1 chosen (string codec): ChunkSource.text/full_text stay String to keep the store shape stable (rpc024/026 source-shape ceilings). A single private codec in diff_format.rs (to_line/parse_line) is the sole encode/decode; chunk_wrap and turn_modal parse_line then call the shared style_row. Marker steganography + context_gutter_len/strip_marker heuristics deleted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Each diff display row is a typed DiffDisplayRow (Removed/Added/Context/Elision), not a marker-encoded string used as a color channel
  #   2. Exactly ONE style_row(row,width) function maps a row to styled spans; scrollback and modal both use it
  #   3. The line-number gutter is styled consistently across all row types with no per-row-type flip; the gutter is always dim/gray and outside the colored bar
  #   4. Removed lines use rgb 139,0,0 background and Added lines use rgb 0,100,0 background, white fg, padded full-width (RPC-392 preserved)
  #   5. Context lines have a gray gutter, white/default content, and no background
  #   6. Gap markers and collapse hints render through ONE shared elision helper with identical indentation and dim styling
  #   7. The visible column layout (line-number width >= 3 right-aligned, -/+ aligned, content start) is unchanged from current output
  #   8. The diff body survives a terminal-width re-wrap (resize via rewrap_at): rows recovered, no markers leak, no panic
  #   9. The turn-content modal shows the FULL uncollapsed diff, styled identically to the scrollback
  #   10. Non-Edit/Write tool output (Bash/Grep/etc.) is unaffected - no diff styling, no regression
  #   11. Option 1 codec: to_line/parse_line are exact inverses (round-trip property); no line.find("[R]") / regex re-derivation outside the single codec
  #   12. No literal marker (e.g. [R]/[A]) is ever visible on screen
  #
  # EXAMPLES:
  #   1. A single-line replacement builds [Context, Removed, Added, Context] rows with correct line_no values under CONTEXT_LINES=3 windowing
  #   2. A mid-file change in a 100-line edit drops the leading region and marks the trailing skipped region with one uniform Elision kind
  #   3. A diff exceeding 25 display rows yields a collapse Elision hint while the full build does not
  #   4. style_row on a Removed/Added row returns spans whose total display width equals render width, bg red/green, fg white, no marker chars, gutter styled per the consistent rule
  #   5. style_row on a Context row returns a gray gutter span and a content span with no background, not padded full-width
  #   6. style_row on an Elision row is dim with one uniform indentation, identical for a gap marker and a collapse hint
  #   7. The gutter style of a Context row and the gutter region of a Removed/Added row follow the same rule with no per-type flip
  #   8. parse_line(to_line(row)) == row for every variant including unusual content (spaces, brackets, digits, empty text)
  #   9. style_row at zero/small width does not panic and pads saturating
  #   10. Scrollback: an Edit ToolCall + ToolResult wrapped at width 50 yields full-width colored removed/added lines, a gray-gutter context line with no bg, consistent gutter, and no [R]/[A] text
  #   11. Resize re-wrap: wrap at width 50 then rewrap_at width 80 still renders the diff correctly with colors intact and no marker leakage
  #   12. Modal: TurnContentModal over the full diff renders full-width bars for diff rows, plain rows not, consistent gutter, no markers on screen
  #   13. No-regression: a Bash tool result renders plain with no diff background
  #
  # ========================================
  Background: User Story
    As a fspec TUI developer
    I want to represent Edit/Write diff rows as a typed structured-row model with one uniform styling function
    So that the diff renders with consistent gutter coloring and no fragile marker steganography while looking identical on screen

  Scenario: A single-line replacement builds typed Context/Removed/Added rows
    Given an old_string and new_string that differ in a single line
    When I build the diff display rows
    Then the rows are typed Context, Removed, Added, Context in order
    And each row carries its correct 1-based line number under three lines of context

  Scenario: A mid-file change drops the leading region and marks the trailing skipped region with one uniform Elision
    Given a 100-line edit with a single changed line in the middle
    When I build the diff display rows
    Then the leading region is dropped and a trailing Elision row marks the skipped region after the change
    And every elision is the same uniform Elision kind rather than a bespoke string

  Scenario: A diff exceeding the collapse limit yields a collapse Elision hint
    Given a diff whose display rows exceed the collapse limit of 25
    When I build the collapsed diff display rows
    Then the final row is an Elision collapse hint
    And the full uncollapsed build contains no collapse Elision hint

  Scenario: style_row on changed rows fills a full-width colored bar
    Given a Removed row and an Added row
    When I style each row at a render width wider than its content
    Then the styled spans total display width equals the render width
    And the removed bar background is rgb 139,0,0 and the added bar background is rgb 0,100,0 with white foreground
    And no styled span contains a marker character

  Scenario: style_row on a context row has a gray gutter and no background
    Given a Context row
    When I style the row at a render width wider than its content
    Then the gutter span is gray and the content span is white
    And neither span carries a background colour
    And the content is not padded full-width

  Scenario: style_row on elision rows uses one uniform dim indentation
    Given a gap-marker Elision row and a collapse-hint Elision row
    When I style each elision row
    Then both render dim with the same uniform indentation

  Scenario: The gutter style is consistent across all row types
    Given a Context row and a Removed row
    When I style both rows
    Then the gutter region of each row follows the same dim/gray rule with no per-row-type flip

  Scenario: The row codec round-trips every variant exactly
    Given diff display rows of every variant including unusual content with spaces, brackets, digits, and empty text
    When I serialize each row to a line and parse it back
    Then the parsed row equals the original row

  Scenario: style_row at zero width does not panic
    Given a Removed row
    When I style the row at width zero
    Then no panic occurs and no padding is added beyond the content

  Scenario: Scrollback renders a wrapped diff with consistent gutter and no markers
    Given a diff tool-call whose body has a context line, a removed line, and an added line
    When the source is wrapped at width 50
    Then the removed and added lines are full-width colored bars
    And the context line has a gray gutter and no background
    And no rendered line contains a literal diff marker

  Scenario: The diff body survives a terminal-width re-wrap on resize
    Given a diff tool-call card wrapped at width 50
    When the chunk is re-wrapped at width 80
    Then the diff still renders removed and added colored bars
    And no rendered line contains a literal diff marker

  Scenario: The modal shows the full diff styled identically
    Given a modal over the full uncollapsed diff body
    When the modal renders its rows
    Then the diff rows are full-width colored bars and plain rows are not
    And no marker characters appear on screen

  Scenario: Non-diff tool output renders plain with no diff styling
    Given a Bash tool result with no captured diff
    When the tool card is wrapped into lines
    Then no line carries a red or green diff background
