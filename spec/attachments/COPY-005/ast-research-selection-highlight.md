# AST Research — COPY-005 Render selection highlight overlay

## Goal
Confirm the paint pipeline (`scrollback_paint.rs`), the buffer cell styling API, and the REVERSED-vs-DIM coexistence before adding `paint_selection_highlight`.

## Paint pipeline (`scrollback_paint.rs`)
- `pub(super) fn paint_chunk_rows(area, buf, chunks, content_width, skip_rows) -> usize` (line 60) paints the windowed chunk text first.
- `pub(super) fn paint_scrollbar(area, buf, vh, total_rows, state)` (line 23) paints the gutter column using `Modifier::DIM` on the rightmost column (`x = area.x + area.width - 1`).
- Arrow bars come from `scrollback_arrows.rs` (`paint_selection_arrow_bars`, re-exported line 105).
- These are all invoked from `ScrollbackList::render_count_visited` (scrollback.rs) in order: paint_chunk_rows → paint_selection_overlay (arrow bars) → paint_scrollbar. COPY-005 inserts `paint_selection_highlight` AFTER chunk rows + arrow bars, BEFORE/independent of the scrollbar (it must never touch the gutter column).

## Cell styling API
Existing painters use `let cell = &mut buf[(x, y)]; cell.set_style(style);` (paint_scrollbar lines 49-51) and `cell.set_symbol(glyph)`. COPY-005 uses `buf[(x, y)].set_style(Style::default().add_modifier(Modifier::REVERSED))` — this MERGES the REVERSED modifier while PRESERVING the underlying glyph (set_style does not clear the symbol). This coexists with the DIM arrow bars (different modifier).

## Signature & content-width clamp
- `pub(super) fn paint_selection_highlight(area: Rect, buf: &mut Buffer, spans_in_viewport: &[RowSpan], content_width: u16)`.
- Input RowSpans are ALREADY viewport-space + offset-mapped + content-width-clamped (COPY-006 derives them the same way it derives the copy region → single source of truth with COPY-004).
- For each span: `y = area.y + span.row`; for `col in span.start_col .. span.end_col.min(content_width)`: `x = area.x + col`; guard `x < area.x + area.width` and `y < area.y + area.height`; `buf[(x,y)].set_style(reversed)`. Rows/cols outside the viewport are simply skipped (rows scrolled off → not in spans_in_viewport, or clamped).

## Testing pattern
Mirror the existing `scrollback_paint.rs` `#[cfg(test)] mod tests`: build `Buffer::empty(area)`, call the paint fn with hand-built RowSpans, then assert `buf[(x,y)].modifier.contains(Modifier::REVERSED)` on target cells and `!contains(REVERSED)` on gutter/out-of-region cells. `RowSpan` is `crate::mouse::selection::RowSpan` (COPY-002, done).

## Module placement
Add `paint_selection_highlight` INTO scrollback_paint.rs (currently 262 lines; adding ~30 lines + tests keeps it near but the tests add more — WATCH the 300-LoC ceiling; if it would exceed, the fn + tests can go in a `#[path]` sibling `scrollback_highlight.rs`). Prefer adding the fn to scrollback_paint.rs and the render-tests there; if over 300, split.
