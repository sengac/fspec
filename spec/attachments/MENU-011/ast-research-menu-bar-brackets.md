# AST research — MENU-011 menu bar bracketed clickable elements

Scope: `rust/fspec-tui/src/components/menu_bar/` (AstGrep `fn $NAME($$$ARGS) { $$$BODY }`,
language rust) + targeted reads of the surface snapshot builders.

## Call map relevant to MENU-011

- `paint_menu_bar(area, buf, snap, theme) -> Option<MenuLayout>` (paint.rs)
  - paints Zone A items from `snap.zone_a` (`category.label`)
  - delegates Zone B cells to `paint_cell(...)` (View / Fold / Chip arms)
  - paints Zone C buttons from `snap.zone_c` (`button.label`) using
    `layout.zone_c_rects`
- `menu_bar_layout(area, snap) -> Option<MenuLayout>` (layout.rs)
  - `zone_a_width(snap)` sums `category.label.width()`
  - item rects built from `category.label.width()`
  - `cell_width(snap, cell, level)` — View: `label.width()`; Fold:
    `format!("+{count}").len()`; Chip: `index_prefix.len() (+ wu.width()+1
    at level 0) + glyph.width()`
  - `zone_c_width(snap)` sums `button.label.width()` + `ZONE_C_GAP`
- `MenuSnapshot { zone_a, focus, open_menu, zone_b, chips, zone_c, clock_ms }`
  (mod.rs) — `ZoneBCell::View { label, active }` / `Chip(i)` /
  `ZoneCButton { label, action }`
- Surface snapshot builders (WHO CALLS THIS):
  - `views/board/menu_snapshot.rs` — `BOARD_ZONE_C = ["New Agent"]`,
    `build_snapshot(...)`
  - `views/agent/menu_render.rs` — `AGENT_ZONE_A` ("Board View"),
    `AGENT_ZONE_C = ["New Agent", "Close Agent [esc]"]`,
    `agent_bar_snapshot(...)`
  - `views/multiplex/menu_snapshot.rs` — `pane_label(kind)` =
    "Board"/"Agent"/"Files"/"Ckpts", `zone_c: &[]`
- Hit-testing consumers of the cached rects (must keep working, R4):
  - `views/board/menu_mouse.rs` — `item_rects` / `cell_rects` /
    `zone_c_rects` hit arms
  - `views/agent/menu_mouse.rs` — same triad via `menu_state`
  - `views/multiplex/menu_mouse.rs` — `menu_item_rects` / `menu_cells`
- Registry consumers that must STAY plain (no brackets):
  - `components/menu_bar/dropdown_paint.rs` — `paint_dropdown_row` uses
    `entry.label`
  - `components/menu_bar_help.rs` — body rows from `entry.key` +
    `entry.label`

## Plan (bracketing as a paint-time transform)

1. `items.rs`: add `MenuCategory::bracketed_label(&self) -> String`
   (`"[ " + label + " ]"`) — single helper shared by layout + painter.
   Registry labels stay plain (dropdown/help dialogs untouched).
2. `layout.rs`: `zone_a_width`, item rects, and `cell_width` use the
   bracketed widths: Zone A `bracketed_label().width()`; Zone B cells
   `+ 2` cells (one space + one bracket on each side); Zone C unchanged
   (labels stored bracketed in the surface `&'static` slices).
3. `paint.rs`: Zone A paints `bracketed_label()`; Zone B `paint_cell`
   wraps each variant's content in `[ ... ]` (prefix `[ `, suffix ` ]`,
   same style per segment); Zone C paints the stored label (already
   bracketed by the surfaces).
4. Surface slices: `BOARD_ZONE_C = ["New Agent"]`, `AGENT_ZONE_C =
   ["New Agent", "Close Agent"]` — stored bracketed (`"[ New Agent ]"`,
   `"[ Close Agent ]"`) so layout width + hit rects + paint agree; the
   `[esc]` suffix is dropped (R3).
5. Mux pane labels flow through the Zone B cell path — no surface change
   needed beyond (2)/(3).
6. All existing proptest overflow bounds keep applying (wider rows just
   truncate earlier via the unchanged R6 ladder).

## Width arithmetic (new)

- Zone A item: `label + 4` cells (`[ `, ` ]`).
- Zone B cell: `content + 4` cells.
- Zone C button: stored label already carries the brackets.
- Inter-item / inter-cell gaps unchanged (1 cell items, 2 cells cells,
  `ZONE_C_GAP` = 2).
