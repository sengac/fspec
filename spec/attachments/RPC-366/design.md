# RPC-366 — Checkpoint delete actions (single / all) + typed-confirmation dialog

## Goal
Add delete actions to the CheckpointsView (RPC-364): delete the selected single checkpoint, or delete
ALL checkpoints, guarded by confirmation dialogs (typed confirmation for delete-all).

Depends on: RPC-364 (the view) and RPC-362 (`delete_checkpoint` / `delete_all_checkpoints`).

## Reference behaviour (TS CheckpointViewer)
- `d/D` → delete SINGLE selected checkpoint — yes/no confirmation (medium risk). On success, remove it
  from the list; if none remain → exit to board; otherwise clamp `selected_checkpoint` and refresh.
- `a/A` → delete ALL checkpoints — **typed confirmation** requiring the phrase `"DELETE ALL"`
  (high risk). On success → exit to board.
- Both notify the board (IPC `checkpoint-changed` in TS) → in Rust, refresh `checkpoint_counts` so the
  board count line updates.

## Rust mapping
- Keys set a pending `DeleteMode::{Single, All}` and open the appropriate confirmation dialog.
- Single: yes/no confirmation modal. All: typed-confirmation modal that only enables confirm when the
  user types `DELETE ALL` exactly.
- On confirm → emit `Action::DeleteCheckpoint{...}` / `Action::DeleteAllCheckpoints`; `App::dispatch`
  spawns the transport call, then folds back a result Action.
- After delete: update `checkpoints` (remove single, or clear all), clamp selection, reload the files +
  diff for the new selection; if the list becomes empty → `Action::CloseCheckpointsView` (back to board).
- Refresh the board `checkpoint_counts` after a successful delete.

## Acceptance criteria
- `d/D` (with checkpoints present) opens a yes/no confirmation naming the selected checkpoint.
- `a/A` opens a typed-confirmation dialog that requires exactly `DELETE ALL` before confirm is enabled.
- Confirming single delete dispatches `delete_checkpoint`, removes the row, clamps selection, and reloads
  the now-selected checkpoint's files/diff.
- Deleting the LAST checkpoint returns the view to the board.
- Confirming delete-all dispatches `delete_all_checkpoints` and returns to the board.
- Cancelling either dialog makes no transport call and leaves the list unchanged.
- After any successful delete, the board checkpoint count is refreshed.
- No `unwrap/expect/panic` in production paths; files < 300 lines.

## Key files
- `codelet/fspec-tui/src/views/checkpoints/` (key handling + delete dialog state + render)
- Typed-confirmation dialog component under `components/` (new or reused)
- `app/dispatch_checkpoints.rs`, `components/mod.rs` (Action variants)
- `transport/*` delete methods from RPC-362
- Feature: extend `spec/features/rust-checkpoints-view.feature` (or a dedicated delete feature)
