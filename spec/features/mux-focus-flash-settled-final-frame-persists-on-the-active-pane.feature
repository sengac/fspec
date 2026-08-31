@done
@mux-007
@tui
@mux
@ui-enhancement
@keyboard-navigation
@mouse-events
@MUX-007
Feature: Mux focus flash — settled final frame persists on the active pane

  """
  Implementation:
  - the render path keeps calling flash_cells but clamps the clock to the settle boundary — after the 350ms window the strip stays parked at the left edge (the LAST_PAINT_MS frame) instead of vanishing. The paint pass must render for the focused pane whenever mux is enabled, not only while is_flash_active(); the tick-gate operand (is_mux_flash_active) keeps its 350ms semantics (animation only, no perpetual redraws). State stays live-only on MultiplexLayout (MuxConfig serde shape unchanged).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: after the 350ms scan window (MUX-006) elapses, the focused pane keeps the scan's FINAL frame painted on every subsequent mux frame — a full-height 2-column dark-purple strip (MUX_FOOTER_BG, RGB 74, 44, 112) parked at the pane's LEFT edge — until focus moves or mux is disabled (1-column panes settle to a 1-column strip)
  #   2. R2: the settled strip moves with focus — when focus changes, the previously focused pane loses its settled strip (no purple remains in that pane) and the newly focused pane plays a fresh 350ms scan that settles into its own left-edge strip (MUX-006 R4 re-arm behavior is preserved)
  #   3. R3: the settled strip is background-only — pane content symbols are never blanked or modified under it (same as MUX-006 R1) and every cell stays inside the focused pane rect (never touches dividers, other panes, or the footer row — MUX-006 R5)
  #   4. R4: the settled strip must NOT keep the run-loop draw gate open — after the 350ms window elapses, tick_should_draw's mux operand (is_mux_flash_active) returns to the idle result even though the strip is still painted on subsequent frames (the strip is repaint content, not an animation; no perpetual 16ms redraws)
  #
  # EXAMPLES:
  #   1. With Board | Agent active and focus on the Board pane: after the 350ms scan ends, the Board pane keeps a full-height 2-column dark-purple strip at its LEFT edge on every frame — the Board pane's left two columns are purple on row 0..bottom, nothing else in the body is purple
  #   2. After the Agent pane's strip has settled, pressing Shift+Left moves focus to the Board pane: the Agent pane's purple strip vanishes immediately (the next frame shows zero purple in the Agent pane) and a fresh 350ms scan sweeps the Board pane right-to-left before its own left-edge strip settles
  #   3. While the settled strip sits under the focused pane's header and input text, the glyphs remain exactly as before the scan (the strip tints backgrounds only, never erases content); with mux disabled (single Board view) no purple cell is ever painted
  #   4. With the session idle and the scan finished, the mux pane shows the settled left-edge strip with no continuous redrawing — the run-loop draw gate reports idle (no animation in flight) yet the strip is still visible on the next user-triggered frame
  #
  # ========================================

  Background: User Story
    As a developer supervising multiple agents in mux mode
    I want to keep the focus-flash final frame (dark-purple left-edge strip) painted on the active pane after the 350ms scan ends
    So that the active pane stays visibly distinguished at a glance instead of losing its accent as soon as the animation finishes

  # R1: after the scan ends, the focused pane keeps the final (left-edge) frame
  Scenario: the active pane keeps the settled left-edge strip after the scan ends
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Board pane is focused with a fresh scan in flight
    When 350ms of rendered frames have elapsed
    Then the Board pane shows the dark purple background (RGB 74, 44, 112) as a full-height 2-column strip at the pane's left edge
    And the Agent pane rect has no dark purple background
    And rendering more frames later the Board pane still shows the same left-edge strip
    And no other cell in the body has the dark purple background

  # R2: the settled strip moves with focus
  Scenario: focus change clears the old pane's strip and settles the new pane's strip
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused with a settled left-edge strip
    When I press Shift+Left
    Then the Agent pane has no dark purple background on the next rendered frame
    And the Board pane is focused and its scan is armed at the start of the 350ms window
    When 350ms of rendered frames have elapsed
    Then the Board pane shows the dark purple strip at its left edge
    And the Agent pane still has no dark purple background

  # R3: background-only, clipped to the focused pane, mux off stays clean
  Scenario: the settled strip keeps pane content readable and never paints with mux off
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused with a settled left-edge strip
    When the frame is rendered to a terminal buffer
    Then every dark purple cell lies inside the Agent pane rect
    And the Agent pane glyphs are unchanged by the strip (background-only tint)
    Given the TUI is in a single-view mode (Board) with mux disabled
    When the frame is rendered to a terminal buffer
    Then no cell in the buffer has the dark purple background

  # R4: the settled strip does not keep the run-loop draw gate open
  Scenario: the settled strip is repaint content that does not force continuous redrawing
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused with a settled left-edge strip
    When the run-loop draw gate is evaluated with no other animation in flight
    Then tick_should_draw reports false (idle — the settled strip is not an animation)
    And the next user-triggered frame still paints the settled left-edge strip on the Agent pane
