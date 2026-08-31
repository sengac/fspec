@done
@tui
@mux
@ui-enhancement
@MUX-008
Feature: Focus flash scans bottom-to-top and settles as a 1-row top bar
  """
  Geometry change to the existing pure flash pattern fn (views/multiplex/flash.rs flash_cells): the scan strip is a single 1-row-high full-width row that travels along the Y axis from the pane's bottom edge up to its top edge over the 350ms window; the MUX-007 settle (clocks >= LAST_PAINT_MS) keeps painting that same final frame — the 1-row bar across the pane's top row (full width) — instead of the flash vanishing. No new state, timers or config fields — MultiplexLayout flash state (flash_pane + flash_clock_ms), the paint pass in views/multiplex/render.rs and the tick-gate chain (is_mux_flash_active) are unchanged. MUX-006 R3 and MUX-007 R1 are superseded (their scenario text is updated in place).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: the MUX-006 focus-flash scan strip is 1 ROW HIGH (a single full-width row of dark purple) and sweeps from the pane's BOTTOM edge UP to the TOP over the 350ms window (bottom-to-top). It spans the full width of the focused pane. The last painted frame sits exactly on the pane's TOP row, which is also the settled frame (R2). Supersedes MUX-006 R3 (full-height 2-column right-to-left strip).
  #   2. R2: the MUX-007 settled final frame is a 1-ROW-HIGH bar across the pane's TOP row (the full width of the focused pane, dark purple MUX_FOOTER_BG background). It is exactly the last painted scan frame of R1, so clocks past the 350ms window settle to that same top-row bar instead of the strip vanishing. Supersedes MUX-007 R1 (full-height 2-column strip at the left edge).
  #
  # EXAMPLES:
  #   1. With Board | Agent active and focus on the Board pane: frame 1 shows a single purple row across the full width at the pane's BOTTOM edge; 9 frames later that row has moved UP to a mid-scan position (still 1 row high, full width); after 350ms the pane shows exactly 1 purple row at its TOP row (full width) on every subsequent frame.
  #
  # ========================================
  Background: User Story
    As a developer supervising multiple agents in mux mode
    I want to see the focus flash sweep from the bottom of the newly-selected pane up to its top and settle as a 1-row top bar
    So that the active pane stays distinguishable without a full-height accent eating the pane edge

  # R1: the scan strip sweeps bottom-to-top and the last scan frame sits
  # at the top edge
  Scenario: focusing a pane scans a purple strip from the bottom edge to the top
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused
    When the mux frame is rendered to a terminal buffer
    Then the Agent pane rect shows the dark purple background (RGB 74, 44, 112) as a single full-width row at the pane's bottom edge
    And the Board pane rect has no dark purple flash background
    And rendering a few frames later the strip has moved UP (a mid-scan position, still 1 row high and full width)
    And rendering to the end of the 350ms window shows the strip at the pane's top edge

  # R1 + R2: after the window the 1-row top bar settles (supersedes the
  # MUX-007 left-edge strip)
  Scenario: the settled frame is a 1-row top bar that persists on the focused pane
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused with a fresh scan in flight
    When 350ms of rendered frames have elapsed
    Then the Agent pane shows the dark purple background (RGB 74, 44, 112) as a 1-row-high bar across the full pane width on the pane's top row
    And no other cell in the Agent pane has the dark purple background
    And the Agent pane shows the same 1-row top bar on every subsequent rendered frame
    And the Board pane rect has no dark purple background
    And no purple cell lies in the divider column or the mux footer row

  # R1 + R2: focus changes re-arm the scan and move the settled bar (same
  # re-arm semantics as MUX-006 R4 / MUX-007 R2)
  Scenario: focus change clears the old pane's bar and settles the new pane's top bar
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused with the settled 1-row top bar
    When I press Shift+Left
    Then the Agent pane has no dark purple background on the next rendered frame
    And the Board pane is focused and its scan is armed at the start of the 350ms window
    When 350ms of rendered frames have elapsed
    Then the Board pane shows the dark purple 1-row top bar across its top row
    And the Agent pane still has no dark purple background

  # R1 + R2: deterministic, background-only, mux off stays clean
  Scenario: the bottom-to-top flash and settled top bar are deterministic, background-only and off when mux is disabled
    Given mux mode is active with the default two-pane grid
    And the Agent pane is focused with a fresh flash
    When the frame is rendered at a given clock value and then re-rendered at the same clock value
    Then both frames paint the identical set of dark purple cells (deterministic pattern)
    And the Agent pane glyphs are unchanged by the flash (background-only tint)
    Given the TUI is in a single-view mode (Board) with mux disabled
    When the frame is rendered to a terminal buffer
    Then no cell in the buffer has the dark purple background
