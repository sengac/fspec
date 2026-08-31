@done
@tui
@ui-enhancement
@MUX-003
Feature: Equal-division pane splits (no minimums) with live resize

  """
  Layout math lives in rust/fspec-tui/src/views/multiplex/layout.rs (calculate_pane_rects / calculate_pane_rects_with_override). The equal-division default replaces the 50% cascade: when splits has no entry for a pane (or the pane list was set without a trailing percent), each pane gets available/n and the last pane absorbs the integer-division remainder. The per-pane minimum clamps (MIN_BOARD_PANE_WIDTH=64, MIN_PANE_WIDTH=20, MIN_PANE_HEIGHT=10) are removed from the default path; only the mouse divider drag produces a non-equal split (its 10..=90 percent clamp is retained). Render (render.rs) already recomputes rects from the live frame area every draw, so terminal resizes re-divide automatically; recompute_rects (rects.rs) is the event-time path for config changes. Feature file: spec/features/equal-division-pane-splits-no-minimums-with-live-resize.feature. Tests: rust/fspec-tui/tests/mux003.rs (new) + mux001.rs clamping tests rewritten to expect equal division.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: When no explicit split percent is given, panes divide the terminal area EQUALLY by pane count (n panes → each ~available/n), not a fixed 50% cascade where the last pane takes the remainder.
  #   2. R2: The per-pane minimum-size clamps (board 64 cols, non-board 20 cols, vertical 10 rows) are removed from the default layout math. A non-equal split is produced ONLY by the mouse divider drag (which keeps its 10..=90 percent clamp).
  #   3. R3: The layout recomputes from the live terminal area on every render, so a terminal resize re-divides the panes equally (or re-applies the stored drag percent) without any stale cached rect.
  #   4. R4: An explicit trailing split percent (/mux board agent 40) is still honored for the first split; when no percent is given the equal-division default applies. The percent is not clamped to any pane minimum.
  #
  # EXAMPLES:
  #   1. User types /mux board agent agent on a 120-column terminal; the three panes each get ~1/3 of the width (39/39/40 with the 1-col dividers) instead of the third pane collapsing to 1 column.
  #   2. User resizes the terminal from 120 to 180 columns while a 3-pane mux is active; the next frame re-divides the three panes equally across the new width with no stale cached layout.
  #   3. User types /mux board agent 40 on a 120-column terminal; the Board pane is 40% (47 cols) and the Agent pane takes the remainder (72 cols) — the percent is honored even though the Board pane is now narrower than the old 64-col minimum; the board view degrades gracefully (blank pane) if it cannot fit its columns.
  #
  # ========================================

  Background: User Story
    As a developer supervising multiple agents
    I want to see every mux pane at an equal share of the terminal width
    So that no pane collapses to a sliver when I list more than two panes

  # ========================================
  # SCENARIOS (one per business rule)
  # ========================================
  # R1: three panes divide the width equally
  Scenario: /mux board agent agent divides the width equally across three panes
    Given mux mode is active on a 120-column terminal
    When I submit the slash command "/mux board agent agent"
    Then the grid shows three panes: Board, Agent and Agent
    And each pane gets an equal share of the width (39, 39 and 40 columns with the 1-col dividers; the last pane absorbs the integer-division remainder)
    And no pane collapses to a 1-column sliver

  # R1: four panes divide the width equally
  Scenario: /mux 4 divides the width equally across four panes
    Given mux mode is active on a 200-column terminal
    When I submit the slash command "/mux 4"
    Then the grid shows four panes: Board, Agent, ChangedFiles and Checkpoints
    And each pane gets an equal share of the width (49, 49, 49 and 50 columns with the 3-col dividers; the last pane absorbs the integer-division remainder)

  # R1: two panes still divide equally (backwards compatible)
  Scenario: two panes divide the width equally
    Given mux mode is active on a 120-column terminal
    When I submit the slash command "/mux board agent"
    Then the grid shows two panes: Board and Agent
    And each pane gets an equal share of the width (59 and 60 columns with the 1-col divider; the last pane absorbs the integer-division remainder)

  # R2: no minimum clamps in the default layout math
  Scenario: a board pane narrower than 64 columns is not clamped up
    Given mux mode is active on a 100-column terminal
    When I submit the slash command "/mux board agent agent"
    Then the Board pane is 32 columns wide (an equal third, not clamped to the 64-column minimum)
    And the remaining panes take the other two equal thirds (32 and 34 columns; the last pane absorbs the integer-division remainder)
    And the board view degrades gracefully when it cannot fit its columns

  # R2: the mouse divider drag still produces a non-equal split
  Scenario: dragging the divider produces a non-equal split
    Given mux mode is active with Board and Agent panes at an equal split on a 120-column terminal
    When I press the mouse down on the divider and drag it to the 40 percent position
    Then the Board pane is 40 percent of the width and the Agent pane takes the remainder
    And the drag state is cleared after the release

  # R3: a terminal resize re-divides the panes equally
  Scenario: a terminal resize re-divides the panes equally
    Given mux mode is active with three panes on a 120-column terminal
    When the terminal is resized to 180 columns
    Then the next frame re-divides the three panes equally across the new width (59, 59 and 60 columns)
    And no stale cached pane rect is used

  # R4: an explicit split percent is honored without minimum clamping
  Scenario: /mux board agent 40 honors the percent even below the old minimum
    Given mux mode is active on a 120-column terminal
    When I submit the slash command "/mux board agent 40"
    Then the Board pane is 40 percent of the width (47 columns)
    And the Agent pane takes the remainder (72 columns)
    And the Board pane is not clamped up to the 64-column minimum
