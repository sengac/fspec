# AST research — MUX-007 (settled final frame persists on the active pane)

Research notes for the change that keeps the MUX-006 focus-flash FINAL frame
(the full-height 2-column dark-purple strip parked at the pane's LEFT edge)
painted on the focused pane after the 350ms scan window elapses. Equivalent
AST/ripgrep searches were run directly (the `fspec research --tool=ast`
command is not yet ported to the Rust binary).

## 1. MUX-006 flash architecture (target of the change)

- `rust/fspec-tui/src/views/multiplex/flash.rs` — PURE pattern math:
  `flash_cells(rect: Rect, clock_ms: u64) -> Vec<(u16, u16)>`.
  - `FLASH_MS = 350`, `FLASH_FRAME_MS = 16`,
    `LAST_PAINT_MS = 336` (350/16*16).
  - Returns `Vec::new()` when `clock_ms >= FLASH_MS` (line 44) — THIS is
    why the purple disappears today.
  - For `clock < FLASH_MS`: the strip's left column travels from `w-2`
    (right edge) to `0` (left edge); at clock 336 (last paintable frame)
    the strip sits exactly at columns [0, 1] — the LEFT edge.
  - So `flash_cells(rect, FLASH_MS)` evaluated AFTER the clamp fix would
    yield the same cells as `flash_cells(rect, LAST_PAINT_MS)`: the
    settled left-edge strip.
- `rust/fspec-tui/src/views/multiplex/types.rs` — flash STATE on
  `MultiplexLayout` (live-only, R8):
  - `flash_pane: Option<MuxFocus>`, `flash_clock_ms: u64`.
  - `rearm_flash(pane)` — arms (clock 0) iff mux enabled + valid pane.
  - `bump_focus(next)` — moves focus and re-arms iff focus CHANGED
    (window.rs focus_prev/focus_next, mod.rs set_focus, window clamp,
    note_session_created all route through this).
  - `advance_flash_clock()` (lines 216-227): +16ms per rendered mux
    frame; when the clock reaches ≥ 350 it CLEARS `flash_pane = None`
    and resets the clock. After this, `is_flash_active()` is false.
  - `is_flash_active()` (lines 178-182): `enabled && flash_pane.is_some()
    && clock < FLASH_MS` — feeds BOTH the paint pass AND the tick gate.
- `rust/fspec-tui/src/views/multiplex/render.rs` — the paint pass:
  - `paint_focus_flash(layout, &rects, buf)` (lines 128-148): returns
    early when `!layout.is_flash_active()` (line 129); otherwise paints
    `flash_cells(rect, flash_clock_ms())` backgrounds over the armed
    pane's rect. Called AFTER pane content, BEFORE dividers/footer.
  - `layout.advance_flash_clock()` (line 119) runs after the paint.
- `rust/fspec-tui/src/views/navigator.rs:127-128` —
  `Navigator::is_mux_flash_active()` = `active_view == Mux &&
  mux.is_flash_active()` → 5th `tick_should_draw` operand
  (`app/mod.rs:101-108`).
- `rust/fspec-tui/src/app/events.rs:307-316` — the run loop evaluates the
  gate on the 16ms tick; while the flash is in flight it keeps redrawing
  even when idle.

## 2. Focus-mutation sites (EXHAUSTIVE, verified by grep — unchanged)

`bump_focus` (re-arm on change): set_focus, focus_prev, focus_next,
note_session_created, recompute_effective_panes window clamp.
`rearm_flash` (unconditional re-arm): enable_default, enable_with_config,
set_pane_count, set_pane_list. `disarm_flash`: disable (mux exit).

## 3. What changes for MUX-007 (design)

1. `flash.rs` — settle boundary: keep the pure fn's signature
   `(rect, clock_ms)` but treat `clock >= FLASH_MS` as "settled" instead
   of "empty": the returned cells are the LEFT-EDGE strip
   (clock == LAST_PAINT_MS geometry). Degenerate rects (w==0/h==0) still
   yield nothing. This keeps the fn total and deterministic for any
   clock, and lets the painter reuse it verbatim.
2. `types.rs` — `advance_flash_clock()` no longer CLEARS `flash_pane`
   when the window elapses (it still clamps the clock so the geometry is
   stable); `is_flash_active()` keeps its exact 350ms semantics (used by
   the tick gate — R4: settled strip must NOT keep the gate open).
   New accessor `has_settled_flash()` (or equivalent): `enabled &&
   flash_pane.is_some()` — "the focused pane owns the accent" — true
   both during the scan and after it.
3. `render.rs` — `paint_focus_flash` paints when `has_settled_flash()`
   (instead of `is_flash_active()`); cells come from the (now clamped)
   `flash_cells(rect, clock)` so mid-scan frames paint the moving strip
   and post-scan frames paint the parked left-edge strip. Focus change:
   `flash_pane` follows `focus` via `bump_focus`, so the old pane's strip
   vanishes automatically (R2) — no per-pane state needed.
4. `disable()` keeps `disarm_flash()` — R7: mux off ⇒ zero purple cells.

## 4. Tests already in place / affected

- `rust/fspec-tui/tests/mux006_focus_flash.rs`:
  - `focusing_a_pane_scans_a_purple_strip_from_right_edge_to_left` — the
    last @step now asserts the SETTLED strip on the focused pane only
    (previously asserted no purple at all).
  - `the_flash_keeps_the_render_tick_redrawing_while_idle_and_stops_after_the_window`
    — asserts gate returns to idle AND the next frame still paints the
    settled strip (R4).
- `flash.rs` unit tests: `window_elapse_yields_no_cells` must flip to
  "window elapse yields the left-edge strip" (settled).
- New integration file `tests/mux007_settled_final_frame.rs` covering the
  4 new Gherkin scenarios in
  `spec/features/mux-focus-flash-settled-final-frame-persists-on-the-active-pane.feature`
  (settled strip persists; focus change moves the strip; background-only
  + mux-off clean; gate idle but strip repainted).
