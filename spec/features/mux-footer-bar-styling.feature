@done
@mux
@ui-enhancement
@tui
@MUX-005
Feature: Mux footer bar styling: white foreground on dark purple background
  """
  Styling is applied in paint_footer (rust/fspec-tui/src/views/multiplex/render.rs): every cell of the footer row is written with Style::default().fg(Color::White).bg(Color::Rgb(74, 44, 112)) — the full row width, not just the label. A named MUX_FOOTER_BG constant mirrors the agent-view chrome constants (FOOTER_BG/HEADER_BG in views/agent/footer.rs|header.rs). No layout changes: the 1-row footer reservation in render_with_stores stays as-is; this is a pure paint change. Dependencies: ratatui Style (fg/bg Color); paint_footer is the only place the footer row is painted (panes render into the body area, dividers are separate), so no other view changes are needed. Mux off still short-circuits in render_with_stores before paint_footer is reached.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The dark purple background fills the FULL footer row width (area.x .. area.x+area.width), not just the cells behind the footer label text
  #   2. The mux footer row is painted with white foreground text (Color::White) on a dark purple background (Color::Rgb(74, 44, 112))
  #
  # EXAMPLES:
  #   1. With mux active and two panes, the bottom row shows 'MUX 2 panes [Board|Agent]  ●pane 0  /mux config · Shift+←/→ focus · drag divider' in white text on a dark purple bar; the cells past the label up to the terminal edge are still dark purple (the whole row is one solid bar).
  #   2. When the mux is off (mux off / single-view modes), no footer bar is painted — the existing 'no mux footer' behavior and its tests stay green (single-view output unchanged).
  #
  # ========================================
  Background: User Story
    As a developer using mux mode
    I want to see the mux status bar clearly distinguished at the bottom of the screen
    So that the footer bar stands out visually with white text on a dark purple background

  # R1: the footer bar is a solid dark purple strip with white text across the full row
  Scenario: The mux footer bar paints white text on a dark purple background across the full row
    Given mux mode is active with the default two-pane grid (Board | Agent)
    When the mux frame is rendered to a terminal buffer
    Then the footer row contains the footer label "MUX 2 panes [Board|Agent]"
    And every label cell in the footer row uses white foreground on a dark purple background (RGB 74, 44, 112)
    And every cell from the end of the label to the right terminal edge is also dark purple (no terminal-background gap)

  # R2: single-view rendering is untouched when mux is off
  Scenario: No footer bar is painted when mux is off
    Given the TUI is in a single-view mode (Board or Agent) with mux disabled
    When the frame is rendered to a terminal buffer
    Then no footer bar is painted at the bottom of the screen
    And no cell in the buffer has the dark purple background
