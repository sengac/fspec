# MENU-007 — AST research: agent view menu-bar key/mouse/state surface

Discovery-time analysis of the existing agent-view menu-bar code paths that
MENU-007 changes (single 'Board View' Zone A item, no dropdowns).

## AgentView menu key arms (`src/views/agent/menu_keys.rs`)

Public entry `handle_menu_key(&mut self, key) -> MenuKeyOutcome` (line 118)
gates on `menu_state.focus()`:

- `walk_menu_ring(&mut self, delta: i32)` (line 61) — ring length is now
  `AGENT_ZONE_A.len() + chip_count()`; MENU-007 sets `set_open(None)` after
  every walk (agent bar is dropdown-free).
- `activate_board_view_item(&mut self)` (line 89) — NEW: clears focus,
  emits `Action::BackToBoard`.
- `activate_menu_chip(&mut self, j: usize)` (line 97) — resolves the chip
  session via `menu_snapshot::active_menu_session_ids`, focuses it in the
  store seam, emits `Action::MenuChipActivate(j)`.

## AgentView menu state (`src/views/agent/menu_state.rs`)

`MenuBarState` getters: `focus`, `open` (now always `None` in the agent view
— kept for snapshot plumbing), `bar_row`, `item_rects`, `cells`,
`open_panel`, `chip_count`. `AgentView` re-exposes `menu_focus()` (line 145)
and `open_menu()` (line 150); `locked_store()` (line 157) is the chip
activation test seam.

## Implication

Because the agent bar has exactly one Zone A item, `MenuFocus::Item(i)` is
effectively `Item(0)` only. The ring walk (`walk_menu_ring`) and the
Enter/click arms (`activate_board_view_item`) are the only new behavior;
chip activation is unchanged. The geometry pass
(`components/menu_bar/layout.rs`) was generalized to lay out Zone A from
`MenuSnapshot.zone_a` so the agent snapshot can carry its own single item
list while board/mux keep `CATEGORIES`.
