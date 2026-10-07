# MENU-009 — AST Research: Zone C integration points (pre-implementation snapshot)

Research date: 2026-10-06. Purpose: record the AST-level surface of every
integration point MENU-009 touches, so the implementation is wired into the
existing ring/paint/dispatch machinery (not forked).

Verified with AstGrep (rust):
- `MenuSnapshot` struct: `rust/fspec-tui/src/components/menu_bar/mod.rs:93`
- `BoardStore::menu_move(delta)`: `rust/fspec-tui/src/store/board_menu.rs:91`


## 1. Component layer — `rust/fspec-tui/src/components/menu_bar/`

| Entity | File | Pre-MENU-009 shape |
|---|---|---|
| `MenuSnapshot` | `mod.rs` | `zone_a: &'static [MenuCategory]`, `focus: Option<MenuFocus>`, `open_menu: Option<(usize,usize)>`, `zone_b: Vec<ZoneBCell>`, `chips: Vec<MenuChip>`, `clock_ms: u64` |
| `MenuFocus` | `mod.rs` | `Item(usize)` / `ZoneB(usize)` only |
| `MenuSnapshot::advance` | `mod.rs` | `len = zone_a.len() + zone_b.len()`; `None`+1 → `Item(0)`, `None`-1 → last cell |
| `menu_bar_layout(area, snap)` | `layout.rs` | returns `MenuLayout { level, item_rects, separator, cells, cell_rects }`; R6 ladder levels 0..=4; 1-cell R1 padding |
| `paint_menu_bar(area, buf, snap, theme)` | `paint.rs` | `#333333` full-row bg; `inverse_style()` = bg Cyan / fg Black / BOLD for focused Zone A item + open item + active chip |
| `MenuAction` | `items.rs` | `NewAgent` → `Action::OpenAgentView(target)` (R8 caller substitution); no CloseAgent |
| `CATEGORIES` | `items.rs` | `Kanban`/`Tools`/`Settings`/`Help` (26 inner cells) |
| `MenuChip` / `build_chips` | `chips.rs` | `index_prefix("#n ")` + WU id suffix (level 0 only) + status glyph |

**Gap (filled by the implementation):** no `ZoneCButton`, no
`MenuFocus::ZoneC`, no `zone_c` field, no `zone_c_rects` in `MenuLayout`, no
`CloseAgent` variant.

## 2. Board surface — `rust/fspec-tui/src/views/board/` + `store/`

| Entity | File | Pre-MENU-009 shape |
|---|---|---|
| `build_snapshot` | `menu_snapshot.rs` | `MenuSnapshot { zone_a: CATEGORIES, zone_b, chips, .. }` (no `zone_c`) |
| `BoardStore::menu_move(delta)` | `store/board_menu.rs` | `len = RING_COLUMNS(7) + CATEGORIES.len() + menu_chips`; `None` focus enters at first item (+1) / last chip (-1) |
| `handle_menu_keys` | `menu_keys.rs` | `Item` / `ZoneB` focus arms; no `ZoneC` arm |
| `handle_menu_mouse` | `menu_mouse.rs` | bar-row left click: `item_rects` → focus+toggle, `cell_rects` → `MenuChipActivate`; BUG-196 `close_outside` (off-bar + open panel/focus) → inline `MenuDismissBar`, excluding open panel + item rects |
| `MenuBarGeometry` | `menu_mouse.rs` | `bar_y`, `layout: MenuLayout`, `open_panel` cached in `BoardView.last_menu_bar_geometry` |

## 3. Agent surface — `rust/fspec-tui/src/views/agent/`

| Entity | File | Pre-MENU-009 shape |
|---|---|---|
| `AGENT_ZONE_A` | `menu_render.rs` | single `Board View` item (MENU-007 precedent — the `&'static` surface-constant pattern Zone C mirrors) |
| `agent_bar_snapshot` / `paint_menu_bar_row` | `menu_render.rs` | builds `MenuSnapshot { zone_a: AGENT_ZONE_A, .. }`; caches `chip_count` / `item_rects` / `cells` in `MenuBarState` (focused pane only, BUG-163) |
| `MenuBarState` | `menu_state.rs` | `focus`, `open`, `chip_count`, `bar_row`, `item_rects`, `cells`, `open_panel` + `clear_geometry()` |
| `walk_menu_ring(delta)` | `menu_keys.rs` | `len = AGENT_ZONE_A.len() + chip_count`; `Enter` on `Item(_)` → `activate_board_view_item`, on `ZoneB(j)` → `activate_menu_chip`; no `ZoneC` arm |
| `handle_menu_mouse` | `menu_mouse.rs` | left click: item rects → activate Board View; chip rects → activate; empty bar space → de-select (BUG-197 R2) |

## 4. Mux surface — `rust/fspec-tui/src/views/multiplex/`

| Entity | File | Pre-MENU-009 shape |
|---|---|---|
| `build_snapshot` | `menu_snapshot.rs` | `MenuSnapshot { zone_a: CATEGORIES, zone_b = pane labels + global chips, .. }`; state on `MultiplexLayout` (RED CARD 1) |
| ring walk | `app/dispatch_menu.rs` `dispatch_menu_mux` | `MenuSnapshot::advance` per frame against a fresh snapshot — an empty `zone_c` slice flows through with zero effect (regression guard) |

## 5. App dispatch — `rust/fspec-tui/src/app/dispatch_menu.rs`

| Entity | Pre-MENU-009 shape |
|---|---|
| `is_menu_action` | `MenuMove` / `MenuFocusToColumns` / `MenuOpenDropdown` / `MenuCloseDropdown` / `MenuDismissBar` / `MenuDropdownCursor` / `MenuExecuteItem` / `MenuChipActivate` / `MenuMoveToItem` / `MenuFocusPane` — no Zone C token |
| `feed_menu_ring_size` | feeds `set_menu_ring_size(chips)` + mux `refresh_menubar_ring` — no Zone C count |
| `MenuChipActivate` resolution | active-surface aware (board vs agent) — the per-surface resolution pattern the Zone C arm follows |
| `handle_open_agent_view(target)` | `Some(sid)` → navigation target + Agent flip; `None` → first open session, else `CreateSessionDialog` (RPC-097 contract the board `New Agent` button reuses) |
| `handle_agent_exit_choice(CloseSession)` | 6-step teardown (snapshot → board detach → `remove_session_if_open` → `set_current_work_unit(None,None)` → spawned `backend.destroy_session` → `BackToBoard`) — the exact teardown the agent `Close Agent` button emits |

## 6. Existing bus actions reused (no new RPC)

- `Action::OpenAgentView(Option<SessionId>)` — `components/mod.rs`
- `Action::AgentExitChoice { choice: ExitChoice::CloseSession }` — `components/mod.rs`

## 7. New token added by the implementation

- `Action::MenuZoneCActivate(usize)` (index into the surface's `zone_c`
  slice) — board/agent key + mouse arms emit it; `App::dispatch_menu`
  resolves per surface (board: `.`-key semantics via
  `board_store.selected_work_unit()` + `session_for`; agent: `AGENT_ZONE_C`
  registry `MenuAction` with R8 substitution); mux arm is a defensive no-op.
