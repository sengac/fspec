# MUX-008 AST research — mux focus flash geometry change

User directive (2026-08-31): the focus flash must scan bottom-to-top (not
right-to-left) and the settled final frame must be a 1-row-high bar at the
pane's TOP (not a full-height 2-column strip at the left edge).

## AST search results

### `flash_cells` — the pure pattern fn (the ONLY geometry change site)

Pattern: `pub fn $NAME($$$ARGS) -> $$$RET { $$$BODY }`
Scope: `rust/fspec-tui/src/views/multiplex`

- `rust/fspec-tui/src/views/multiplex/flash.rs:51` —
  `pub fn flash_cells(rect: Rect, clock_ms: u64) -> Vec<(u16, u16)>`

Current math (X-axis strip):
- `travel = width - 2` (saturating), `offset = travel * local / LAST_PAINT_MS`
- `left_col = travel - offset` → 2-column strip travels `w-2` → `0`
- degenerate guard: `width == 0 || height == 0` → empty vec

New math (Y-axis, 1-row strip):
- `travel = height - 1` (saturating), `offset = travel * local / LAST_PAINT_MS`
- `top_row = travel - offset` → single row travels `h-1` → `0`
- clocks ≥ `FLASH_MS` settle to the top row (the last scan frame), same
  clamp semantics (`local = clock_ms.min(LAST_PAINT_MS)`)
- degenerate guard: `width == 0 || height == 0` → empty vec (unchanged)
- a 1-row pane settles/paints its only row (the strip covers the whole pane
  for the window, like the old 2-column pane did)

### Inline tests in `flash.rs` (all in `#[cfg(test)] mod tests`, lines 87-199)

Pattern: `fn $NAME($$$ARGS) { $$$BODY }`

- `flash.rs:88` — `cells_are_deterministic_and_inside_the_rect()` — geometry-agnostic, keep
- `flash.rs:106` — `window_elapse_yields_the_settled_left_edge_strip()` — asserts x==0||x==1; must assert y == rect.y (top row) instead
- `flash.rs:133` — `strip_is_full_height_and_two_columns_wide()` — asserts 23 rows / 2 cols; must assert 1 row / full width
- `flash.rs:153` — `strip_sweeps_right_to_left()` — asserts left col 118→0; must assert top row h-1→0 (bottom-to-top)
- `flash.rs:184` — `narrow_panes_stay_visible_and_inside()` — 1-col/2-col panes; must cover 1-row/2-row panes instead

### Callers / integration points (no changes required)

Pattern: `pub fn $NAME($$$ARGS) -> $$$RET { $$$BODY }` in `types.rs`

- `types.rs:185` — `is_flash_active` — 350ms window semantics unchanged
- `types.rs:197` — `has_settled_flash` — paint gate unchanged
- `render.rs:137` — `paint_focus_flash` — calls `flash_cells(rect, clock)`
  and paints whatever it returns; geometry-agnostic
- `render.rs:123-124` — paint-then-advance ordering unchanged

### Existing test files that pin the old geometry (must be updated)

- `rust/fspec-tui/tests/mux006_focus_flash.rs` — asserts right-edge start,
  left-edge end, 2-col width, full-height span
- `rust/fspec-tui/tests/mux007_settled_final_frame.rs` — asserts
  left-edge 2-col strip settle geometry (`assert_left_edge_strip` helper)
