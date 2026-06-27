# RPC-356 — Dual-pane `ChangedFilesView` with F-key board wiring and Navigator integration

Parent: **RPC-354**. Depends on: **RPC-355** (provides `changed_files()` + `file_diff()`).

This is the **UI** layer: build the view, wire the `F` key, integrate `ViewMode::ChangedFiles`.

## TypeScript reference

`src/tui/components/ChangedFilesViewer.tsx` + the shared `FileDiffViewer.tsx`. Behaviour to match:

- **Left pane — file list.** One row per changed file: a selection cursor (`>` for the selected row,
  space otherwise), a colored single-letter status, then the path. Colors:
  **A=green, M=yellow, D=red, R=cyan** (default M=yellow). Truncate long paths.
- **Right pane — diff.** Shows the unified diff of the selected file, colored: `+` add lines green,
  `-` remove lines red, hunk headers (`@@`) dim/cyan, context lines default.
- **Focus** starts on the file list.
- **Keys:** `Esc` → back to board; `Tab` / `Right` / `Left` → toggle pane focus; `Up`/`Down` →
  move file selection (and reload the diff for the new selection); `PgUp`/`PgDn` → scroll the focused
  pane. Mouse wheel scrolls (reuse `scroll_viewport` / `WheelVelocity`, the RPC-353 infra).
- **Footer:** `ESC: Back | Tab: Switch Panes | ↑↓: Navigate | PgUp/PgDn: Scroll`.
- **Empty state:** when there are no changed files, show a friendly message (e.g.
  `No changed files`) and Esc still returns to the board.

## Architecture — follow the existing Navigator/Action pattern

The board emits Actions; `Navigator::apply_action` flips `ViewMode`; `Navigator::handle_event`
routes events to the active view. Mirror how `ProviderSettings` / `ModelSelector` are wired.

### 1. Board key — `views/board.rs` (`BoardView::handle_event`)
Add an arm: on `KeyCode::Char('f') | KeyCode::Char('F')` emit `Action::OpenChangedFilesView` and
return `EventResult::consumed()`. (Other keys are unchanged; this currently falls through to
`ignored()`.)

### 2. Action variants — `components/mod.rs` (Action enum)
- `OpenChangedFilesView`
- `CloseChangedFilesView`
- `ChangedFilesLoaded(Vec<ChangedFile>)` — populates the view's file list.
- `FileDiffLoaded { path: String, diff: Option<String> }` — populates the diff pane.
  (Or have the App load the diff on selection-change; pick one and keep it consistent.)

### 3. `ViewMode::ChangedFiles` — `views/navigator.rs`
- Add the enum variant.
- Add an owned `changed_files: ChangedFilesView` field on `Navigator`.
- `apply_action`: `OpenChangedFilesView` → set `active_view = ViewMode::ChangedFiles` and kick off a
  `changed_files()` load; `CloseChangedFilesView` / Esc → back to `ViewMode::Board`.
- `handle_event`: when `active_view == ChangedFiles`, forward events to
  `self.changed_files.handle_event(...)` (translate its outcome enum to bus actions, mirroring
  `navigator_events.rs::handle_*`).
- `render_with_stores`: render the view when active.

### 4. Data loading — `App::dispatch`
On `OpenChangedFilesView`, call `backend.changed_files().await`, dispatch `ChangedFilesLoaded`.
On file-selection change, call `backend.file_diff(path).await`, dispatch `FileDiffLoaded`.
Mirror how `checkpoint_counts` → `CheckpointCountsLoaded` flows.

### 5. The view component — `views/changed_files/` (new module)
Keep each file under 300 lines (split: `mod.rs` state + event handling, `render.rs` panes,
`row.rs` file-row formatting, `diff_render.rs` colored diff lines). A `ChangedFilesView` struct holds:
`files: Vec<ChangedFile>`, `selected_index`, `focused_pane: Pane { Files, Diff }`,
`diff_lines`/scroll offsets, and a `wheel: WheelVelocity`.

## Rules to encode (Example Map)
- Pressing `F` (or `f`) on the board opens the Changed Files view.
- The view lists staged then unstaged changed files, each with a colored A/M/D/R status and the path.
- The currently selected file row shows the `>` cursor; others show a space.
- The diff pane shows the unified diff of the selected file with colored +/- lines.
- Moving the selection with Up/Down reloads the diff pane for the newly selected file.
- Tab (and Left/Right) toggles focus between the file list and the diff pane.
- PgUp/PgDn and the mouse wheel scroll the focused pane (WheelVelocity ramp, like the chat view).
- Esc returns to the board.
- With no changed files, the view shows an empty-state message and Esc still returns to the board.

## Examples (green cards)
- Repo with `a.txt` modified + `b.txt` newly added: list shows `M a.txt` (yellow M) and `A b.txt`
  (green A); selecting `a.txt` shows its diff with green/red lines.
- Pressing Down from `a.txt` to `b.txt` swaps the diff pane to `b.txt`'s diff.
- Pressing Tab moves focus to the diff pane; PgDn then scrolls the diff, not the list.
- In a clean repo, opening the view shows `No changed files`; Esc returns to the board.

## Testing
- Component tests for `ChangedFilesView` (ratatui `TestBackend` buffer assertions, as used by the
  other views): row rendering with status colors + cursor, diff coloring, pane-focus toggle,
  selection navigation reloading the diff, empty state, Esc → close.
- A board-level test: `F` emits `OpenChangedFilesView`; Navigator flips to `ViewMode::ChangedFiles`.
- `cargo build` + `cargo test` for `codelet-fspec-tui`.

## Constraints
- No `unwrap()`/`expect()` in production paths.
- Files under 300 lines (split the module).
- Every Gherkin step needs a matching `// @step` comment in the test.
- Reuse `scroll_viewport` / `WheelVelocity`; do not hand-roll new scroll math.
