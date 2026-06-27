# RPC-364 — Three-pane CheckpointsView + C-key wiring + Navigator/dispatch

## Goal
Greenfield `views/checkpoints/` module implementing the three-pane Checkpoints browser (browse +
diff only; restore/delete come in RPC-365/366). Mirror the `views/changed_files/` architecture and
wiring, but with THREE panes and a focus state machine.

Depends on: RPC-362 (transport) and RPC-363 (shared diff/row/scrollbar helpers).

## Layout (three panes)
- Top row split horizontally: **Checkpoints** list (~33%) | **Files** list (~67%)
- Bottom: **Diff** pane (full width, ~67% height), scroll mode
- Focused pane heading highlighted (green bg / black text, like the TS reference); other headings plain.

## State machine
`CheckpointsView` struct fields (model on `ChangedFilesView`):
- `checkpoints: Vec<CheckpointInfo>`, `selected_checkpoint: usize`, `checkpoint_scroll: usize`
- `files: Vec<ChangedFile>`, `selected_file: usize`, `file_scroll: usize`
- `diff_lines: Vec<String>` (or parsed), `diff_scroll: usize`
- `focused: Pane` (Checkpoints | Files | Diff)
- `wheel: WheelVelocity`, cached rects per pane
- loading flags for lazy loads

Focus cycling: **Tab / Right → forward** (Checkpoints→Files→Diff→Checkpoints); **Left → backward**.
Up/Down/PgUp/PgDn act on the focused pane (lists move selection, diff scrolls) — apply the
RPC-358 pane-aware lesson: arrows scroll the diff when the Diff pane is focused, move selection in
list panes.

## Data flow (lazy, mirrors changed_files loaded-action pattern)
1. `Action::OpenCheckpointsView` → Navigator flips to `ViewMode::Checkpoints`; `App::dispatch` calls
   `backend.list_checkpoints()` → `Action::CheckpointsLoaded(list)`.
2. Selecting a checkpoint (or initial load) → `backend.checkpoint_diff_files(work_unit_id, name)` →
   `Action::CheckpointFilesLoaded { ... }`.
3. Selecting a file → `backend.checkpoint_file_diff(work_unit_id, name, path)` →
   `Action::CheckpointFileDiffLoaded { ... }` (drop stale results whose key ≠ current selection,
   like `ChangedFilesView::set_diff`).

## Rendering rules
- Checkpoints list: automatic checkpoints render `"{workUnitId}: {Phase}"` (Phase = capitalized state
  parsed from the `-auto-<state>` suffix); manual checkpoints render the raw `name`. Selected row shows
  a `>` cursor / highlight.
- List sorted most-recent-first, capped 200 (enforced by transport, but view must not assume more).
- Files pane uses the shared `file_row` helper; Diff pane uses shared `diff_line`.
- Scrollbars: reuse shared pane-scrollbar helper; show on a pane only when its content overflows
  (apply the RPC-359 rule). Match the TS "scrollbar on focused pane" or "on any overflowing pane" —
  pick one in Example Mapping and encode it in scenarios.
- Empty state: "No checkpoints available" when the list is empty; Esc still returns to board.

## Wiring
- `views/board.rs`: add `C`/`c` key → `Action::OpenCheckpointsView` (sibling to the `F` handler).
- `components/mod.rs`: add `OpenCheckpointsView`, `CloseCheckpointsView`, `CheckpointsLoaded(...)`,
  `CheckpointFilesLoaded{...}`, `CheckpointFileDiffLoaded{...}` Action variants.
- `navigator.rs`: add `ViewMode::Checkpoints` + owned `CheckpointsView`; route events via
  `navigator_events.rs` (mirror `handle_changed_files_event`).
- `app/dispatch*.rs`: add a `dispatch_checkpoints.rs` mirroring `dispatch_changed_files.rs`
  (spawn tokio tasks for the three loads; fold results into the view).
- Esc → `Action::CloseCheckpointsView` → back to Board.

## Acceptance criteria
- Pressing `C` on the board opens the Checkpoints view (emits `Action::OpenCheckpointsView`, consumes key).
- Navigator flips to `ViewMode::Checkpoints` on open and back to `Board` on close.
- The checkpoints list renders auto rows as `"{id}: {Phase}"` and manual rows as the raw name, with a
  selection cursor; sorted most-recent-first.
- Selecting a checkpoint loads & shows its changed files; selecting a file loads & shows its colored diff.
- Tab/Right cycles focus forward across the three panes; Left cycles backward; the focused pane heading
  is highlighted.
- Arrow keys act on the focused pane (list selection vs diff scroll); scrollbars appear only on overflow.
- Empty repo shows "No checkpoints available"; Esc returns to the board.
- No restore/delete yet (those are RPC-365/366).

## Constraints
- Reuse shared helpers from RPC-363 (no duplicated diff/row/scrollbar logic).
- Reuse `WheelVelocity`/`ensure_visible` for scroll math.
- Files < 300 lines (split mod/render/row-of-checkpoint/dispatch as needed).
- No `unwrap/expect/panic` in production paths.

## Key files
- New: `codelet/fspec-tui/src/views/checkpoints/{mod.rs,render.rs,checkpoint_row.rs,tests.rs}`,
  `app/dispatch_checkpoints.rs`
- Modified: `views/board.rs`, `views/navigator.rs`, `views/navigator_events.rs`, `views/mod.rs`,
  `components/mod.rs`, `app/dispatch.rs`/`bootstrap.rs`
- Feature: new `spec/features/rust-checkpoints-view.feature`
