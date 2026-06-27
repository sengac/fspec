# RPC-357 — Mouse-wheel selection does not reload the diff pane

## Symptom (user-reported)
> "Mouse scrolling does NOT update the diff view when the selected file changes (only arrow keys do)."

When the user scrolls the mouse wheel over the **file-list pane**, the selection highlight
(`>` cursor) moves to a different file, but the **diff pane keeps showing the previously
selected file's diff**. Pressing an arrow key afterwards immediately corrects it, proving the
data path works — only the mouse path is broken.

## Root cause
File: `codelet/fspec-tui/src/views/changed_files/mod.rs`

`move_selection(delta)` (≈ lines 237‑258) does the right thing: it clamps the new
`selected_index`, resets `diff_scroll`, calls `ensure_visible`, and **returns**
`ChangedFilesEvent::Emit(Action::LoadFileDiff(path))` for the newly selected file.

`handle_mouse` (≈ lines 191‑208) handles wheel events over the Files pane like this:

```rust
Pane::Files => {
    let _ = self.move_selection(step);   // <-- return value DISCARDED
}
...
ChangedFilesEvent::Consumed              // <-- always returns Consumed, never Emit
```

The `let _ =` throws away the `Emit(LoadFileDiff)` that `move_selection` produced, and the
function unconditionally returns `Consumed`. Therefore no `Action::LoadFileDiff` ever reaches
the action bus (`navigator_events.rs::handle_changed_files_event` →
`app/dispatch_changed_files.rs::spawn_file_diff` → `backend.file_diff()`), so the diff is
never reloaded.

By contrast `handle_key` (≈ lines 166‑189) **returns** the event from `move_selection`
directly for `KeyCode::Up`/`KeyCode::Down`, which is why arrow keys work.

## Reference behavior (original TypeScript TUI)
In `src/tui/components/VirtualList.tsx`, a mouse-wheel event in `selectionMode: 'item'`
calls `navigateTo(selectedIndex + delta)`, which updates `selectedIndex`; the `onFocus`
effect then fires `onFileSelect → setSelectedFileIndex`, and `FileDiffViewer`'s
`useEffect([selectedFileIndex,...])` reloads the diff. **Wheel selection updates the diff.**

## Required fix
When a mouse wheel event moves the selection in the **Files** pane, `handle_mouse` must
propagate the `ChangedFilesEvent::Emit(Action::LoadFileDiff(path))` returned by
`move_selection` instead of discarding it — exactly as `handle_key` does.

Scope note: keep wheel over the **Diff** pane behaviour unchanged (it scrolls the diff via
`apply_diff_scroll`). Mouse-**click** selection is out of scope (the reference TS view does
not support click-to-select either).

## Acceptance criteria
- A mouse-wheel `ScrollDown`/`ScrollUp` over the file-list pane that changes the selected
  index emits `Action::LoadFileDiff` for the newly selected file's path.
- The wheel-driven selection still updates the highlight and keeps the selected row visible.
- Wheel over the diff pane continues to scroll the diff (no regression).

## Key files
- `codelet/fspec-tui/src/views/changed_files/mod.rs` (`handle_mouse`, `move_selection`)
- `codelet/fspec-tui/src/views/changed_files/tests.rs` (add wheel-reload test)
- Feature: `spec/features/rust-changed-files-view.feature` (scenario 5 covers arrow reload)
