@done
@agent-view
@tui-component
@rust
@tui
@BLOCK-008
Feature: BlocklistView: implement viewport scrolling (parity with model_selector/changed_files scroll pattern)

  """
  render_left_pane paints only rules[scroll_offset..(scroll_offset+visible_rows).min(len)] and reserves a 1-column scrollbar gutter (reusing the shared scrollbar helper) when rules.len() > visible_rows. A 'Showing X-Y of N' indicator reflects the visible range. Must not regress RPC-056 behaviour (category tags, ○/● glyphs, (disabled) suffix, empty-state, Esc close). File stays under 300 lines — split into blocklist/ sibling modules if needed. Clippy clean, no unwrap/expect/todo in non-test code.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BlocklistView owns a scroll_offset (and a visible_rows) reconciled via the shared scroll_viewport::ensure_visible primitive
  #   2. Every navigation key (Down/Up, and PageUp/PageDown/Home/End per BLOCK-010) clamp-moves selected_index then calls adjust_scroll() so the selection stays inside the visible window
  #   3. adjust_scroll() delegates to scroll_viewport::ensure_visible(scroll_offset, selected_index, visible_rows, rules.len()) and is also called defensively at render time once the real body height is known
  #   4. scroll_offset never runs past total - visible_rows and resets to 0 when rules are empty or visible_rows is 0
  #   5. set_rules resets/clamps scroll_offset alongside selected_index so a shorter list can never leave a stale offset
  #   6. render_left_pane windows the rules to rules[scroll_offset .. (scroll_offset+visible_rows).min(len)] and draws an overflow scrollbar gutter when rules.len() > visible_rows
  #
  # EXAMPLES:
  #   1. A view seeded with 20 rules and visible_rows 8, selected_index 0: pressing Down 10 times moves selected_index to 10 and scroll_offset becomes 3 so row 10 stays in the window [3,11)
  #   2. After scrolling down into the list, pressing Up back above the window scrolls scroll_offset back up so the selection stays visible
  #   3. With 20 rules and visible_rows 8, selecting the last row clamps scroll_offset to 12 (total-visible) and never beyond
  #   4. A view with 30 rules is rendered into a short buffer; only the windowed slice of rows is painted and a 'Showing X-Y of N' indicator reflects the visible range
  #   5. A view holding 20 rules with scroll_offset 12 is re-seeded via set_rules with 3 rules; scroll_offset resets into range (0) and selected_index is clamped
  #   6. An overflowing list renders a scrollbar gutter column in the left pane; a list that fits entirely renders no scrollbar
  #
  # ========================================

  Background: User Story
    As a fspec TUI user viewing a long blocklist
    I want to scroll the rule list so the focused rule always stays visible
    So that I can navigate to and inspect every configured rule even when there are more rules than fit on screen

  Scenario: Scrolling down keeps the focused row inside the visible window
    Given a BlocklistView seeded with 20 rules and a visible window of 8 rows
    And selected_index is 0 and scroll_offset is 0
    When the user presses Down 10 times
    Then selected_index equals 10
    And scroll_offset equals 3 so the focused row stays inside the window

  Scenario: Scrolling back up above the window scrolls the offset back
    Given a BlocklistView seeded with 20 rules and a visible window of 8 rows
    And selected_index is 10 and scroll_offset is 3
    When the user presses Up 8 times
    Then selected_index equals 2
    And scroll_offset equals 2 so the focused row stays inside the window

  Scenario: scroll_offset clamps at total minus visible when selecting the last row
    Given a BlocklistView seeded with 20 rules and a visible window of 8 rows
    When the focused row is moved to the last rule
    Then scroll_offset equals 12
    And scroll_offset never exceeds total minus visible rows

  Scenario: Rendering an overflowing list windows the rows and shows a scroll indicator
    Given a BlocklistView seeded with 30 rules
    When the view is rendered into a buffer shorter than the rule list
    Then only the windowed slice of rows is painted
    And the rendered text contains a "Showing" scroll indicator reflecting the visible range

  Scenario: Re-seeding with a shorter list resets the scroll offset into range
    Given a BlocklistView holding 20 rules with scroll_offset 12
    When set_rules replaces the list with 3 rules
    Then scroll_offset resets to 0
    And selected_index is clamped inside the new list

  Scenario: An overflowing list renders a scrollbar gutter and a fitting list does not
    Given a BlocklistView seeded with more rules than fit the pane
    When the view is rendered
    Then a scrollbar gutter column is drawn in the left pane
    When the view is re-seeded with a list that fits entirely
    And the view is rendered again
    Then no scrollbar gutter column is drawn
