# MENU-003 — AST research: Agent view surface seams

Research date: 2026-09-30. Scope: the exact seams the agent-view menu bar
must touch (layout, chrome paint, key dispatch, navigation, mouse).

## 1. Layout seam — `views/agent.rs` L263-278

`AgentView::pane_layout_constraints(role_height, input_height) -> [Constraint; 5]`:

```rust
[
    Constraint::Length(1),            // header
    Constraint::Length(role_height),  // role banner
    Constraint::Min(0),               // scrollback
    Constraint::Length(1),            // footer
    Constraint::Length(input_height), // input
]
```

Doc comment states it is "Pinned by rpc013-source-shape.feature
(`agent_view_splits_into_scrollback_input_and_footer_rows`)". MENU-003
adds a `menu_row: bool`-gated 6th constraint (`Length(1)` at index 1).

Callers:
- `AgentView::render_with_store` (L281-290) → `render_session_pane(area, buf, store, PaneSession::current_session())` — single-view, flag ON.
- `views/multiplex/render.rs` (MuxPaneKind::Agent arm) → `render_session_pane(...)` with the mux `PaneSession` — flag OFF (MENU-004 paints the mux-level bar).

## 2. Pane render — `views/agent/pane_render.rs` (188 lines)

- `PaneSession { session: Option<SessionId>, is_focused: bool }` — BUG-163
  per-pane targeting; `current_session()` helper. Gains `menu_row: bool`
  (Default = false keeps every existing call site on the pinned 5-list).
- `render_session_pane` (L61-187): splits via the constraint list, builds
  `ChromeAreas { header, role, scrollback, footer, input }` (L99-105),
  caches view-level geometry ONLY when `pane.is_focused` (L111-116,
  L136-138, L156-165 — the BUG-163 rule to extend for the menu-rect
  caches).
- Paint order: `chrome_paint::paint_header_and_role` (L143) →
  scrollback (L155) → `chrome_paint::paint_footer` (L169) → input / ghost
  (L175-186) → popups + turn modal (L177-183, full-pane overlays).
  MENU-003: paint the bar into `areas.menu` right after
  `paint_header_and_role`; paint the dropdown as a full-pane overlay in
  the same slot as `paint_turn_modal` (L183) — the turn-modal precedent
  for anchored overlay painting.

## 3. Chrome areas — `views/agent/chrome_paint.rs` (143 lines)

`ChromeAreas` struct (L16-22) gains `menu: Rect` (height 0 when the flag
is off; `paint_menu_bar` already no-ops on zero height — MENU-001 guard).
`paint_header_and_role` signature unchanged (the bar is painted by the
caller, not here, keeping the header painter pure of menu state).

## 4. Key dispatch seam — `views/agent/dispatch.rs`

- L192: the Home-key arm already checks `self.input.is_empty()` — the
  same seam for the bare-Left entry check.
- MENU-003 early arm placement: BEFORE the input arms.
  `menu_focus.is_some()` → handle ring/dropdown, consume.
  Else `input.is_empty() && bare Left (no modifiers)` →
  `menu_focus = Some(Item(0))`, consume.
  Character keys while bar-focused (closed) → clear `menu_focus`, return
  `Ignored` (cascade types into `MultiLineInput` — R9).
- Shift+Left/Right session cycling arms stay untouched (modifiers present
  → the bare-Left guard never fires).

## 5. Navigation resolution — `store/agent_view/navigation.rs`

- `AgentViewStore::navigate_next / navigate_prev` (L41-69) and
  `current_session_index` / `open_sessions()` — the existing Shift+arrow
  path. MENU-003's `MenuChipActivate(j)` (agent-view context) resolves to
  `open_sessions[j]` via the same store; factor a `set_session_index_clamped`
  shared with the Shift+arrow dispatch arm (DRY).
- Board context (MENU-002, unchanged): `MenuChipActivate(j)` →
  `OpenAgentView(Some(open_sessions[j].id))` — flips to the agent view.
  `App::dispatch_menu` (src/app/dispatch_menu.rs, 110 lines) gains the
  context split on `navigator.active_view`.

## 6. State — `AgentView` struct (views/agent.rs L111-169)

Gains plain fields (no `RefCell` — `render_session_pane` takes `&mut
self`): `menu_focus: Option<MenuFocus>`, `open_menu: Option<(usize, usize)>`
(category, cursor row), `last_menu_item_rects: Option<Vec<Rect>>`,
`last_menu_chip_rects: Option<Vec<Rect>>` — refreshed by the focused pane
only (BUG-163). `MultiLineInput` (`multiline_input.rs`) exposes
`is_empty()` for the entry guard (verified at the Home-key arm).

## 7. Mouse seam — `views/agent/mouse_dispatch.rs` (295 lines, at the ceiling)

Gains the bar-row hit-test BEFORE the input/scrollback tests, using the
cached `last_menu_item_rects` / `last_menu_chip_rects`. NOTE: the file is
at 295/299 lines under the rpc049 300-LoC ceiling for `src/views/agent/` —
the hit-test logic should live in a NEW sibling module
(`menu_mouse.rs`-style, mirroring the board's MENU-002 split) with
`mouse_dispatch.rs` delegating.

## 8. Pinned constraints that MUST keep passing

- `tests/source_shape_rpc013.rs` (rpc013-source-shape.feature) —
  `agent_view_splits_into_scrollback_input_and_footer_rows` pins the
  5-constraint list byte-identically. Flag-off path unchanged.
- `tests/app_with_mock_backend*.rs` + mux pane snapshots — agent frames
  painted with flag-on shift by one row; with flag-off they stay
  byte-identical. Single-view tests that render through
  `render_with_store` WILL gain the row (flag-on) — those snapshots need
  regeneration; mux pane snapshots (flag-off) do not.
- `components/mod.rs` — NO new Action variants (the 8 `Menu*` variants
  from MENU-002 are reused; the rpc094 ceiling stays at 1418).
- `src/views/agent/` 300-LoC ceiling (rpc049) — every new agent-view menu
  module must land < 300 lines; `mouse_dispatch.rs` at 295 has no room.
