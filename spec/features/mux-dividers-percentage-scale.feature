@bug
@bug-fix
@tui
@ui-enhancement
@mouse-events
@layout
@BUG-166
Feature: Mux dividers — one per inter-pane gap, drag release persists, percentage-scale splits
  """
  Fixes MUX-001/003 divider defects (BUG-166, 2026-08-27). Layout math:
  rust/fspec-tui/src/views/multiplex/layout.rs + new splits.rs (pure
  percentage-scale math); live state in types.rs (divider_rects: Vec<Rect>,
  drag_index/drag_width); render.rs paints every divider; mouse.rs
  hit-tests every divider; navigator.rs routes each divider's drag to the
  right split entry.

  Percentage-scale model (replaces "missing entries mean equal share"):
  - splits stores ONE percentage per inter-pane gap: n panes → n-1 entries,
    splits[i] = pane i's share of the axis (after divider subtraction) in
    percent; the LAST pane always absorbs the integer remainder so the scale
    always sums to 100.
  - Default (fresh entry, /mux n) is equal portions: every entry ≈ 100/n
    (e.g. 50/50 for 2, 33/33/34 for 3, 25/25/25/25 for 4).
  - Changing the pane count RESCALES the existing scale proportionally
    (largest-remainder rounding): adding a pane gives it an equal share
    100/new_n and shrinks the others; removing one re-allocates its share to
    the survivors. Nothing is reset to a bare equal split and no user
    position is thrown away.
  - Every inter-pane gap has its own 1-col/row divider, independently
    draggable. Releasing a divider stores the released percent — the grid
    must NOT snap back to an equal split (the old set_split_from_position
    early-return on an empty splits vec caused the snap-back).
  - Persisted tui.mux splits keep the same shape (Vec<u16>, n-1 entries);
    old empty-vec configs normalize to the equal scale on load.
  """

  Background: User Story
    As a developer supervising multiple agents in mux mode
    I want a draggable divider between every pair of panes that stays where I leave it
    So that I can shape the grid the way I want it and have the shape survive resize, pane-count changes and restarts

  # R17: every inter-pane gap has its own divider
  Scenario: three panes render one divider between every pair of panes
    Given mux mode is active with three panes on a 120-column terminal
    When the grid is rendered
    Then two dividers are painted: one between panes 1 and 2 and one between panes 2 and 3
    And each divider is 1 column wide and spans the full pane height
    And the dividers sit immediately to the right of their left pane

  # R17: vertical orientation gets full-width row dividers
  Scenario: vertical three-pane mux paints one row divider between every pair
    Given mux mode is vertical with three panes on a 40-row terminal
    When the grid is rendered
    Then two full-width row dividers separate the three stacked panes

  # R18: release keeps the position (the reported snap-back bug)
  Scenario: releasing a divider keeps the released position instead of resetting to an equal split
    Given mux mode is active with Board and Agent panes at an equal split on a 120-column terminal
    When I press the mouse down on the divider, drag it right past the midpoint and release
    Then the Board pane keeps its released width (the stored percent of the available width)
    And the panes do NOT return to the equal 59/60 split
    And the same widths are recomputed on the next frame and after a terminal resize

  # R18: the released percent is stored in the config (persists with /mux save)
  Scenario: releasing the second divider stores the percentage in the mux config
    Given mux mode is active with three panes on a 120-column terminal
    When I drag the divider between the second and third panes and release it
    Then the second split entry of the mux config holds the released percent
    And the third pane holds the integer remainder of the scale

  # R17: every divider is independently draggable
  Scenario: dragging the second divider resizes its two adjacent panes
    Given mux mode is active with three equal panes on a 120-column terminal
    When I press the mouse down on the second divider and drag it right
    Then the second pane grows and the third pane shrinks live during the drag
    And the first pane keeps its width
    And releasing stores a non-equal percentage scale in the config

  # R17: the dragged divider is highlighted while the drag is in flight
  Scenario: the dragged divider is highlighted while dragging
    Given mux mode is active with three panes
    When I press the mouse down on the first divider
    Then the first divider is highlighted (cyan) and the other divider is dimmed
    And releasing clears the highlight

  # R19: the scale rescales when panes are ADDED
  Scenario: adding a third pane rescales a 40/60 split proportionally
    Given mux mode is active with Board at 40 percent and Agent at 60 percent
    When I submit the slash command "/mux board agent agent"
    Then the new third pane gets an equal share (one third of the width)
    And the Board and Agent panes shrink proportionally so the whole scale still sums to the full width
    And no user-set position is thrown away: the relative Board-to-Agent ratio is preserved

  # R19: the scale rescales when panes are REMOVED (proportional
  # redistribution — the exact inverse of the add-pane rescale)
  Scenario: removing a pane re-allocates its share to the surviving panes
    Given mux mode is active with three panes at 40, 30 and 30 percent
    When I submit the slash command "/mux board agent"
    Then the surviving panes keep their relative 40-to-30 ratio at the new scale (57 and 43 percent)
    And the two panes fill the full width with no share lost
    And the stored scale has exactly one entry (n-1 entries for n panes)

  # R19: /mux n rescales the current scale to n equal-default positions when only equal splits exist
  Scenario: /mux 4 on an equal two-pane split divides the width equally across four panes
    Given mux mode is active with the default two panes at an equal split on a 200-column terminal
    When I submit the slash command "/mux 4"
    Then each of the four panes gets an equal share of the width
    And the stored scale has three entries that sum with the remainder to 100

  # R20: the persisted config round-trips the full scale
  Scenario: /mux save persists every split entry and a fresh bootstrap restores them
    Given mux mode is active with three panes whose dividers were dragged to a non-equal scale
    When I submit the slash command "/mux save"
    Then the tui.mux key in fspec-config.json contains all n-1 split entries
    And a fresh bootstrap followed by /mux on restores the same non-equal scale
