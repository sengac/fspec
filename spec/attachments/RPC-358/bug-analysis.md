# RPC-358 — Arrow keys do not scroll the diff pane when it is focused

## Symptom (user-reported)
> "Selecting the diff view (arrow-right) does not allow scrolling via arrow keys."

After moving focus to the diff pane (Tab, or Left/Right), pressing Up/Down still moves the
**file-list selection** instead of scrolling the diff content. The only ways to scroll the
diff today are PgUp/PgDn or the mouse wheel.

## Root cause
File: `codelet/fspec-tui/src/views/changed_files/mod.rs`

In `handle_key` (≈ lines 166‑189), `KeyCode::Up`/`KeyCode::Down` are hard-wired to
`move_selection(±1)` **regardless of `self.focused_pane`**. There is no branch on the focused
pane for the arrow keys. A code comment (≈ lines 233‑236) documents this as intentional
("the file list owns Up/Down"), but it diverges from the reference TS behavior and from user
expectation.

`scroll_focused` (≈ lines 260‑268) already routes to `apply_diff_scroll` **only when**
`focused_pane == Pane::Diff` — but it is currently reached only from PgUp/PgDn.

## Reference behavior (original TypeScript TUI)
- File list `VirtualList` uses `selectionMode: 'item'` → Up/Down move the selection.
- Diff `VirtualList` uses `selectionMode: 'scroll'` and `isFocused={focusedPane==='diff'}`
  → when the diff pane is focused, Up/Down adjust `scrollOffset` by ±1 line
  (`handleScrollNavigation`, `VirtualList.tsx`). PgUp/PgDn/Home/End also supported.
- `ChangedFilesViewer` returns `false` for up/down so they fall through to the focused list.

So the correct behavior is **pane-aware arrow keys**:
- Focus on Files pane → Up/Down move the file selection (and reload the diff).
- Focus on Diff pane → Up/Down scroll the diff content one line at a time.

## Required fix
Make `handle_key` branch on `self.focused_pane` for `KeyCode::Up`/`KeyCode::Down`:
- `Pane::Files` → `move_selection(±1)` (current behavior, emits `LoadFileDiff`).
- `Pane::Diff`  → scroll the diff by ±1 line (reuse `apply_diff_scroll` / `scroll_focused`
  with a one-line step), clamped to the existing bounds (0 .. diff_len − pane_height).

Update the footer hint in `render.rs` if it implies Up/Down only navigate.

## Acceptance criteria
- With focus on the file-list pane, Up/Down move the selection and reload the diff
  (no regression to RPC-357 / existing scenario 5).
- With focus on the diff pane, Down increases `diff_scroll` by one line and Up decreases it,
  clamped so it never goes below 0 or past `diff_len − pane_height`.
- With focus on the diff pane, Up/Down do **not** change the file selection index.
- Tab/Left/Right still toggle focus between panes.

## Key files
- `codelet/fspec-tui/src/views/changed_files/mod.rs` (`handle_key`, `scroll_focused`,
  `apply_diff_scroll`, `toggle_pane`)
- `codelet/fspec-tui/src/views/changed_files/render.rs` (footer hint)
- `codelet/fspec-tui/src/views/changed_files/tests.rs` (add diff-focused arrow-scroll tests)
- Feature: `spec/features/rust-changed-files-view.feature`
