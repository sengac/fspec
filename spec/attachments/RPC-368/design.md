# RPC-368 — Click-to-select a file row in the Changed Files view

## Problem

In the Rust ratatui `ChangedFilesView` (the **F**-key dual-pane view: file list ▸ diff),
the user can change the selected file only via the keyboard (Up/Down/PgUp/PgDn) or the
mouse **wheel**. A left **click** on a file row does nothing.

Mouse events already reach the view, but `handle_mouse` matches only
`MouseEventKind::ScrollUp` / `ScrollDown` and explicitly drops everything else
(`_ => ChangedFilesEvent::Ignored`), so `MouseEventKind::Down(_)` (a click) is discarded.

## Goal

A left-click on a file row in the file-list pane:
1. selects that file (`selected_index` ← clicked row),
2. focuses the Files pane,
3. scrolls the row into view if needed, and
4. reloads the diff pane for the clicked file (`Action::LoadFileDiff`).

A click in the diff pane only focuses the diff pane. Clicks outside both panes, or on
empty space below the last file, change nothing.

## Where the code lives

| Concern | File |
|---|---|
| View state + `handle_event` + `handle_mouse` + `pane_at` + `move_selection` | `codelet/fspec-tui/src/views/changed_files/mod.rs` |
| Pane rendering (caches `last_files_rect` / `last_diff_rect`) | `codelet/fspec-tui/src/views/changed_files/render.rs` |
| File-row formatting | `codelet/fspec-tui/src/views/diff_common/row.rs` (`file_row`) |
| Event routing into the view | `codelet/fspec-tui/src/views/navigator_events.rs` (`handle_changed_files_event`) |
| `ChangedFile` wire type | `codelet/rpc-types/src/lib.rs` |

### Relevant existing state (`ChangedFilesView`)

```rust
pub struct ChangedFilesView {
    files: Vec<ChangedFile>,
    selected_index: usize,        // file-line selection
    focused_pane: Pane,           // enum Pane { Files, Diff }
    diff_lines: Vec<String>,
    diff_path: Option<String>,
    file_scroll: usize,           // scroll offset of the file-list pane
    diff_scroll: usize,
    wheel: WheelVelocity,
    last_files_rect: Option<Rect>, // CONTENT rect cached at render time
    last_diff_rect: Option<Rect>,
}
```

## Event flow (already wired — no plumbing changes)

```
App::handle_event (events.rs:82)
  -> Navigator::handle_event
    -> handle_changed_files_event (navigator_events.rs)
      -> ChangedFilesView::handle_event  (matches Event::Mouse -> handle_mouse)
        -> handle_mouse(MouseEvent)
```

Returning `ChangedFilesEvent::Emit(Action::LoadFileDiff(path))` from `handle_mouse` is
already relayed onto `action_tx` by `navigator_events.rs`. **No App or Navigator change
is required.**

## Row → index mapping

`last_files_rect` is the **content** rect returned by `pane_header` — the two header rows
(heading + underline) are already excluded. Therefore the first *visible* row maps to file
index `file_scroll`:

```
clicked_index = file_scroll + (ev.row - rect.y) as usize
```

Guards:
- only when `pane_at(ev.column, ev.row) == Some(Pane::Files)` (bounds already checked),
- clamp `clicked_index` to `files.len() - 1`,
- ignore the click when `(ev.row - rect.y) as usize >= files.len().saturating_sub(file_scroll)`
  (the click landed on empty space below the last file).

## Implementation sketch

Add a `Down` arm to `handle_mouse` **before** the wheel match:

```rust
fn handle_mouse(&mut self, ev: MouseEvent) -> ChangedFilesEvent {
    if let MouseEventKind::Down(_) = ev.kind {
        return self.handle_click(ev.column, ev.row);
    }
    // ... existing wheel handling unchanged ...
}

fn handle_click(&mut self, col: u16, row: u16) -> ChangedFilesEvent {
    match self.pane_at(col, row) {
        Some(Pane::Diff) => {
            self.focused_pane = Pane::Diff;
            ChangedFilesEvent::Consumed
        }
        Some(Pane::Files) => {
            self.focused_pane = Pane::Files;
            let rect = match self.last_files_rect {
                Some(r) => r,
                None => return ChangedFilesEvent::Consumed,
            };
            let offset = row.saturating_sub(rect.y) as usize;
            if offset >= self.files.len().saturating_sub(self.file_scroll) {
                return ChangedFilesEvent::Consumed; // empty space
            }
            let target = self.file_scroll + offset; // < files.len()
            self.move_selection(target as i32 - self.selected_index as i32)
        }
        None => ChangedFilesEvent::Ignored,
    }
}
```

`move_selection(delta)` already clamps, resets `diff_scroll`, calls `ensure_visible`, and
emits `Action::LoadFileDiff` (or `Consumed` when the index is unchanged), so the no-op
behaviour for "click the already-selected row" comes for free.

> File-size watch: `mod.rs` is near the 300-line ceiling. If adding `handle_click` pushes
> it over, extract the click helper (or the whole mouse block) into a sibling file in the
> `changed_files/` module, mirroring how `checkpoints/keys.rs` was split out.

## Acceptance criteria (Example Map → scenarios)

- Click on `b.txt` (currently `a.txt` selected) → `selected_index == 1`, emits
  `LoadFileDiff(b.txt)`.
- File list scrolled so first visible row is index 3 → click top visible row selects 3.
- Click the already-selected row → no emit, selection unchanged.
- Click inside the diff pane → focuses diff pane, selection unchanged.
- Click empty space below the last file / outside both rects → nothing changes.

## Testing

Pure unit tests on `ChangedFilesView` (mirror the existing wheel tests in this module):
seed `files`, perform a render pass against a known `Rect` (or seed `last_files_rect` +
`file_scroll` directly), then dispatch a synthetic
`crossterm::event::MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, modifiers }`
and assert on `selected_index`, `focused_pane`, and the returned `ChangedFilesEvent`.

## Out of scope

- Drag, double-click, right-click context menus.
- Click-to-scroll on the scrollbar gutter (clicking that column may select the row under
  it; harmless and acceptable).
- Any change to the Checkpoints view (tracked separately in RPC-369).
