@done
@bug
@menu-bar
@tui-component
@agent-view
@bug-195
Feature: BUG-195 — clicking a session chip in the Agent view switches to that session
  """
  Regression from MENU-003 (agent-view-surface-2-zone-bar-row-under-session-header):
  in the Agent view a left click on a session chip (the '#n' agent number) did
  nothing — the bar focus cleared but the pane kept showing the previous
  session. The same click from the Board view worked (it flips INTO the Agent
  view on that session).

  Root cause (two stacked no-ops in production):
  1. AgentView::activate_menu_chip focuses the session through the
     `store_handle` test seam — `None` in production — then emits
     Action::MenuChipActivate on the action bus.
  2. App::dispatch_menu's MenuChipActivate arm ASSUMED the agent view already
     focused the session (comment: "the chip's session focus already happened
     in-view") and early-returned for ViewMode::Agent — but with the seam
     None, step 1 focused nothing. The keyboard path (Enter on a focused chip)
     is affected the same way: the emitted action hits the same early-return.

  Fix: the AGENT branch of `App::dispatch_menu`'s MenuChipActivate arm
  resolves the chip against the PAINTED chip list (`menu_snapshot` parity —
  open sessions, `Cleared` dropped), and switches the focused open-session
  slot through `App::switch_to_session_index` (the RPC-024 draft round-trip
  path shared with the Shift+Left/Right cycle: snapshot the outgoing draft,
  focus the incoming slot, restore the incoming draft, refresh supervisors,
  re-probe the exec-stdin overlay). No view flip: the active view is already
  Agent. The Board arm (MENU-002: focus + flip to Agent) is unchanged; the
  Mux arm (MENU-004: focus without leaving Mux) is unchanged.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R1 (dispatch, Agent view): `MenuChipActivate(index)` on the AGENT
  #      surface resolves `index` against the painted chip list (open
  #      sessions, `Cleared` dropped — `menu_snapshot` parity) and runs the
  #      RPC-024 session switch to that open-session slot.
  #   2. R2 (draft round-trip): the switch reuses `App::switch_to_session_index`
  #      — the outgoing session's live input draft is snapshotted into its
  #      `input_draft`, the incoming session's saved draft is restored into
  #      the shared MultiLineInput.
  #   3. R3 (parity): after a chip activation in the Agent view the pane
  #      shows the clicked session's scrollback and the incoming session's
  #      draft — identical to navigating with Shift+Right/Left or to
  #      clicking the same chip from the Board view (minus the view flip).
  #   4. R4 (no regression): the Board arm (flip to the Agent view) and the
  #      Mux arm (focus without leaving Mux) behave exactly as before.
  #
  # EXAMPLES:
  #   1. 3 open sessions, Agent view on s-1: click chip #2 → the Agent pane
  #      shows s-2 (store's current session flips), the bar focus clears.
  #   2. 3 open sessions, Agent view on s-1 with a live draft "hi" in the
  #      input: click chip #3 → s-1's draft is saved (reappears on return),
  #      the Agent pane shows s-3 with its saved (empty) draft.
  #   3. 3 open sessions, Agent view: walk the ring to chip #2 and press
  #      Enter → the same switch as the click (the keyboard path was
  #      affected by the same early-return).
  #
  # ========================================
  Background: User Story
    As a TUI user (agent view)
    I want clicking my agent number in the menu bar to switch to that session
    So that chip switching behaves the same on every surface (board, agent, mux)

  Scenario: Clicking a chip in the Agent view switches to that session
    Given the App is in the Agent view with 3 open sessions
    And session s-1 is the focused session
    When I click chip #2 in the menu bar
    Then the agent pane shows session s-2

  Scenario: A chip click in the Agent view keeps the draft round-trip
    Given the App is in the Agent view with 3 open sessions
    And session s-1 is focused with a live input draft of "hi"
    When I click chip #3 in the menu bar
    Then the agent pane shows session s-3
    And session s-1's saved draft is "hi"

  Scenario: Enter on a chip in the Agent view switches to that session
    Given the App is in the Agent view with 3 open sessions
    And session s-1 is the focused session
    And the menu bar ring is focused on chip #2
    When I press Enter once
    Then the agent pane shows session s-2

  Scenario: The board chip activation still flips to the Agent view
    Given the App is in the Board view with 3 open sessions
    And the menu bar ring is focused on chip #2
    When I press Enter once
    Then the active view is the Agent view showing session s-2
