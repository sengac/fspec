# BUG-184 Research — Mux Files/Checkpoints 10s refresh

## Symptoms (user report)

The recent mux fixes (BUG-182 `GitStateWatcher`) poll git state every 10 s
and push a `GitState` frame containing `changed_files` + `checkpoints`.
Two problems observed:

1. **No loading dialog in mux mode** for the Files/Checkpoints panes while
   loading — single-view mode shows one.
2. **Losing scroll position** (and flicker) on every 10 s refresh of the
   Files/Checkpoints list.

## User decisions (Example Mapping — red cards answered)

- **dialog_scope: "Initial only"** — show the full loading dialog when a
  mux pane first loads (matching F/C single-view behaviour). Keep the
  10 s background refresh seamless (no dialog), but preserve scroll +
  selection so it doesn't flicker or jump.
- **scroll_scope: "All panes"** — preserve scroll in every sub-pane
  (checkpoint list, Files list, Diff pane).

## Root-cause analysis

1. `App::handle_git_state_changed` (dispatch_git_state.rs) re-fetches a
   visible lazy view on EVERY `GitState` frame — even when the frame's
   list is byte-identical to what the pane already displays (the 10 s
   poll's steady-state tick). The re-fetch re-runs the cascade
   (files + diff) and churns the pane.
2. `refresh_files_preserving_selection` /
   `refresh_checkpoints_preserving_selection` reset the list scroll to 0
   whenever the path-stable re-selection fell below the old window —
   so the visible window jolted to the top on the steady-state tick.
3. Checkpoints `set_files` / `set_diff` unconditionally reset
   `selected_file`, `file_scroll`, `diff_scroll` on every cascade reload —
   even for the SAME (work_unit, name[, path]) key the pane already
   displays.

## Design (working-tree changes at discovery)

1. **Signature-based no-op drop** (before any re-fetch):
   - `LoadTracker::list_signature` records the identity of the list the
     view currently displays (set on `set_files` / `set_checkpoints` /
     refresh paths).
   - Frame legs are signed with the SAME shape the view stores
     (sanitized fields, list order; checkpoints exclude the timestamp
     leg — row rendering never shows it and the fallback "now" stamp
     churns when no index sidecar exists; order DOES participate).
   - `refresh_changed_files_view(state)` /
     `refresh_checkpoints_view(state)` drop the refresh when the
     signatures match → no re-fetch, no dialog flash, no scroll/selection
     churn. The 10 s tick becomes a no-op on the pane.
2. **Scroll preservation** in the refresh paths:
   - Preserve the list scroll offset, clamped to the fresh list length
     (`list_len - visible_rows` max; shorter-than-window lists fall back
     to the top).
   - Selection/scroll consistency: if the re-looked-up row falls outside
     the preserved window, the selection snaps to the window's anchor
     row (first visible for a fallback; last visible when the selection
     slid past the bottom) and the window snaps to it — highlighted row
     and scroll window never diverge (same rule `ensure_visible` uses).
3. **Cascade same-key preservation** (checkpoints):
   - `set_files` with the SAME `(work_unit_id, name)` key keeps
     `selected_file` + `file_scroll`; a key change resets both.
   - `set_diff` with the SAME `(work_unit_id, name, path)` key clamps
     `diff_scroll` to the fresh diff length; a key change resets it.

## Files touched (uncommitted at discovery)

- `rust/fspec-tui/src/app/dispatch_git_state.rs` — signature drop
  (`changed_files_signature`, `checkpoints_signature`), refresh fns take
  `&GitState`.
- `rust/fspec-tui/src/components/load_state.rs` — `list_signature` /
  `set_list_signature` on `LoadTracker` + unit test.
- `rust/fspec-tui/src/views/changed_files/mod.rs` — `list_signature`,
  scroll preservation in `refresh_files_preserving_selection`.
- `rust/fspec-tui/src/views/changed_files/tests.rs` — 4 BUG-184 tests
  (scroll stable on same-list tick; stable when rows above disappear;
  clamp when list shrinks beneath window; fallback snap when selected
  file vanished).
- `rust/fspec-tui/src/views/checkpoints/mod.rs` — `list_signature`,
  scroll preservation in refresh + same-key cascade preservation +
  `checkpoint_scroll()` test seam.

## Open question at discovery

Whether the mux-mode initial-load loading dialog (problem 1, "Initial
only" scope) already renders — needs verification of the mux-aware
loading gate; if the gate is already correct, only the refresh-seamless
half of the decision is net-new work.
