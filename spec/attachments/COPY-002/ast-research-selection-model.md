# AST Research — COPY-002 Text selection region model

## Goal
Confirm existing conventions (half-open rects, Cell-style row/col types) before adding the pure `selection.rs` geometry module.

## Half-open convention already in the codebase

`codelet/fspec-tui/src/mouse/hit_test.rs` `rect_contains` uses half-open right/bottom edges (`x < rect.x + rect.width`). RowSpan `start_col..end_col` will be half-open to match: a span cols 2..6 covers 2,3,4,5. Middle rows are `0..row_width`.

## No existing Cell / Selection / RowSpan type in the mouse module

AstGrep for structs in `codelet/fspec-tui/src/mouse/` finds only `MouseTrackingToggle` (toggle.rs) and the free `rect_contains` (hit_test.rs). No `Cell`/`Selection`/`RowSpan` — safe to introduce them in a new `selection.rs` with no collision. (Note: ratatui also has a `Cell` type but this pure module does NOT import ratatui, so no name clash — it lives in `crate::mouse::selection::Cell`.)

## Module plan
- New pure module `codelet/fspec-tui/src/mouse/selection.rs`, exported from `mouse/mod.rs`.
- `pub struct Cell { pub row: u16, pub col: u16 }`
- `pub struct Selection { pub anchor: Cell, pub cursor: Cell }`
- `pub struct RowSpan { pub row: u16, pub start_col: u16, pub end_col: u16 }` (end exclusive)
- `impl Selection { pub fn spans(&self, row_width: u16) -> Vec<RowSpan> }`
- Normalization: order (row,col) lexicographically; anchor==cursor → empty vec. Single row → one span. Multi row → first row start_col..row_width, full middle rows 0..row_width, last row 0..end_col.
- No crossterm / ratatui / io imports. Pure. In-module unit tests assert exact Vec<RowSpan>.
