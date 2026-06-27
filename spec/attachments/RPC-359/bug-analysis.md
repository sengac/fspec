# RPC-359 — Missing scrollbars for the file-list and diff panes

## Symptom (user-reported)
> "No scroll bar for either the files list or the diff view."

Neither pane renders any visual scroll indicator, so the user cannot tell how much content is
off-screen or where they are within it.

## Root cause
File: `codelet/fspec-tui/src/views/changed_files/render.rs`

- `render_files_pane` (≈ lines 101‑121) renders a header plus a plain `Paragraph` of the
  visible rows via `.skip(file_scroll).take(visible)`. No scrollbar.
- `render_diff_pane` (≈ lines 123‑140) renders a header plus a plain `Paragraph` via
  `.skip(diff_scroll).take(visible)`. No scrollbar.

Grep confirms the `changed_files/` module uses **no** `Scrollbar` / `ScrollbarState` widget.
The only `Scrollbar` usages in the crate are in unrelated views
(`views/agent/scrollback_paint.rs`, `views/model_selector/rows_render.rs`), which can serve
as in-repo reference implementations.

## Reference behavior (original TypeScript TUI)
`src/tui/components/VirtualList.tsx` renders a `Scrollbar` component on the focused pane:
- File list: `showScrollbar={focusedPane === 'files'}`
- Diff:      `showScrollbar={focusedPane === 'diff'}`

The scrollbar draws a vertical track with a `■` thumb / `│` track **only when the content
overflows** the visible height (`itemCount > visibleHeight`). Thumb height ∝
`visibleHeight / itemCount`; thumb position ∝ `scrollOffset / itemCount`. It is hidden on the
unfocused pane and when content fits.

## Required fix
Render a vertical scrollbar for each pane using ratatui's `Scrollbar` /
`ScrollbarState` widget (matching the existing usage in `scrollback_paint.rs` /
`rows_render.rs`):
- Files pane: scrollbar driven by `file_scroll` over `files.len()` with viewport height =
  visible row count.
- Diff pane: scrollbar driven by `diff_scroll` over `diff_lines.len()` with viewport height =
  visible diff line count.
- Show the scrollbar only when the content overflows the pane height (content > visible).
  Follow the TS reference of emphasising the focused pane's scrollbar; matching that exactly
  (scrollbar only on focused pane) is acceptable, but rendering on both when overflowing is
  also acceptable — decide during Example Mapping and encode the chosen rule in scenarios.

## Acceptance criteria
- When the file list has more rows than the visible height, a vertical scrollbar is rendered
  in the file-list pane whose thumb position reflects `file_scroll`.
- When the diff has more lines than the visible height, a vertical scrollbar is rendered in
  the diff pane whose thumb position reflects `diff_scroll`.
- When content fits within the pane (no overflow), no scrollbar is rendered for that pane.
- Scrollbar rendering must not change the existing pane layout split (Files 40% / Diff 60%)
  beyond reserving the scrollbar column.

## Key files
- `codelet/fspec-tui/src/views/changed_files/render.rs` (`render_files_pane`,
  `render_diff_pane`)
- Reference: `codelet/fspec-tui/src/views/agent/scrollback_paint.rs`,
  `codelet/fspec-tui/src/views/model_selector/rows_render.rs`
- `codelet/fspec-tui/src/views/changed_files/tests.rs` (assert scrollbar presence/absence)
- Feature: `spec/features/rust-changed-files-view.feature`
