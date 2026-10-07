@done
@menu-bar
@tui-component
@keyboard-navigation
@agent-view
@MENU-007
Feature: Agent view menu bar — single 'Board View' item instead of board-only Actions/Help menus
  """
  The 2-zone menu bar row under the SessionHeader (MENU-003) currently
  paints the board's Zone A items ('Actions', 'Help') in the AGENT view.
  Those items open board surfaces (search, checkpoints, changed files,
  FOUNDATION.md, mux, attachments, help, exit) that the agent view does
  not host — the user does not see the board in that view, so the items
  are dead weight.

  MENU-007: in the agent view the bar's Zone A becomes a SINGLE item
  labelled 'Board View'. It paints the same selected-item inverse-video
  highlight (bg Cyan / fg Black / bold) when the ring focuses it, but
  opens NO dropdown — it is a plain activation button. Clicking it OR
  pressing Enter while it is focused returns to the Board view
  (Action::BackToBoard semantics: single-view flips active_view to
  Board; an active mux grid keeps its existing BackToBoard behavior).
  The bar focus clears after activation (keyboard + mouse parity).

  Zone B is UNCHANGED in the agent view: one chip per open session, and
  chip activation (click / Enter) still switches to that session's pane
  (BUG-195 behavior). The agent ring is now 'Board View' → chip #1 →
  … → chip #N → 'New Agent' → 'Close Agent' → wrap (MENU-009: the two
  Zone C buttons are the ring's last stops); bare Left on an empty input
  still enters at Item(0), which is now 'Board View'.

  The board bar (MENU-002) and the mux bar (MENU-004) are UNCHANGED —
  they keep their Zone A items + dropdowns ('Kanban'/'Tools'/'Settings'/'Help' after MENU-008, was 'Actions'/'Help'). The change is
  surface-local to the agent view:
    - views/agent/menu_render.rs — build the agent bar's MenuSnapshot
      with a single 'Board View' Zone A item instead of CATEGORIES.
    - views/agent/menu_keys.rs   — Enter on Item(0) emits
      Action::BackToBoard instead of opening a dropdown; the ring
      length is 1 item + chips; the dropdown key arms are unreachable
      (no dropdown can open in the agent view).
    - views/agent/menu_mouse.rs  — clicking the item emits
      Action::BackToBoard (no focus+open, no close-outside panel).
    - paint.rs — Zone A items are painted from a per-surface item
      list (board/mux keep CATEGORIES; the agent passes its own
      single-item list), so the shared painter stays surface-agnostic.

  Architecture note: the shared MenuCategories registry
  (components/menu_bar/items.rs) stays the single source of truth for
  the board/mux surfaces. The agent bar's Zone A is a surface-local
  constant (a one-entry category list) so the board/mux registries,
  help dialogs and 'u' menu-bar help dialog (MENU-005) are untouched.
  """

  Background: User Story
    As an agent-view TUI user
    I want the menu bar's Zone A to be a single 'Board View' item
    So that I get back to the board with one click or Enter and never see menu items for surfaces I cannot reach from here

  # Supersession (MENU-008): the board's first Zone A item was renamed
  # 'Actions' → 'Kanban' (and Tools/Settings items added). The agent view
  # still shows NO board items at all — 'Board View' only.
  Scenario: The agent bar paints a single 'Board View' Zone A item instead of Actions and Help
    Given the agent pane has 2 open sessions (one running, one idle) and an empty input
    When the agent pane renders
    Then the bar row shows the 'Board View' item and the dim separator and the two session chips
    And the bar row does not show the 'Kanban' item (MENU-008: the board-only 'Actions' item was renamed to 'Kanban')
    And the bar row does not show the 'Help' item

  Scenario: The 'Board View' item paints the selected-item highlight when the ring focuses it
    Given the agent pane has 1 open session and an empty input
    When I press bare Left once
    Then the 'Board View' item paints inverse-video (bg Cyan fg Black bold) with no dropdown open

  Scenario: Enter on 'Board View' returns to the Board view
    Given the App is in the Agent view with 2 open sessions and the bar is focused on the 'Board View' item
    When I press Enter once
    Then the active view is the Board view
    And the agent bar highlight clears

  Scenario: Clicking 'Board View' returns to the Board view
    Given the App is in the Agent view with 2 open sessions
    When I click the 'Board View' item in the bar
    Then the active view is the Board view
    And no dropdown panel paints over the pane

  Scenario: The agent ring wraps from 'Board View' through the chips and back
    Given the agent pane has 2 open sessions and the menu bar is focused on the 'Board View' item
    When I press Right five times
    Then the focus lands on chip #1 then chip #2 then 'New Agent' then 'Close Agent' and finally back on 'Board View'

  Scenario: Left from 'Board View' wraps to the last Zone C button
    Given the agent pane has 3 open sessions and the menu bar is focused on the 'Board View' item
    When I press Left once
    Then the 'Close Agent' button paints inverse-video (the ring's last stop — MENU-009)

  Scenario: Chip activation in the agent view is unchanged
    Given the App is in the Agent view with 3 open sessions
    When I click chip #2 in the menu bar
    Then the agent pane shows session s-2
    And the active view is still the Agent view

  Scenario: The agent bar with no open sessions shows only 'Board View'
    Given the agent pane has no open sessions
    When the agent pane renders
    Then the bar row shows only the 'Board View' item with no separator and no chips

  Scenario: Board and mux bars keep their Actions and Help items
    Given the App is in the Board view
    When I render the board header row
    Then the board menu bar still shows the 'Kanban' item and the 'Help' item (MENU-008)
