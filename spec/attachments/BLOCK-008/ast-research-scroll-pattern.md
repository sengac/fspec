# AST research — BlocklistView scrolling (BLOCK-008)

Goal: adopt the established Rust viewport-scroll pattern for
`codelet/fspec-tui/src/views/blocklist/mod.rs`.

## Shared primitive (reuse, do NOT reimplement)

`codelet/fspec-tui/src/components/scroll_viewport.rs:46`

```
pub fn ensure_visible(scroll_offset: &mut usize, selected: usize, visible_rows: usize, total: usize)
```

Reconciles `scroll_offset` so `selected` stays in `[scroll_offset, scroll_offset+visible_rows)`,
clamps to `total - visible_rows`; `total==0 || visible_rows==0` → 0.

## Reference `adjust_scroll` (pattern to mirror)

`codelet/fspec-tui/src/views/model_selector/state.rs:107`

```
pub(crate) fn adjust_scroll(&mut self) {
    crate::components::scroll_viewport::ensure_visible(
        &mut self.scroll_offset, self.selected_index, self.visible_rows, self.rows.len());
    // (+ view-specific top-reveal override)
}
```

Render recomputes `visible_rows` from real body height and calls `adjust_scroll()`
again defensively: `model_selector/render.rs:127-131`.

## Shared scrollbar helpers (reuse, do NOT reimplement)

- `codelet/fspec-tui/src/components/list_scrollbar.rs:23`
  `pub fn render_list_scrollbar(area, buf, scroll_offset, visible, total)` — paints
  the proportional `■`/`│` thumb in a 1-col area.
- `codelet/fspec-tui/src/views/diff_common/mod.rs:28`
  `pub fn render_pane_scrollbar(content, buf, list_width, scroll, visible, total)` —
  wraps the above to paint in the reserved gutter right of a pane. Used by
  `changed_files/render.rs::render_files_pane` (the closest analog).

## changed_files windowing analog

`changed_files/render.rs:98-129` (`render_files_pane`):
- `visible = content.height as usize`
- `overflow = files.len() > visible`
- `list_width = content.width - (overflow ? 1 : 0)`
- paint `files.iter().enumerate().skip(scroll).take(visible)`
- `if overflow { render_pane_scrollbar(content, buf, list_width, scroll, visible, files.len()) }`

## Current blocklist state (to change)

`codelet/fspec-tui/src/views/blocklist/mod.rs`:
- struct `BlocklistView { rules: Vec<BlocklistRuleInfo>, selected_index: usize }` — NO
  scroll_offset / visible_rows.
- `render_left_pane` iterates ALL rules into one `Paragraph` (no windowing, no scrollbar).
- `handle_key` Down/Up/j/k move `selected_index` only (no `adjust_scroll`).
- `set_rules` clamps `selected_index` only.

## Plan

1. Add `scroll_offset: usize`, `visible_rows: usize` to the struct.
2. Add `adjust_scroll()` delegating to `ensure_visible`.
3. Each nav arm: clamp-move then `adjust_scroll()`.
4. `set_rules`: reset `scroll_offset` (and clamp selection), then `adjust_scroll()`.
5. `render_left_pane`: window slice + `render_pane_scrollbar` on overflow; compute
   `visible_rows` from the pane height and call `adjust_scroll()` defensively; add a
   `Showing X-Y of N` indicator.
6. Split into `blocklist/{mod,render,scroll,tests}.rs` if >300 lines.
