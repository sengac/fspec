# RPC-369 — Click-to-select a checkpoint or file row in the Checkpoints view

## Problem

In the Rust ratatui three-pane `CheckpointsView` (the **C**-key view:
checkpoints list ▸ files list ▸ diff), the user can change the selected checkpoint or file
only via the keyboard (Tab/arrows) or the mouse **wheel**. A left **click** on a
checkpoint-name row or a file row does nothing.

Mouse events already reach the view, but `handle_mouse` (in
`views/checkpoints/keys.rs`) matches only `MouseEventKind::ScrollUp` / `ScrollDown` and
drops everything else (`_ => CheckpointsEvent::Ignored`), so a `Down` (click) is discarded.

## Goal

- Left-click a **checkpoint-name row** (Checkpoints pane) → select that checkpoint and
  reload its changed-files list (`Action::LoadCheckpointFiles`).
- Left-click a **file row** (Files pane) → select that file and reload its diff
  (`Action::LoadCheckpointFileDiff`).
- A click focuses the pane it lands in.
- A click in the Diff pane only focuses the diff pane.
- While a restore/delete dialog is open, the dialog swallows the click (guard precedence).
- Click on empty space / outside all rects, or on the already-selected row → no change.

## Where the code lives

| Concern | File |
|---|---|
| View state, `Pane`, `pane_at`, `handle_event` | `codelet/fspec-tui/src/views/checkpoints/mod.rs` |
| `handle_key` + `handle_mouse` (the file to edit) | `codelet/fspec-tui/src/views/checkpoints/keys.rs` |
| Selection setters `move_checkpoint_selection` / `move_file_selection` | `codelet/fspec-tui/src/views/checkpoints/navigation.rs` |
| Pane rendering (caches `last_*_rect`) | `codelet/fspec-tui/src/views/checkpoints/render.rs` |
| Checkpoint-row label | `codelet/fspec-tui/src/views/checkpoints/checkpoint_row.rs` |
| Restore/delete dialog state | `checkpoints/dialog.rs`, `checkpoints/delete_dialog.rs` |
| Event routing into the view | `views/navigator_events.rs` (`handle_checkpoints_event`) |

### Relevant existing state (`CheckpointsView`)

```rust
pub struct CheckpointsView {
    checkpoints: Vec<CheckpointInfo>,
    selected_checkpoint: usize,
    checkpoint_scroll: usize,
    files: Vec<ChangedFile>,
    selected_file: usize,
    file_scroll: usize,
    diff_lines: Vec<String>,
    diff_scroll: usize,
    focused_pane: Pane,             // enum Pane { Checkpoints, Files, Diff }
    wheel: WheelVelocity,
    last_checkpoints_rect: Option<Rect>, // CONTENT rects cached at render
    last_files_rect: Option<Rect>,
    last_diff_rect: Option<Rect>,
    restore_dialog: Option<dialog::RestoreDialog>,
    delete_dialog: Option<delete_dialog::DeleteDialog>,
}
```

## Event flow (already wired — no plumbing changes)

```
App::handle_event
  -> Navigator::handle_event
    -> handle_checkpoints_event (navigator_events.rs)
      -> CheckpointsView::handle_event (Event::Mouse -> handle_mouse)
        -> handle_mouse(MouseEvent)   // keys.rs
```

`CheckpointsEvent::Emit(action)` returned from `handle_mouse` is relayed onto `action_tx`
by `navigator_events.rs`. **No App/Navigator change required.**

## Row → index mapping

`last_checkpoints_rect` and `last_files_rect` are **content** rects (header + underline
rows already excluded by `pane_header`). So the first visible row maps to the scroll
offset:

```
clicked_index = scroll + (ev.row - rect.y) as usize
```

with `scroll`/`rect` being `checkpoint_scroll`/`last_checkpoints_rect` for the Checkpoints
pane and `file_scroll`/`last_files_rect` for the Files pane. Clamp to the list length and
ignore clicks past the last populated row.

## Implementation sketch (`views/checkpoints/keys.rs`)

```rust
pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> CheckpointsEvent {
    // Dialog guard FIRST — restore/delete modal swallows all mouse input.
    if self.dialog().is_some() || self.delete_dialog().is_some() {
        return CheckpointsEvent::Consumed;
    }
    // NEW: left click selects the row under the cursor.
    if let MouseEventKind::Down(_) = ev.kind {
        return self.handle_click(ev.column, ev.row);
    }
    // ... existing wheel handling unchanged ...
}

fn handle_click(&mut self, col: u16, row: u16) -> CheckpointsEvent {
    let pane = match self.pane_at(col, row) {
        Some(p) => p,
        None => return CheckpointsEvent::Ignored,
    };
    self.set_focused_pane(pane);
    match pane {
        Pane::Diff => CheckpointsEvent::Consumed,
        Pane::Checkpoints => {
            let Some(rect) = self.last_checkpoints_rect else { return CheckpointsEvent::Consumed };
            let offset = row.saturating_sub(rect.y) as usize;
            if offset >= self.checkpoints.len().saturating_sub(self.checkpoint_scroll) {
                return CheckpointsEvent::Consumed;
            }
            let target = self.checkpoint_scroll + offset;
            self.move_checkpoint_selection(target as i32 - self.selected_checkpoint as i32)
        }
        Pane::Files => {
            let Some(rect) = self.last_files_rect else { return CheckpointsEvent::Consumed };
            let offset = row.saturating_sub(rect.y) as usize;
            if offset >= self.files.len().saturating_sub(self.file_scroll) {
                return CheckpointsEvent::Consumed;
            }
            let target = self.file_scroll + offset;
            self.move_file_selection(target as i32 - self.selected_file as i32)
        }
    }
}
```

`move_checkpoint_selection` / `move_file_selection` (navigation.rs) already clamp, call
`ensure_visible`, emit `Action::LoadCheckpointFiles` / `Action::LoadCheckpointFileDiff`,
and early-return `Consumed` (no Emit) when the clamped index equals the current selection —
so the "click already-selected row" no-op is free.

> `set_focused_pane` / direct `focused_pane` assignment: use whatever accessor the module
> already exposes (mirror `cycle_pane`). Keep `keys.rs` under 300 lines — if needed, put
> `handle_click` in `navigation.rs` alongside the selection setters.

## Acceptance criteria (Example Map → scenarios)

- Two checkpoints, first selected → click second → `selected_checkpoint == 1`, emits
  `LoadCheckpointFiles`.
- Files `a.txt`/`b.txt`, `a.txt` selected → click `b.txt` → `selected_file == 1`, emits
  `LoadCheckpointFileDiff(b.txt)`.
- Click inside Diff pane → focuses Diff, selections unchanged.
- Restore dialog open → click on checkpoint row swallowed, selection unchanged.
- Click already-selected row / empty space / outside rects → no change.

## Testing

Unit tests on `CheckpointsView` mirroring the existing wheel tests
(`views/checkpoints/tests.rs` / `navigation` tests): seed `checkpoints`, `files`, scroll
offsets and `last_*_rect`, then dispatch a synthetic
`MouseEvent { kind: Down(Left), column, row, .. }` and assert the selection index, the
`focused_pane`, and the emitted `Action`. Add a test that opens a restore dialog and
verifies the click is consumed with no selection change.

## Dependency

Depends on **RPC-368** — that card establishes the click-handling pattern in the simpler
two-pane `ChangedFilesView`; this card applies the same approach to the three-pane view.

## Out of scope

- Drag, double-click, right-click menus, scrollbar-gutter click handling.
- Restore/delete dialog mouse interactions beyond "swallow the click".
