@done
@tui-component
@agent-view
@rust
@RPC-383
Feature: TurnContentModal not full-screen and not scrollable (parity gap)

  """
  Sizing: TurnContentModal must paint at a FIXED rect of area.width-4 by area.height-6 (centered), independent of content length. Either extend dialog_theme::FspecDialog/dialog_rect with a fixed/fill-size mode, or compute the forced rect in TurnContentModal::render. Other dialogs (confirm, model selector, thinking) MUST keep shrink-to-content (no regression).
  Scroll state: add a modal scroll offset on AgentView alongside turn_modal_seq (reset to 0 on open via handle_open_turn_modal in app/dispatch_scroll.rs). wrapped_rows() must window by offset (skip offset rows) instead of clipping from row 0, and clamp offset so the last page is fully visible.
  Keyboard: re-wire Up/Down (currently no-ops while modal open in views/agent/dispatch_select.rs:24-37) plus PageUp/PageDown/Home/End to scroll the modal via new Actions reduced on the App task. The turn-selection gate must remain: scrolling the modal must NOT move the underlying selected_seq.
  Mouse wheel: route ScrollUp/ScrollDown to the modal in views/agent/mouse_dispatch.rs while turn_modal_seq.is_some(), mirroring scrollback wheel handling.
  Scrollbar + footer: reuse scrollback_paint::paint_scrollbar (canonical ■/│ DIM painter) for the rightmost column when content overflows; do NOT write a second scrollbar. Footer '↑↓ Scroll | Esc Close' rendered dim/centered via dialog_theme footer support. Keep all touched Rust files < 300 lines (source_shape guards); extract helpers if needed. Tests follow *_parity_rpcNNN.rs convention asserting parity with src/tui/components/TurnContentModal.tsx.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The turn content modal fills the screen (area.width-4 wide by area.height-6 tall), centered, regardless of content length
  #   2. When the body content exceeds the modal's visible height, a single-column scrollbar is shown
  #   3. While the modal is open, Up/Down scroll the modal body by one row without moving the underlying turn selection
  #   4. While the modal is open, PageUp/PageDown scroll by a page and Home/End jump to the top/bottom
  #   5. While the modal is open, the mouse wheel scrolls the modal body
  #   6. The modal shows a dim footer reading '↑↓ Scroll | Esc Close'
  #   7. Opening the modal resets the scroll offset to the top, and the offset is clamped so the last page is fully visible
  #   8. No body content is silently dropped; all text is reachable by scrolling
  #
  # EXAMPLES:
  #   1. On a 40x12 terminal with a 3-line turn, the modal still renders 36 columns wide and 6 rows tall (full screen minus margins), not shrunk to the 3 lines
  #   2. A turn with 100 wrapped lines opens in a 20-row modal; a scrollbar appears and only the first page of lines is visible
  #   3. With a long turn modal open, pressing Down once reveals the next line at the bottom and hides the top line; the selected turn seq is unchanged
  #   4. With a long turn modal open, pressing End scrolls to the bottom showing the final line of the body
  #   5. With a long turn modal open, scrolling the mouse wheel down advances the visible window
  #   6. The modal's bottom row shows the dim text '↑↓ Scroll | Esc Close'
  #   7. Closing and re-opening the modal on a scrolled turn shows the top of the content again (offset reset)
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to view a long turn's full content in a full-screen, scrollable modal
    So that I can read past turns of any length in full, matching the TypeScript reference TUI

  Scenario: The modal fills the screen regardless of short content
    Given a turn content modal showing a 3-line turn on a 40x12 terminal
    When the modal is rendered
    Then the modal occupies 36 columns and 6 rows
    And the modal is not shrunk to fit the 3 lines of content

  Scenario: A scrollbar appears when content overflows the viewport
    Given a turn content modal showing a turn with 100 wrapped lines on a 20-row screen
    When the modal is rendered
    Then a single-column scrollbar is shown in the modal's rightmost column
    And only the first page of lines is visible

  Scenario: Pressing Down scrolls the body without moving the selection
    Given a turn content modal open over a long turn with the second of three turns selected
    When I press the Down arrow key
    Then the visible window advances by one line
    And the selected turn is still the second turn

  Scenario: Pressing End scrolls to the bottom of the content
    Given a turn content modal open over a long turn
    When I press the End key
    Then the modal shows the final line of the body

  Scenario: Pressing PageDown advances the body by a page
    Given a turn content modal open over a long turn
    When I press the PageDown key
    Then the visible window advances by more than one line

  Scenario: Pressing PageUp moves the body back by a page
    Given a turn content modal scrolled to the bottom of a long turn
    When I press the PageUp key
    Then the visible window moves back by more than one line

  Scenario: Pressing Home jumps to the top of the content
    Given a turn content modal scrolled down over a long turn
    When I press the Home key
    Then the modal shows the first line of the body

  Scenario: The mouse wheel scrolls the modal body
    Given a turn content modal open over a long turn
    When I scroll the mouse wheel down over the modal
    Then the visible window advances

  Scenario: The modal shows the scroll/close footer
    Given a turn content modal open over a long turn
    When the modal is rendered
    Then the modal's bottom row shows the dim text "↑↓ Scroll | Esc Close"

  Scenario: Re-opening the modal resets the scroll offset to the top
    Given a turn content modal that has been scrolled down over a long turn
    When I close the modal and re-open it on the same turn
    Then the modal shows the top of the content again
