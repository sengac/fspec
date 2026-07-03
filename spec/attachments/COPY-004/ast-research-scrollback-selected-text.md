# AST Research — COPY-004 Reconstruct selected text from scrollback

## Goal
Confirm ScrollbackList internals (chunks/lines/scroll offset), the reserve_gutter/content_width math, and the 300-LoC split convention before adding `selected_text`.

## ScrollbackList shape (`scrollback.rs`)
- `pub struct ScrollbackList { chunks: Vec<RenderedChunk>, scroll_state: ScrollState, viewport_height: u16, viewport_width: u16, ... }` (line 44).
- `ScrollState { pub offset: usize, pub stick_to_bottom: bool }` (line 26). `offset` is in VISUAL ROWS.
- `total_visual_rows(&self)` = `self.chunks.iter().map(|c| c.lines.len()).sum()` (line 256) — the exact iteration COPY-004 mirrors to map a visual row index to a chunk line.
- `RenderedChunk { seq: u64, lines: Vec<Line<'static>>, source }` (rendered_chunk.rs:97). A "visual row" is one `Line`.

## Gutter / content_width math (`scrollback.rs` render_count_visited)
- `reserve_gutter = self.total_visual_rows() > vh && area.width >= 4` (line 224).
- `content_width = if reserve_gutter { area.width - 2 } else { area.width }` (lines 225-231; reserve = 2 cols). This is the value COPY-006 passes to `Selection::spans` so end cols are pre-clamped; COPY-004 ALSO clamps to each row's real char length (double-guard against the TS │/■ bug).

## 300-LoC split convention
- `scrollback.rs` is 298 lines (near the ceiling). `scrollback_select.rs` (RPC-381) was extracted precisely to keep scrollback.rs under 300. COPY-004 follows suit: a NEW sibling module `scrollback_copy.rs` with an `impl ScrollbackList { pub fn selected_text(&self, region: &[RowSpan]) -> String }` (attached via `#[path=...] mod copy;` from scrollback.rs, mirroring how `mod select;` is wired at scrollback.rs:294). It needs read access to `chunks` + `scroll_state.offset` — either via `pub(super)`/existing accessors or by placing the impl in a `#[path]` submodule that shares the private module scope.

## Row flattening plan
- `visual_row_index = scroll_state.offset + viewport_row`.
- Walk chunks in order summing `lines.len()` to locate the Line for a visual row.
- Build plain string by concatenating `span.content` across `line.spans`.
- char-slice `[start_col .. min(end_col, char_len)]` via `chars().skip(start).take(end-start)` (unicode-safe; no byte split → emoji intact).
- Join rows with `\n`. Empty region → empty String.

## Signature
`fn selected_text(&self, region: &[RowSpan]) -> String` taking `RowSpan`s from `crate::mouse::selection` (COPY-002, done). Reuses that `RowSpan` type. Pure, no I/O. Tests build a ScrollbackList with known chunks (the existing scrollback_tests helper builds `RenderedChunk { lines: vec![Line::from(Span::raw(body))], .. }`), set offset, call selected_text, assert exact String.
