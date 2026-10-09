@done
@menu-009
@menu-bar
@menu-ring
@tui-component
@board-view
@agent-view
@mux
Feature: Menu bar Zone C — right-aligned New Agent / Close Agent buttons
  """
  Adds a third zone (Zone C) to the shared 2-zone menu bar component (MENU-001):
  right-aligned action buttons painted at the right edge of the bar row.
  The board bar paints ONE button (New Agent); the agent bar paints TWO
  (New Agent, then "Close Agent [esc]" left-to-right); the mux top bar
  paints NONE (empty zone — Q1). Both buttons reuse EXISTING bus
  actions (New Agent -> the New Agent semantics of the surface, Close
  Agent -> the AgentEscPressed cascade — BUG-199 superseded the direct
  AgentExitChoice{CloseSession} teardown) — no new backend/RPC work.
  The buttons join the unified ring focus (R3) with the same
  inverse-video highlight as Zone A items (R4), are mouse-clickable
  (R6: a click never fires the click-away dismiss), and drop before
  Zone B content when the row cannot afford their width (R5, the R6
  ladder otherwise unchanged; the proptest bound extends to their rects).

  BUG-199 supersession (the agent-bar button semantics were fixed in
  spec/features/bug199-agent-bar-zone-c-new-agent-close-agent-esc-semantics.feature):
  the agent bar's "New Agent" button starts a NEW agent (the
  CreateSessionDialog mounts as an overlay) instead of re-entering the
  current session; "Close Agent" is the Esc gesture (the AgentEscPressed
  cascade) and its label carries the trailing "[esc]" hint. The board
  surface is unchanged.

  BUG-204 supersession (the "Close Agent" button semantics were refined in
  spec/features/bug204-close-agent-button-always-shows-exit-confirmation-dialog.feature):
  the agent bar's "Close Agent" button ALWAYS mounts the "Exit Session?"
  dialog (RPC-098) — it no longer runs the AgentEscPressed cascade's
  interrupt / draft-clear branches (those belong to the physical Esc key
  only). The board surface is unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1: The board bar paints exactly one right-aligned Zone C button
  #      labeled "New Agent"; the agent bar paints two right-aligned Zone C
  #      buttons "New Agent" then "Close Agent [esc]" (left to right —
  #      BUG-199 R3 added the [esc] hint); the mux top bar paints NO Zone
  #      C buttons.
  #   2. R2: Both buttons reuse EXISTING bus actions — the board's New
  #      Agent emits Action::OpenAgentView(target) (the '.'-key R8
  # NOTE (BUG-203): the 'New Agent' R8 substitution described here is SUPERSEDED
  # for the BOARD surface — the registry mapping is now payload-free (MenuAction
  # NewAgent → Action::OpenCreateSessionDialog { preselect: None }); the board
  # 'New Agent' gesture ALWAYS mounts the CreateSessionDialog and never resumes
  # a session. Shift+Right keeps the cycle/resume semantics. See
  # spec/features/board-new-agent-gesture-prompts-create-session-dialog.feature
  #      substitution); the agent's buttons (BUG-199 supersession) — New
  #      Agent mounts the CreateSessionDialog (start a NEW agent) and
  #      "Close Agent [esc]" ALWAYS mounts the "Exit Session?" dialog
  #      (BUG-204 supersession — no longer the AgentEscPressed cascade).
  #   3. R3: Zone C buttons join the unified ring focus — ring order is
  #      items -> Zone B -> Zone C -> wrap; Left from the first item wraps
  #      to the last Zone C button; Right from the last chip lands on the
  #      first Zone C button.
  #   4. R4: Zone C buttons carry the SAME inverse-video ring-focus
  #      highlight (bg Cyan / fg Black / bold) as Zone A items when the
  #      ring focus lands on them.
  #   5. R5: When the row cannot afford Zone C's width, Zone C drops
  #      BEFORE Zone B content (the Zone B ladder otherwise unchanged);
  #      the painted row never exceeds the area width (proptest).
  #   6. R6: A left click on a Zone C button MUST NOT fire
  #      MenuDismissBar (the BUG-196/197 "outside" close gesture) and MUST
  #      NOT re-open a dropdown — the board's close_outside hit-test
  #      excludes the Zone C rects.
  #
  # EXAMPLES:
  #   1. Board bar at 120 cols with 1 open session: " Kanban Tools
  #      Settings Help │ #1 ●" left-anchored, "New Agent" right-aligned at
  #      the row's edge; no "Close Agent" anywhere.
  #   2. Agent bar at 100 cols with 0 open sessions: " Board View"
  #      left-anchored, "New Agent  Close Agent [esc]" right-aligned —
  #      the buttons paint even with an empty chip list (BUG-199 R3
  #      label).
  #   3. Ring walk on the board: Right from the last chip lands on
  #      "New Agent"; Right again wraps to the Kanban column/item; Left
  #      from the Kanban item lands on "New Agent" (the last Zone C button).
  #   4. Ring walk on the agent: Right from the last chip lands on
  #      "New Agent", Right again on "Close Agent", Right again wraps to
  #      "Board View"; Left from "Board View" wraps to "Close Agent".
  #   5. A 40-col board row (1 chip bound to a long WU id): "New Agent" no
  #      longer fits — the row drops Zone C but keeps the chip (Zone B
  #      ladder step 1, WU id dropped), painted row still within the width.
  #   6. Board with the Kanban dropdown open: a left click on "New Agent"
  #      executes it (OpenAgentView) — the dropdown closes, the bar is
  #      de-selected, NO MenuDismissBar double-fire, no re-open.
  #
  # ========================================
  Background: User Story
    As a TUI user
    I want to start or close an agent directly from the right-aligned Zone C buttons on the menu bar
    So that the New Agent / Close Agent actions are one keystroke away on every surface without opening a dropdown

  # ========================================
  # SCENARIOS
  # ========================================
  @menu-bar-paint
  Scenario: The board bar paints a right-aligned "New Agent" button and no "Close Agent"
    Given a board menu bar snapshot with 1 open session and no ring focus
    When the bar is painted into a 120-column row
    Then the row ends with the right-aligned "New Agent" Zone C button
    And the row contains no "Close Agent" button
    And the left-anchored Zone A items and the Zone B chip are unchanged

  @menu-bar-paint
  Scenario: The agent bar paints "New Agent" then "Close Agent [esc]" right-aligned
    Given an agent menu bar snapshot with 0 open sessions and no ring focus
    When the agent bar is painted into a 100-column row
    Then the row ends with "New Agent" followed by "Close Agent [esc]" right-aligned
    And the left-anchored "Board View" item is unchanged

  @menu-bar-paint
  @regression
  Scenario: The mux bar paints no Zone C buttons — the row is byte-identical to pre-MENU-009
    Given a mux menu bar snapshot with 2 panes and 1 open session
    When the mux bar is painted into a 120-column row
    Then the row contains no Zone C button
    And the Zone A items, the pane labels and the chip paint exactly as before

  @keyboard-navigation
  Scenario: Enter on the board's "New Agent" button activates the "."-key semantics
    Given the board bar is painted with the "New Agent" Zone C button and no open sessions
    When the ring focuses "New Agent" and I press Enter
    Then the CreateSessionDialog mounts over the board (the "."-key None-target path)
    And the board bar's ring focus clears

  @mouse-events
  Scenario: A left click on the board's "New Agent" button activates it
    # NOTE (BUG-203 supersedes this scenario's Then): the board 'New Agent'
    # gesture now ALWAYS mounts the CreateSessionDialog (payload-free registry
    # mapping) — it never dispatches OpenAgentView with a substituted target.
    Given the board bar is painted with the "New Agent" Zone C button
    When I left-click the "New Agent" button
    Then the CreateSessionDialog mounts over the board (BUG-203: the button ALWAYS starts a new agent — the OpenAgentView R8 substitution is gone)
    And the board bar's ring focus clears

  @keyboard-navigation
  @regression
  Scenario: Enter on the agent's "New Agent" button starts a new agent (BUG-199 R1)
    Given the agent bar is painted with "New Agent" and "Close Agent [esc]" and 1 open session
    When the ring focuses the agent bar's "New Agent" and I press Enter
    Then the CreateSessionDialog mounts over the Agent view (the current session is untouched)
    And the agent bar's ring focus clears

  @keyboard-navigation
  @regression
  Scenario: Enter on the agent's "Close Agent [esc]" shows the same exit confirmation as Esc (BUG-199 R2)
    Given the agent view shows 1 open idle session and the agent bar is painted
    When the ring focuses "Close Agent [esc]" and I press Enter
    Then the "Exit Session?" confirmation dialog (Detach / Close Session / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @mouse-events
  @regression
  Scenario: A left click on the agent's "Close Agent [esc]" shows the same exit confirmation as Esc (BUG-199 R2)
    Given the agent view shows 1 open idle session and the agent bar is painted
    When I left-click the "Close Agent [esc]" button
    Then the "Exit Session?" confirmation dialog (Detach / Close Session / Cancel) is shown
    And the session is not destroyed until an option is committed
    And the agent bar's ring focus clears

  @menu-focus
  Scenario: A focused Zone C button carries the inverse-video ring highlight
    Given a board menu bar snapshot with the ring focus on "New Agent"
    When the bar is painted into a 120-column row
    Then the "New Agent" button's cells are styled bg Cyan, fg Black, bold
    And no other Zone C or Zone A cell is inverted

  @menu-ring
  Scenario: The ring walks last chip -> "New Agent" -> columns on the board
    Given the board ring has 7 columns, 4 items and 1 chip
    When the ring walks Right from the last chip
    Then "New Agent" (the only Zone C button) holds the ring focus
    When the ring walks Right again
    Then the focus wraps to the first kanban column (no bar highlight)
    When the ring walks Left from the first item
    Then "New Agent" holds the ring focus

  @menu-ring
  Scenario: The ring wraps through both Zone C buttons on the agent bar
    Given the agent bar has the "Board View" item, 1 chip and 2 Zone C buttons
    When the ring walks Right from the last chip
    Then "New Agent" holds the ring focus
    When the ring walks Right once more
    Then "Close Agent" holds the ring focus
    When the ring walks Right once more
    Then "Board View" holds the ring focus (the wrap)
    When the ring walks Left from "Board View"
    Then "Close Agent" holds the ring focus (the reverse wrap)

  @menu-geometry
  Scenario: Zone C drops before Zone B content when the row cannot afford it
    Given a board menu bar snapshot with 1 chip bound to a long work-unit id
    When the bar is painted into a 40-column row
    Then the row contains no Zone C button
    And the chip still paints (Zone B ladder step 1, WU id dropped)
    And the painted row fits within the area width

  @menu-geometry
  @unit
  Scenario: The Zone C truncation bound holds for any width and chip count (proptest)
    Given a board menu bar snapshot with a Zone C "New Agent" button and any number of chips
    When the bar is painted into any render area width
    Then the painted row (Zone A items, Zone B cells AND Zone C rects) never exceeds the area width

  @mouse-events
  Scenario: A Zone C click with an open dropdown does NOT fire the click-away dismiss
    Given the board bar's Kanban dropdown is open and the "New Agent" button is painted
    When I left-click the "New Agent" button
    Then OpenAgentView dispatches (the button executes)
    And NO MenuDismissBar gesture fires (the click is not "outside")
    And the dropdown closes and the bar de-selects exactly once (the execute path)

  @empty
  Scenario: With no open sessions the agent bar still shows both Zone C buttons
    Given an agent menu bar snapshot with zero open sessions
    When the agent bar is painted into a 80-column row
    Then the row ends with "New Agent" followed by "Close Agent [esc]" right-aligned
    And the buttons are independent of the (empty) chip list
