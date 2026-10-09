@done
@bug
@menu-bar
@mux
@tui-component
@bug-202
@BUG-202
Feature: Mux bar — the focused pane's view label paints the bar palette, not a standalone cyan
  """
  Regression from MENU-011 (menu-bar-bracketed-clickable-elements-in-all-zones) —
  the focused pane's view label on the mux menu bar (e.g. the [ Board ] label
  when the Board pane is focused) is hard to read: it paints a standalone
  CYAN foreground on the bar's #333333 row, while every other bar element
  paints white (theme.fg) or dim (theme.dim). On a terminal with a
  blue-tinted palette the cyan label blends into the blue background and
  the label is barely visible.

  Root cause: the shared menu bar painter (rust/fspec-tui/src/components/menu_bar/paint.rs,
  the DisplayCell::View arm of paint_cell) styles the ACTIVE view label
  `fg(Color::Cyan)` + BOLD. The active flag is set only by the mux snapshot
  builder (views/multiplex/menu_snapshot.rs: the focused pane's label).

  Fix: the active view label styles BOLD + theme.fg (white) — it stays
  visually distinct from the inactive (dim) labels, but no longer carries
  a standalone cyan foreground. The inverse-video selection highlight
  (bg Cyan / fg Black — the ring focus and the selected session chip) is
  the ONLY place cyan appears in the bar row, so 'focused pane' and
  'ring-focused' stay visually distinguishable.

  No new code surface: one styling expression in the shared painter; the
  layout pass, the hit-test rects and the truncation ladder are untouched.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: In the mux menu bar, a pane view label's ACTIVE (focused-pane)
  #      styling is BOLD + theme.fg (white) — never a standalone cyan
  #      foreground. Inactive labels stay theme.dim. The inverse-video
  #      selection highlight (bg Cyan / fg Black, the ring focus / selected
  #      chip) is the ONLY place cyan appears in the bar row, so 'focused
  #      pane' (active) and 'ring-focused' are visually distinguishable and
  #      the active label reads consistently with the rest of the bar.
  #   2. R2 (no regression): only the view-label styling changes.
  #      Session-chip styling is untouched — status glyphs keep their own
  #      colors (Running braille = magenta, Compacting = cyan, Paused =
  #      yellow, Interrupted = red, Idle = green dim), the WU id suffix
  #      stays dim, the ring focus / selected-chip inverse highlight stays
  #      bg Cyan / fg Black, and every other bar element (Zone A items,
  #      Zone C buttons, separator) keeps its existing styling.
  #
  # EXAMPLES:
  #   1. Mux grid [Board | Files | Ckpts], Board pane focused, no ring focus:
  #      the '[ Board ]' label paints BOLD + theme.fg (white) while
  #      '[ Files ]' and '[ Ckpts ]' paint theme.dim — no cell in the row
  #      carries a cyan foreground.
  #   2. Mux grid [Board | Agent], an Agent pane focused showing session #1:
  #      chip '[ #1 ● ]' carries the selected-item inverse highlight (cyan
  #      background, black text) while the inactive '[ Board ]' label stays
  #      dim — the two kinds of 'active' never share the same color treatment
  #      (only the inverse highlight carries cyan).
  #
  # ========================================
  Background: User Story
    As a TUI user in mux mode
    I want to read the focused pane's view label on the shared menu bar
    So that see it in the same readable palette as every other bar element (not a standalone cyan foreground that blends into the blue-tinted background)

  @bug-202
  Scenario: The focused pane's view label paints bold white instead of cyan
    Given the mux grid is [Board | Files | Ckpts] with no open sessions and the Board pane is focused
    When the mux view renders
    Then the '[ Board ]' label paints bold with the primary foreground (theme.fg — white)
    And the '[ Files ]' and '[ Ckpts ]' labels paint the dimmed foreground (theme.dim)
    And no text cell in the top menu bar row carries a cyan foreground

  @bug-202
  @integration
  Scenario: The active view label and the selected-chip highlight stay distinguishable
    Given the mux grid is [Board | Agent] with 1 open session and the Agent pane is focused showing session #1
    When the mux view renders
    Then chip '[ #1 ● ]' carries the selected-item highlight (cyan background, black text)
    And the inactive '[ Board ]' view label stays dimmed (theme.dim) on the bar background
    And no text cell in the top menu bar row carries a cyan foreground outside the selected chip's cells

  @bug-202
  @regression
  Scenario: Everything else on the bar keeps its existing styling
    Given the mux grid is [Board | Agent | Agent] with 1 Running and 1 Idle session and the Board pane is focused
    When the mux view renders
    Then the Zone A items and the '[ New Agent ]'-free row paint exactly as before (white items, dim separator)
    And the Running chip's braille glyph stays magenta and the Idle chip's dot stays green-dimmed
    And the focused pane's '[ Board ]' label is bold white — the only view-label change
