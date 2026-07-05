# BLOCK-008 — BlocklistView viewport scrolling

## Problem

`codelet/fspec-tui/src/views/blocklist/mod.rs` renders **every** rule into a
single `Paragraph` (`render_left_pane` iterates over all `self.rules` and pushes
2 lines per rule). There is:

- **No `scroll_offset`** field.
- **No `visible_rows`** field.
- **No windowing** — the `Paragraph` is handed the full list; ratatui simply
  clips whatever does not fit the pane height.
- **No scroll indicator** (`Showing X-Y of N`).
- **No overflow scrollbar gutter.**

Consequence: when the number of rules exceeds the pane height, the user can move
`selected_index` past the bottom of the visible window but the list never
scrolls — off-screen rules are unreachable/invisible. This diverges from the
TypeScript reference `src/tui/components/BlocklistListView.tsx`, which computes
`visibleHeight = max(1, terminalHeight - 10)`, keeps a `scrollOffset` that
auto-scrolls to keep the selection visible, slices
`rules.slice(scrollOffset, scrollOffset + visibleHeight)`, and paints a
`Showing X-Y of N` indicator.

## Established Rust pattern to adopt

The shared primitive lives at
`codelet/fspec-tui/src/components/scroll_viewport.rs`:

```rust
pub fn ensure_visible(scroll_offset: &mut usize, selected: usize, visible_rows: usize, total: usize)
```

It reconciles `scroll_offset` so `selected` stays inside the half-open window
`[scroll_offset, scroll_offset + visible_rows)`, clamping to
`max_offset = total - visible_rows`. `total == 0 || visible_rows == 0` resets to 0.

### Reference implementations

- **`model_selector`** (`state.rs::adjust_scroll`, `render.rs:127-131`,
  `navigation.rs`): stores `scroll_offset` + `visible_rows`; every nav method
  clamp-moves then calls `adjust_scroll()`; render recomputes `visible_rows`
  from the real body height and calls `adjust_scroll()` **again** defensively,
  then windows the row slice; draws a scrollbar column when
  `total > visible_rows`.
- **`changed_files`** (`mod.rs`, `render.rs::render_files_pane`): uses
  `ensure_visible`; `render_files_pane` computes `visible = content.height`,
  `overflow = files.len() > visible`, reserves a 1-col scrollbar gutter via
  `render_pane_scrollbar`, and paints `files.iter().skip(scroll).take(visible)`.

## Required changes (implementation guidance)

1. Add `scroll_offset: usize` (and, if needed for nav-time reconciliation, a
   `visible_rows: usize`) to `BlocklistView`.
2. Each navigation arm in `handle_key` (`Down`/`Up`/`j`/`k`) must clamp-move
   `selected_index` **then** call an `adjust_scroll()` that delegates to
   `scroll_viewport::ensure_visible`.
3. `set_rules` must reset/clamp `scroll_offset` alongside `selected_index`.
4. `render_left_pane` must window the rows: paint only
   `rules[scroll_offset .. (scroll_offset + visible_rows).min(len)]`, computing
   `visible_rows` from the real left-pane height. It should reserve a 1-column
   scrollbar gutter and draw the overflow scrollbar when `rules.len() > visible_rows`
   (reuse the shared scrollbar helper used by `changed_files` /
   `list_scrollbar.rs` where practical — DRY).
5. Render must call `adjust_scroll()` defensively once the true body height is
   known (covers resize + the seed-height case), mirroring
   `model_selector/render.rs:127-131`.

## Constraints

- File stays under 300 lines — split into sibling modules
  (`blocklist/mod.rs` + `blocklist/render.rs` + `blocklist/scroll.rs` /
  `blocklist/tests.rs`) if the addition pushes it over, mirroring how
  `changed_files/` and `model_selector/` are organised.
- Clippy clean, no `unwrap`/`expect`/`todo` in non-test code.
- DRY: reuse `scroll_viewport::ensure_visible` and the shared scrollbar helper
  rather than hand-rolling.
- Do NOT regress existing RPC-056 behaviour (category tags, session-disabled
  glyphs, toggle emit, empty-state, Esc close).

## Test expectations

- Navigating `Down` past the visible window advances `scroll_offset` so the
  selection stays visible (assert via `ensure_visible` semantics on a seeded
  list taller than the viewport).
- Navigating back `Up` above the window scrolls the offset back.
- `scroll_offset` clamps at `total - visible_rows` and never runs past the end.
- A render into a small buffer with more rules than rows shows the last
  selectable row's id after paging to the end (buffer-text assertion) and shows
  a `Showing X-Y of N` indicator.
- `set_rules` with a shorter list resets `scroll_offset` into range.
- Every test carries `// @step` comments matching the generated Gherkin.
