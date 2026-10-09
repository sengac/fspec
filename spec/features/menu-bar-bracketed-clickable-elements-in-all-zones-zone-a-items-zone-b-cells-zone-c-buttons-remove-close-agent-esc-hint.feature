@done
@ui-enhancement
@menu-bar
@tui-component
@MENU-011
Feature: Menu bar bracketed clickable elements in all zones (Zone A items, Zone B cells, Zone C buttons) + remove Close Agent [esc] hint
  """
  Bracketing is a pure paint-time transform in the shared menu_bar component: Zone A item labels render as ' [ Label ] ' (layout + painter derive the bracketed label from the registry label), Zone B cells render brackets around the existing chip / view-label / fold-marker content (paint only — hit rects keep covering the same painted span via the width math), and Zone C button labels carry the brackets as part of the stored 'label' (so layout width, paint and the cached hit rects all agree). The agent bar's Close Agent label loses its '[esc]' suffix (menu_render.rs AGENT_ZONE_C). The R6 truncation ladder and Zone C drop-before-Zone-B rules are unchanged; all existing overflow proptests still apply to the wider bracketed rows.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: Every Zone B cell is painted bracketed with a space between the word and each bracket — session chips as [ #1 WU-id glyph ] (e.g. [ #1 MENU-001 ● ], [ #1 ● ]) and mux pane view labels as [ Board ] [ Agent ] [ Files ] [ Ckpts ] (the focused pane stays bold + white/theme.fg per BUG-202, the others dim). The R6 truncation ladder semantics are unchanged: WU-id drop, +N folds (painted as [ +1 ]), dropping non-active view labels, and the absolute-minimum level all apply to the bracketed cells.
  #   2. R1: Every Zone A menu item label is painted bracketed with a space between the word and each bracket: [ Kanban ] [ Tools ] [ Settings ] [ Help ] (board/mux) and [ Board View ] (agent view). The brackets are part of the painted label — width, hit-test rects and the inverse-video focus highlight all cover them.
  #   3. R3: Every Zone C button label is painted bracketed with a space between the word and each bracket — [ New Agent ] and [ Close Agent ] — with a 2-cell gap between two buttons. The existing '[esc]' suffix on the agent bar's Close Agent button is REMOVED: the label reads exactly 'Close Agent' (the Esc cascade semantics of MENU-009/BUG-199 are unchanged — only the hint text goes).
  #   4. R4: The bracketed text is the button — width, hit-testing and focus. The layout pass measures labels WITH their brackets, mouse hit rects cover the full bracketed span, and the inverse-video focus highlight (Zone A items, Zone B cells, Zone C buttons) extends over the brackets. Nothing overflows the bar row (the existing proptest bounds still hold).
  #
  # EXAMPLES:
  #   1. Board bar at 120 columns with 2 open sessions (Idle, Running): ' [ Kanban ] [ Tools ] [ Settings ] [ Help ]  │ [ #1 ● ]  [ #2 ⠋ ]  ...  [ New Agent ]' — Zone A items and chips bracketed, New Agent button bracketed, #2's glyph magenta, #333333 bg on every cell
  #   2. Agent bar at 100 columns with 0 open sessions: ' [ Board View ]  ...  [ New Agent ]  [ Close Agent ]' — both Zone C buttons bracketed, NO '[esc]' hint anywhere on the bar; the Close Agent button still runs the AgentEscPressed cascade on activation (later refined by BUG-204 to the always-dialog rule — the bracketing itself is unchanged)
  #   3. Tight 84-column board row with 4 chips (bracketed Zone A = 42 cells): the R6 ladder still runs on the bracketed cells — WU id suffixes drop first (chip reads '[ #1 ● ]'), the 4th chip folds into a single dim '[ +1 ]' marker, nothing overflows the row
  #   4. Mux bar at 120 columns: ' [ Kanban ] [ Tools ] [ Settings ] [ Help ]  │ [ Board ]  [ Files ]  [ #1 ● ]' — the pane view labels are bracketed too (focused pane bold + white per BUG-202); the mux bar still paints no Zone C buttons
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to see every clickable menu bar element bracketed so affordance is obvious at a glance, and drop the noisy [esc] hint
    So that the clickable surface of the bar (Zone A items, Zone B chips and pane labels, Zone C buttons) reads as buttons, with the Esc meaning kept implicit on the Close Agent button

  # ========================================
  # SCENARIOS
  # ========================================
  @menu-bar-paint
  Scenario: The board bar paints its Zone A items bracketed
    Given a board menu bar snapshot with 1 open session and no ring focus
    When the bar is painted into a 120-column row
    Then Zone A reads "[ Kanban ] [ Tools ] [ Settings ] [ Help ]"
    And the Zone B chip paints as "[ #1 ● ]"

  @menu-bar-paint
  Scenario: The agent bar paints its "Board View" item bracketed
    Given an agent menu bar snapshot with 0 open sessions and no ring focus
    When the agent bar is painted into a 100-column row
    Then the left-anchored item reads "[ Board View ]"

  @menu-chips
  Scenario: A session chip paints bracketed with its work-unit id when width allows
    Given a MenuSnapshot with 1 open session (Idle) bound to work unit "MENU-001"
    And a 200-column render area
    When the bar is rendered
    Then the chip reads "[ #1 MENU-001 ● ]"

  @menu-chips
  @menu-geometry
  Scenario: The truncation ladder runs on bracketed cells
    Given a MenuSnapshot with 4 open sessions each bound to a long work-unit id
    When the bar is rendered into a 84-column area
    Then no chip shows a work-unit id suffix and each visible chip reads "[ #n ● ]"
    And the 4th chip is folded into a single dim "[ +1 ]" marker
    And the painted row fits within the area width

  @menu-bar-paint
  @mux
  Scenario: Mux pane view labels paint bracketed
    Given a mux menu bar snapshot with [Board, Files] panes (Board focused) and 1 open session
    When the mux bar is painted into a 120-column row
    Then Zone B reads "[ Board ]  [ Files ]  [ #1 ● ]" with the focused pane label bold
    And the row contains no Zone C button

  @menu-bar-paint
  Scenario: The board bar paints a right-aligned "[ New Agent ]" button
    Given a board menu bar snapshot with 1 open session and no ring focus
    When the bar is painted into a 120-column row
    Then the row ends with the right-aligned "[ New Agent ]" Zone C button
    And the row contains no "Close Agent" button

  @menu-bar-paint
  Scenario: The agent bar paints "[ New Agent ]" then "[ Close Agent ]" right-aligned without the [esc] hint
    Given an agent menu bar snapshot with 0 open sessions and no ring focus
    When the agent bar is painted into a 100-column row
    Then the row ends with "[ New Agent ]" followed by "[ Close Agent ]" right-aligned
    And the row contains no "[esc]" hint

  @menu-bar-paint
  Scenario: With no open sessions the agent bar still shows both bracketed Zone C buttons
    Given an agent menu bar snapshot with zero open sessions
    When the agent bar is painted into a 80-column row
    Then the row ends with "[ New Agent ]" followed by "[ Close Agent ]" right-aligned
    And the buttons are independent of the (empty) chip list

  @keyboard-navigation
  @regression
  Scenario: Enter on the agent's "[ Close Agent ]" still shows the same exit confirmation as Esc
    Given the agent view shows 1 open idle session and the agent bar is painted
    When the ring focuses "Close Agent" and I press Enter
    Then the "Exit Session?" confirmation dialog (Detach / Close Session / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @mouse-events
  @regression
  Scenario: A left click on the board's "[ New Agent ]" button activates it
    # NOTE (BUG-203 supersedes this scenario's Then): the board 'New Agent'
    # gesture now ALWAYS mounts the CreateSessionDialog (payload-free registry
    # mapping) — it never dispatches OpenAgentView with a substituted target.
    Given the board bar is painted with the "[ New Agent ]" Zone C button
    When I left-click the button
    Then the CreateSessionDialog mounts over the board (BUG-203: the button ALWAYS starts a new agent — the OpenAgentView R8 substitution is gone)
    And the board bar's ring focus clears

  @menu-focus
  Scenario: A focused bracketed element's inverse-video highlight covers the brackets
    Given a board menu bar snapshot with the ring focus on the "New Agent" button
    When the bar is painted into a 120-column row
    Then every cell of the "[ New Agent ]" button (brackets included) is styled bg Cyan, fg Black, bold
    And no other Zone C or Zone A cell is inverted

  @menu-geometry
  @unit
  Scenario: The bracketed row never exceeds the area width (proptest)
    Given a board menu bar snapshot with a Zone C "[ New Agent ]" button and any number of chips
    When the bar is painted into any render area width
    Then the painted row (Zone A items, Zone B cells AND Zone C rects) never exceeds the area width

  @menu-bar-paint
  @regression
  Scenario: The dropdown rows and the "u" help dialog keep their plain registry labels
    Given the Kanban dropdown is open on a board bar whose items paint bracketed
    When the dropdown panel and the "Menu bar" help dialog are inspected
    Then the dropdown rows and dialog rows list the plain registry labels (". New Agent", "/ Search", "D FOUNDATION.md", "A Attachments")
    And brackets appear only on the bar row itself, never in the dropdown panel or the help dialog body
