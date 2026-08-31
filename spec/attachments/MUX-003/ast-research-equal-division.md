# AST Research — MUX-003 (equal-division pane splits)

Date: 2026-08-26
Tool: AstGrep (rust)

## Functions identified for modification

### `split_sizes` — rust/fspec-tui/src/views/multiplex/layout.rs:137
`fn split_sizes(available, panes, splits, orientation, first_override) -> Vec<u16>`

Current behavior (the bug):
- Every pane takes `splits[i]` percent (default 50 via `unwrap_or(50)`), clamped UP to
  `min_size(kind, orientation)` (board 64 / non-board 20 / vertical 10).
- With `splits = [50, 50]` for 3 panes: pane0=50%, pane1=50%, pane2=remainder (~1 col).
  This is why `/mux board agent agent` collapses the third pane.

Required change:
- Default (no explicit percent) → equal division: pane i gets `available / n`,
  last pane absorbs the remainder (`available - taken`).
- Remove the `min_size` clamp-up from the default path (R2).
- Keep the `first_override` (mouse drag) path: clamp to `[1, available - (n-1)]` only.
- An explicit percent entry is honored as-is (R4), no minimum clamp.

### `min_size` — rust/fspec-tui/src/views/multiplex/layout.rs:26
`fn min_size(kind, orientation) -> u16` — becomes dead once clamps are removed from
`split_sizes`. Delete (and the `MIN_BOARD_PANE_WIDTH` / `MIN_PANE_WIDTH` /
`MIN_PANE_HEIGHT` consts if no other callers — verified: only layout.rs + mod.rs re-export
+ tests/mux001.rs).

### `set_pane_list` — rust/fspec-tui/src/views/multiplex/mod.rs:237
`pub fn set_pane_list(&mut self, panes, split_percent: Option<u16>)`

Current: `splits = match split_percent { Some(p) => vec![p], _ => vec![50; n-1] }`.
Required: when no percent is given, store an equal-division marker — cleanest is
`splits = vec![]` (empty) meaning "equal division"; `split_sizes` falls back to
`available / n` for panes with no entry. `Some(p)` keeps `vec![p]`.

### `set_pane_count` — rust/fspec-tui/src/views/multiplex/mod.rs:224
Current: `splits = vec![50; count - 1]`.
Required: `splits = vec![]` (equal division).

### `calculate_pane_rects` / `calculate_pane_rects_with_override` — layout.rs:74/87
No signature change; the equal-division logic lives in `split_sizes`.

### `render_with_stores` — rust/fspec-tui/src/views/multiplex/render.rs:32
Already recomputes `pane_rects` from the live frame `area` every draw (lines 50-57) —
terminal resize re-division (R3) is already wired; no change needed beyond the new
`split_sizes` math. `recompute_rects` (rects.rs:20) is the event-time path and reuses
the same function.

## Persistence compatibility
`MuxConfig.splits` is persisted under `tui.mux` (rust/sessions/src/mux_config_persistence.rs).
Saved configs with `splits: [50, 50]` will load and render equal-ish (50/50/remainder) —
acceptable; `/mux save` after the fix stores the new shape. No schema migration needed
(the field is `Vec<u16>`, empty vec is valid).

## Test impact
- tests/mux001.rs: `clamping_never_produces_a_sub_minimum_pane` (710),
  `explicit_split_percent_is_respected_when_above_minimum` (739, uses `.max(MIN_BOARD_PANE_WIDTH)`),
  `tiny_terminal_degrades_gracefully` (831), `vertical_orientation_enforces_minimum_height` (848),
  `non_board_panes_use_the_20_col_floor` (864), `all_min_splits_leave_remainder_for_last_pane` (874),
  layout.rs inline tests `board_pane_clamps_to_minimum` (217) — all assert the removed
  minimum-clamp behavior and must be rewritten to expect equal division.
- New tests/mux003.rs for the 7 scenarios in
  spec/features/equal-division-pane-splits-no-minimums-with-live-resize.feature.
