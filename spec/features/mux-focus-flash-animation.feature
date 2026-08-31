@done
@MUX-006
@tui
@mux
@ui-enhancement
@keyboard-navigation
@mouse-events
Feature: Mux focus flash — 350ms right-to-left scan strip on the selected pane
  """
  When a mux pane becomes the selected (focused) pane, paint a brief 350ms
  background scan across the focused pane rect using the mux footer dark
  purple (MUX_FOOTER_BG = Color::Rgb(74, 44, 112), MUX-005). The animation
  is a single full-height 2-column scan strip that sweeps from the pane's
  RIGHT edge to the LEFT over the 350ms window (a right-to-left scanner).
  Only cell BACKGROUNDS are touched — pane content symbols are never
  blanked. The flash is re-armed on every focus change while mux is
  enabled (Shift+Left/Right focus movement, click-to-focus, Enter on a
  board work unit, BackToBoard, new-agent focus, pane-count/pane-list
  changes, fresh mux entry). The run-loop draw gate (app/mod.rs
  tick_should_draw) has a 5th operand (is_mux_flash_active) so the 16ms
  render tick keeps redrawing during the 350ms window even when the
  session is idle. The clock is a render-driven animation clock (+16ms per
  rendered mux frame — the same pattern as AgentView::animation_clock_ms),
  NOT a wall-clock timer and NOT a per-view tokio interval (TUI-106
  architecture decision). Flash state is LIVE-ONLY (not persisted;
  MuxConfig serde shape unchanged). Mux off / single-view rendering stays
  byte-for-byte unchanged (no purple cells, rust-mux-mode.feature R10).
  Implementation: MultiplexLayout owns the flash state (flash pane index +
  clock) in views/multiplex; the pure frame-pattern function (clock_ms,
  pane rect) -> set of purple cells + the painter live in
  views/multiplex/flash.rs and render.rs. Tick-gate chain:
  MultiplexLayout::is_flash_active → Navigator (active_view == Mux) →
  App::is_mux_flash_active → 5th tick_should_draw operand.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: the flash uses the SAME dark purple as the mux footer bar (Color::Rgb(74, 44, 112) — MUX-005 MUX_FOOTER_BG) and is painted as cell BACKGROUNDS only — pane content symbols are never blanked or modified
  #   2. R2: the flash lasts 350ms from the focus change; during that window the scan strip is always present (right-edge start to left-edge end) and is clipped to the focused pane rect
  #   3. R3: the flash is a single full-height vertical scan strip — 2 columns wide, all rows of the focused pane — sweeping from the RIGHT edge of the pane to the LEFT over the 350ms window (right-to-left); there are no other phases — no burst ring, no rain, no 2-row band
  #   4. R4: the flash is re-armed on EVERY focus change while mux is enabled (Shift+Left/Right focus movement, click-to-focus, Enter on a board work unit → agent pane, BackToBoard → board pane, new-agent focus, pane-count/pane-list changes, fresh mux entry); re-arming restarts the 350ms window at the start of the right-to-left scan
  #   5. R5: no purple cell is ever painted OUTSIDE the focused pane's rect (dividers, other panes, the footer row, and the terminal area outside the body are untouched) and pane content remains readable under the flash
  #   6. R6: the 16ms render tick keeps redrawing during the flash even when nothing else is animating (a 5th tick_should_draw operand); after the 350ms window elapses the flash stops and the tick gate returns to idle
  #   7. R7: when mux is off (single-view modes), no flash state is armed, no purple flash cells are painted, and the tick gate's mux operand is false — single-view behavior is byte-for-byte unchanged (rust-mux-mode.feature R10)
  #   8. R8: flash state is live-only — it is NOT persisted with MuxConfig (tui.mux in fspec-config.json keeps its existing serde shape) and a restart never resumes a flash mid-flight
  #
  # EXAMPLES:
  #   1. With Board | Agent active and focus on the Board pane, pressing Shift+Right moves focus to the Agent pane: a full-height 2-column purple strip appears at the Agent pane's RIGHT edge and sweeps left across the pane over 350ms, then the flash is gone while the Agent pane stays focused
  #   2. Clicking inside the Board pane while the Agent pane is focused re-arms the flash on the Board pane; the Agent pane's flash (if still in flight) is not re-armed
  #   3. With the Board pane focused and work unit AUTH-001 selected, pressing Enter binds the unit and focuses the agent pane — the agent pane flashes; the board pane does not
  #   4. With the session idle (no spinner, no input animation), a Shift+Left focus change still produces a full 350ms scan flash — the run loop keeps ticking for the flash alone
  #   5. During the flash, the pane's header, scrollback text, input placeholder, and footer text remain visible — the flash tints their backgrounds but never erases their glyphs
  #   6. With mux disabled (single Board view), no purple (RGB 74,44,112) cell is ever painted anywhere on screen
  #   7. The flash pattern is deterministic for a given (clock, pane rect) — re-rendering the same frame at the same clock value paints the identical set of purple cells.
  #
  # ASSUMPTIONS:
  #   1. The flash is an additive paint pass AFTER the panes render into their rects and BEFORE the dividers/footer paint; it touches only background channels of cells inside the focused pane rect.
  #   2. The run loop owns the clock (SSR, TUI-106/107 decision): the view reports is_flash_active and the mux render advances a render-driven clock by +16ms per rendered frame — no tokio timers in the view.
  #
  # ========================================
  Background: User Story
    As a developer supervising multiple agents in mux mode
    I want a brief 350ms purple scan strip to sweep from the right to the left across the pane that just became selected
    So that I know instantly which pane has keyboard focus without staring at the footer indicator

  # R1 + R3: the flash paints a full-height 2-column strip scanning right-to-left
  Scenario: focusing a pane scans a purple strip from the right edge to the left
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused
    When the mux frame is rendered to a terminal buffer
    Then the Agent pane rect shows the dark purple background (RGB 74, 44, 112) as a full-height 2-column strip at the pane's right edge
    And the Board pane rect has no dark purple flash background
    And rendering a few frames later the strip has moved LEFT (a mid-scan position, still 2 columns wide)
    And rendering to the end of the 350ms window shows the strip at the pane's left edge
    And after 350ms of rendered frames the flash has ended and no flash purple cells remain in either pane

  # R2 + R5: the flash stays inside the focused pane rect and never erases content
  Scenario: the flash stays inside the focused pane and keeps pane content readable
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Agent pane is focused
    When the mux frame is rendered to a terminal buffer during the flash
    Then every flash-purple cell lies inside the Agent pane rect
    And no flash-purple cell lies in the divider column, the Board pane rect, or the mux footer row
    And the Agent pane header, scrollback text and input placeholder glyphs are unchanged by the flash (background-only tint)

  # R4: every focus-change path re-arms the flash on the newly focused pane
  Scenario: Shift+Right, click-to-focus, Enter on a work unit and BackToBoard each re-arm the flash on the newly focused pane
    Given mux mode is active with the default two-pane grid (Board | Agent)
    And the Board pane is focused with work unit AUTH-001 selected
    When I press Shift+Right
    Then the Agent pane is focused and its flash is armed at the start of the 350ms window
    When I click inside the Board pane rect
    Then the Board pane is focused and its flash is armed
    When I press Enter on the selected work unit
    Then the Agent pane is focused and its flash is re-armed (restarted at the right edge)
    When BackToBoard lands
    Then the Board pane is focused within the grid and its flash is armed

  # R6: the flash keeps the run loop ticking while idle, and stops after 350ms
  Scenario: the flash keeps the 16ms render tick redrawing while idle and stops after the window elapses
    Given mux mode is active with the default two-pane grid and the session is idle
    And the Agent pane is focused with a fresh flash
    When the run-loop draw gate is evaluated with no other animation in flight
    Then tick_should_draw reports true because the mux flash is active
    When 350ms of rendered frames have elapsed
    Then the mux flash is no longer active and tick_should_draw returns to the idle result (false when nothing else is animating)

  # R7: single-view rendering is untouched (rust-mux-mode.feature R10)
  Scenario: no flash occurs and no purple cells are painted when mux is off
    Given the TUI is in a single-view mode (Board) with mux disabled
    When the frame is rendered to a terminal buffer
    Then no cell in the buffer has the dark purple flash background
    And the mux-flash operand of the run-loop draw gate is false

  # R8 + R3: deterministic, live-only flash state
  Scenario: the flash pattern is deterministic for a given clock and never persisted
    Given mux mode is active with the default two-pane grid
    And the Agent pane is focused with a fresh flash
    When the frame is rendered at a given clock value and then re-rendered at the same clock value
    Then both frames paint the identical set of dark purple cells (deterministic pattern)
    And the persisted tui.mux config shape is unchanged by the flash (no flash fields written)
