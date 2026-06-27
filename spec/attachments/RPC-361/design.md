# RPC-361 — Rust Checkpoints Viewer (umbrella)

## Goal
Port the original TypeScript `CheckpointViewer` (`src/tui/components/CheckpointViewer.tsx`) to the
Rust ratatui TUI (`codelet/fspec-tui/`), reusing the components and wiring patterns established by
the recently-shipped `views/changed_files/` module (RPC-354..359).

## Reference behaviour (TS — confirmed via DeepSearch)
The TS view is a **THREE-pane** browser (NOT the dual-pane ChangedFilesViewer):
- **Top-left:** Checkpoints list (~33% width)
- **Top-right:** Files list of the selected checkpoint (~67% width)
- **Bottom:** Diff pane for the selected file (~67% height), in scroll mode

Key facts:
- It does NOT reuse `FileDiffViewer`; it inlines three `VirtualList`s with one shared focus/selection
  state machine.
- Focus cycles Checkpoints → Files → Diff (Tab/→ forward, ← backward); focused pane heading turns
  green; scrollbar shown on the focused pane when content overflows.
- Checkpoint list read from `.git/fspec-checkpoints-index/*.json`, refs resolved, **sorted
  most-recent-first, capped at 200**. `name.contains("-auto-")` ⇒ automatic; auto rows render as
  `"{workUnitId}: {Phase}"`, manual rows render the raw name.
- Lazy load: per-checkpoint changed files on selection, then per-file diff (colored add/remove/hunk).
- Actions: `r/R` restore single file (files pane), `t/T` restore all, `d/D` delete one,
  `a/A` delete all (typed "DELETE ALL"), `Esc` exit. NO create-checkpoint UI.
- Mouse: wheel scroll with velocity ramp; left-click = native text-selection only (no click-to-select).

## Current Rust state (confirmed via DeepSearch)
NO checkpoints view exists. Only a 1-row aggregate count line
(`views/board/checkpoint_status.rs`, RPC-015) and `FspecBackend::checkpoint_counts()`. The board `F`
key opens ChangedFiles; there is no `C` key, no `ViewMode::Checkpoints`, no checkpoint Actions.

The backend git layer ALREADY supports the data (no git logic to reimplement):
`codelet/git/src/ghost_commit.rs`: `list_ghost_checkpoints` (552), `get_checkpoint_diff_files` (614),
`restore_ghost_commit` (447), `delete_ghost_checkpoint` (582), `count_checkpoints` (51).

## Child work units (build/review order)
1. **RPC-362** — Checkpoint transport methods (list + diff-files + file-diff + restore + delete).
2. **RPC-363** — Refactor: lift `changed_files` diff/row/scrollbar helpers into a shared module.
3. **RPC-364** — Three-pane CheckpointsView + `C`-key board wiring + Navigator/dispatch (browse + diff).
4. **RPC-365** — Restore actions (single/all) + confirmation + progress dialog.
5. **RPC-366** — Delete actions (single/all) + typed-confirmation dialog.

## Dependency graph
```
RPC-362 ─┐
RPC-363 ─┼─▶ RPC-364 ─┬─▶ RPC-365
         │            └─▶ RPC-366
```

## Definition of done (umbrella)
- Pressing `C` on the Rust board opens a three-pane Checkpoints viewer.
- User can browse checkpoints, see each one's changed files, and read colored diffs.
- User can restore (single/all) and delete (single/all) with confirmation; the board count line
  refreshes after mutations.
- All children done with 100% scenario coverage; full crate `cargo test` green; no production
  `unwrap/expect/panic`; files < 300 lines; diff/row/scrollbar logic shared DRY (not duplicated).
