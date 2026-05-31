@done
@RPC-019
@rust
@tui
@ui
@agent-view
@scrollback
@scroll
@ui-enhancement
Feature: RPC-019 AgentView windowed ScrollbackList (O(1) per frame)
  """
  RPC-019 (scrollback slice) — AgentView's flat `Vec<RenderedChunk>` and
  manual scroll math are replaced by a `ScrollbackList` widget that
  paints O(1) rows per frame regardless of total chunk count.

  Layout stays the 4-row vertical split established by RPC-018:
  [SessionHeader (Length 1), Scrollback (Min 0), Input (Length N+2),
  SessionFooter (Length 1)]
  where N is the MultiLineInput's current visible-row count, clamped
  to `[1, max_visible_rows]` (default cap = 6).

  ScrollbackList semantics:
  - `push(chunk)` appends; while `stick_to_bottom` is true, `offset`
  auto-advances so the latest chunk stays visible.
  - PageUp disables stick_to_bottom and decrements `offset` by exactly
  one viewport height (capped at 0).
  - PageDown / End increment `offset` by one viewport height; reaching
  `total_lines - viewport_height` re-enables stick_to_bottom.
  - `render(area, buf)` only iterates the chunks that fall in
  `offset..offset+area.height` — total chunk count does NOT affect
  per-frame work.

  Pair: render tests live in
  codelet/fspec-tui/tests/view_agent_scrollback_rpc019.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want an O(1)-per-frame windowed ScrollbackList in AgentView with PageUp/PageDown + stick-to-bottom semantics
    So that long agent sessions (10_000+ chunks) remain responsive on every keystroke

  Scenario: ScrollbackList::push appends a chunk and bumps offset in stick mode
    Given a ScrollbackList in stick_to_bottom mode with 100 single-line chunks and viewport_height = 12
    When the ScrollbackList::push appends one more single-line chunk (chunk #101)
    Then the ScrollbackList's offset is 89
    And ScrollbackList::stick_to_bottom is true
    And the visible chunks include chunk #101 at the bottom

  Scenario: PageUp on the scrollback decrements offset by viewport_height and disables stick
    Given a ScrollbackList in stick_to_bottom mode with 100 single-line chunks and viewport_height = 12
    When the user presses PageUp inside AgentView
    Then the ScrollbackList's offset is exactly 76
    And ScrollbackList::stick_to_bottom is false

  Scenario: PageDown / End from a scrolled-up position re-enables stick when offset reaches the tail
    Given a ScrollbackList at offset 76 with stick_to_bottom = false, 100 single-line chunks, viewport_height = 12
    When the user presses PageDown inside AgentView
    Then the ScrollbackList's offset is exactly 88
    And ScrollbackList::stick_to_bottom is true

  Scenario: ScrollbackList::render only lays out the visible window
    Given a ScrollbackList with 10_000 single-line chunks in stick_to_bottom mode and viewport_height = 12
    When ScrollbackList::render is called against an 80x12 area
    Then the number of chunks visited during layout is at most 12
    And the rendered buffer's bottom row contains chunk #9999's body

  Scenario: AgentView vertical layout reserves Length(visible_rows) for the input box (RPC-029)
    Given an AgentView whose MultiLineInput contains "a\nb\nc"
    When the App renders AgentView against an 80x20 TestBackend
    Then the input box occupies exactly 3 rows (RPC-029: no 4-sided border)
    And the scrollback region occupies the remaining flex rows between the header and the footer
    And the SessionFooter row above the input does NOT contain the substring "Enter=send" (RPC-029)
