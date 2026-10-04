# MENU-006 AST Research — active session chip highlight

## Scope
- `rust/fspec-tui/src/components/menu_bar/` — shared 2-zone menu bar (snapshot, geometry, painter)
- `rust/fspec-tui/src/views/{board,agent,multiplex}/menu_*.rs` — the three surfaces' snapshot builders
- `rust/fspec-tui/src/store/agent_view.rs` — `AgentViewStore` session focus slots

## Query: `pub struct $NAME { $$$FIELDS }` in components/menu_bar
- MenuSnapshot (mod.rs:74) — per-frame owned snapshot: focus, open_menu, zone_b, chips, clock_ms
- MenuEntry / MenuCategory (items.rs) — the MenuCategories registry
- MenuLayout (layout.rs:50) — geometry: level, item_rects, separator, cells, cell_rects
- **MenuChip (chips.rs:30)** — Zone B chip: index, glyph, glyph_style, wu_id, **active** (MENU-006)
- **ChipInput (chips.rs:48)** — per-session builder input: index, status, wu_id, **active** (MENU-006)

## Query: `fn $NAME($$$ARGS) { $$$BODY }` in components/menu_bar (production fns)
- paint.rs: paint_menu_bar, **focused_cell** (the highlight decision — now ORs `MenuChip.active`
  with the ring focus), paint_cell, paint_text, inverse_style
- chips.rs: build_chips, status_glyph, index_prefix
- layout.rs: menu_bar_layout, display_cells, cell_width, zone_a_width, zone_b_width, chip_index
- mod.rs: MenuSnapshot::advance / dropdown_cursor / chip_for_cell
- dropdown*.rs: dropdown_rect, scroll_window, render_menu_dropdown, row truncation

## Query: snapshot builders per surface (integration points)
- views/board/menu_snapshot.rs: active_menu_session_ids, **chip_inputs** (decides `active` from
  `store.current_session()`), zone_b_and_chips, build_snapshot
- views/agent/menu_render.rs: paint_menu_bar_row (reuses board's build_snapshot)
- views/multiplex/menu_snapshot.rs: build_snapshot (reuses board's zone_b_and_chips; chips are GLOBAL)

## Query: `AgentViewStore` focus slots (store/agent_view.rs)
- open_sessions, current_session_index (132), focus_session_index (174),
  current_session (204), session_index_for (session_indexing.rs:19)

## Conclusion
A per-chip `active` flag flows store → ChipInput → MenuChip → MenuSnapshot →
painter (focused_cell OR). All three surfaces (board, agent, mux) build their
snapshots through the shared `chip_inputs` / `build_snapshot` helpers, so the
highlight is implemented ONCE in the shared painter and needs no per-surface
wiring. `SessionContext` / `current_session` semantics: "focused session" =
`current_session_index` (RPC-024 focus slot).
