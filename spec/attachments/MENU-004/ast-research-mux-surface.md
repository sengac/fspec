# AST Research — MENU-004 (Mux surface: single top-of-mux menu bar)

Scope: the seams touched by the MENU-004 implementation. Findings below were
verified via AstGrep + Read against the live `rust/fspec-tui` sources on
2026-09-30.

## 1. `MultiplexLayout` (state seam — RED CARD 1)

File: `rust/fspec-tui/src/views/multiplex/types.rs`
- `pub struct MultiplexLayout` (L54): plain fields, `&mut` through
  `render_with_stores` — no `RefCell` needed (mirrors the MENU-003
  `AgentView` pattern).
- Existing fields: `config`, `pane_rects`, `divider_rects`, `is_dragging`,
  `drag_index`, `drag_width`, `drag_axis`, `focus`, `window_start`,
  `sessions`, `rendered_panes`, `closed_panes`, `pending_new_agent`,
  `body_area`, `pre_mux_view`, `flash_pane`, `flash_clock_ms`.
- GAIN (new fields): `menu_focus: Option<MenuFocus>`,
  `open_menu: Option<(usize, usize)>`, cached item/chip rects
  (`Option<MenuLayout>` from `components::menu_bar::layout`), open-panel rect.
- `impl MultiplexLayout` (L113): accessors + `new()`. GAIN: accessors for the
  bar state + a `reset_menu_state()` called from `disable()`.
- `enable_default` / `enable_with_config` / `disable` live in
  `multiplex/mod.rs` (L69-133) — `disable()` is where bar state is reset.

## 2. `render_with_stores` (render seam)

File: `rust/fspec-tui/src/views/multiplex/render.rs`
- `render_with_stores(layout, area, buf, board_store, agent_store, views)`
  (L34): guard `if !layout.config.enabled || area.height < 3 || area.width < 2`
  (L42). GAIN: raise min height 3 → 4 when the bar is to be painted; keep the
  legacy 3-row path when `area.height < 4` (RED CARD 4 degrade).
- `body` (L61-66): `height = area.height - 1` (footer only). GAIN: `area.y+1
  .. area.height-2` (top bar + bottom footer) when the bar paints.
- Agent pane branch (L91-114): already sets
  `PaneSession { menu_row: false }` (MENU-003 suppression flag — the
  per-agent bar does NOT paint). No change needed for agent panes.
- Board pane branch (L86-90): `views.board.render_with_store(*rect, buf,
  board_store, agent_store)`. GAIN: a `menu_suppressed` flag (mux = true) so
  the board's 4-row header row 3 paints blank.
- After panes + `paint_focus_flash` (L133), BEFORE `paint_dividers`
  (L136) / `paint_footer` (L137): GAIN `paint` of the full-width bar into the
  top row, building the per-frame `MenuSnapshot` (Zone B = pane view labels +
  global chips) and the dropdown panel over the body.
- `MuxRenderViews<'a>` bundle (L26) keeps the arg count under the clippy
  ceiling — the new bar painter reads `layout` + both stores, both already in
  scope. No signature change needed.

## 3. Board suppression flag

File: `rust/fspec-tui/src/views/board/render.rs`
- `render_with_store(view, area, buf, store, agent_store)` (L40): builds the
  per-frame snapshot `menu_snapshot::build_snapshot(agent_store,
  store.menu_focus(), store.open_menu(), now_ms())` (L88-89) and paints the
  bar into header row 3 (`bar_rect = rows.bar_row`, L90).
- GAIN: a `menu_suppressed: bool` param (single-view callers = false, mux
  board pane = true). Suppressed: skip `paint_menu_bar` + `cache_menu_geometry`
  (clear geometry so mouse arms stay inert), leave row 3 blank.
- Callers: `BoardView::render_with_store` (board.rs L276-284, single-view) and
  the mux render branch.

## 4. Navigator + event routing

File: `rust/fspec-tui/src/views/navigator.rs`
- `Navigator` struct (L59): `pub mux: MultiplexLayout`, `pub active_view:
  ViewMode`, children board/agent/changed_files/checkpoints.
- `ViewMode` enum (L30): includes `Mux`.
- `handle_event` (L147): `ViewMode::Mux => self.handle_mux_event(event,
  board_store)`.
- GAIN: `pub fn is_menu_bar_animating(&self) -> bool` (the R-TICK gate) —
  active_view in {Board, Agent, Mux} AND any open session Running/Compacting
  (reuse `AgentViewStore::any_session_busy`).

File: `rust/fspec-tui/src/views/navigator_mux_events.rs`
- `handle_mux_event` (L23): mouse → `mux_mouse::classify_mouse`; keyboard →
  `mux_keys::classify_key`. GAIN: before classifying, if the mux bar is
  focused or a dropdown is open (state on `self.mux`), route to the new
  `multiplex/menu_keys.rs` bar-key arms instead of the focused pane. Files/
  Checkpoints panes still do NOT feed the bar (RED CARD 2).
- `forward_mux_event_to_focused_pane` (L178) unchanged.

File: `rust/fspec-tui/src/views/multiplex/keys.rs`
- `classify_key` (L41): Shift+arrows → FocusPrev/FocusNext; else Forward.
  UNTOUCHED (the bar never enters via Shift+arrows).
- GAIN: a new module `views/multiplex/menu_keys.rs` owning the bar's key arms
  (ring walk via `MenuSnapshot::advance` with zone_b = pane labels + chips;
  true-modal dropdown; char-key forward = clear focus + return Ignored).

## 5. App dispatch

File: `rust/fspec-tui/src/app/dispatch_menu.rs`
- `App::dispatch_menu` (L59): arms route to `BoardStore` (Board) / local
  `AgentView.menu_state` (Agent). GAIN: a `ViewMode::Mux` branch — the
  MenuMove/MenuOpenDropdown/MenuCloseDropdown/MenuDropdownCursor/
  MenuFocusToColumns/MenuMoveToItem/MenuChipActivate/MenuExecuteItem arms
  mutate the mux layout's bar state. ChipActivate in mux: resolve against
  `active_menu_session_ids`, `focus_session_index`, NO view flip. New
  `Action::MenuFocusPane(usize)` for view-label focus (chip vs pane need
  different resolution — DECISION in feature arch note 1).

File: `rust/fspec-tui/src/components/mod.rs`
- Action variants (L1349-1381): `MenuMove`, `MenuFocusToColumns`,
  `MenuOpenDropdown`, `MenuCloseDropdown`, `MenuDropdownCursor`,
  `MenuExecuteItem`, `MenuChipActivate`, `MenuMoveToItem`. GAIN:
  `MenuFocusPane(usize)` (one new variant).

## 6. Tick gate

File: `rust/fspec-tui/src/app/mod.rs`
- `tick_should_draw(should_render, is_busy, is_animating, is_view_loading,
  is_mux_flash_active)` (L117) — 5 operands. GAIN: 6th operand
  `is_menu_bar_animating`.
- Existing 5-operand tests (L129-157) need a `false` 6th arg + 1 new test.

File: `rust/fspec-tui/src/app/run_loop.rs`
- tick arm (L74-117): reads `is_busy`, `is_animating`, `is_view_loading`,
  `is_mux_flash_active` and calls `tick_should_draw`. GAIN: read
  `self.navigator.is_menu_bar_animating()` and pass it.

## 7. Shared helpers (reused, not new)

- `components::menu_bar::MenuSnapshot` (mod.rs L74): `focus`, `open_menu`,
  `zone_b`, `chips`, `clock_ms`; `advance`, `dropdown_cursor`, `chip_for_cell`.
- `ZoneBCell` (mod.rs L63): `View { label, active }` / `Chip(usize)` — the mux
  shape already interleaves view labels.
- `views/board/menu_snapshot.rs`: `active_menu_session_ids` (global, Cleared
  dropped), `chip_inputs`, `zone_b_and_chips`, `build_snapshot` — REUSED for
  the mux bar's global chip list (RED CARD 3).
- `components/menu_bar/paint.rs::paint_menu_bar` + `dropdown_rect` +
  `render_menu_dropdown` — the pure painters the mux render path calls with a
  `clock_ms`.

## 8. Test harness (menu002 pattern)

- `tests/menu002_board_surface.rs`: App + MockBackend; `fresh_app`,
  `drain_pending`, `render_app` into `TestBackend::new(120,24)`; observations
  via `app.board_store().menu_focus()` + inverse-video (bg Cyan) cells.
- GAIN `tests/menu004_mux_surface.rs`: App + MockBackend; enable mux via
  `/mux on` (bug164 pattern) or `navigator_mut().mux.enable_default()`;
  `app.navigator_mut().mux` accessors for `menu_focus()`/`open_menu()`;
  `active_view() == Mux` for no-flip assertions; session seed via
  `Action::SessionCreated`; status via `Action::SessionStatusChanged`.
- Navigator-level (mux001 pattern) available for pure ring/render assertions:
  `Navigator` + `BoardStore` + `AgentViewStore` + `TestBackend`.
