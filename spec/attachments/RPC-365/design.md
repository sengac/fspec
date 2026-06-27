# RPC-365 — Checkpoint restore actions (single / all) + dialogs

## Goal
Add restore actions to the CheckpointsView (RPC-364). Restore the selected single file, or all files,
of the selected checkpoint, guarded by a confirmation dialog and a progress/status dialog.

Depends on: RPC-364 (the view) and RPC-362 (`restore_checkpoint_file` / `restore_checkpoint_all`).

## Reference behaviour (TS CheckpointViewer)
- `r/R` → restore SINGLE file — only enabled when the **Files pane is focused and files exist**.
  Confirmation dialog (yes/no, medium risk) with overwrite warning, then a StatusDialog
  (restoring → complete | error). On success, reload the diff pane.
- `t/T` → restore ALL files of the selected checkpoint (enabled when checkpoints exist).
  Confirmation dialog naming "ALL N files" (high risk), then StatusDialog.
- Both notify the board (TS used IPC `checkpoint-changed`); in Rust, refresh `checkpoint_counts`
  and/or the changed-files-derived state so the board's count line stays accurate.

## Rust mapping
- Keys handled in `CheckpointsView` (or a small dialog sub-state machine). Pressing the key sets a
  pending `RestoreMode::{Single, All}` and opens a confirmation dialog.
- Confirmation + status dialogs: model on an existing Rust TUI dialog (e.g.
  `components/disconnect_dialog.rs` pattern) — a modal state the view renders over the panes and that
  captures input while active.
- On confirm → emit an Action (`Action::RestoreCheckpointFile{...}` / `RestoreCheckpointAll{...}`);
  `App::dispatch` spawns the transport call (`restore_checkpoint_file` / `restore_checkpoint_all`),
  then folds back a result Action that drives the StatusDialog to complete/error.
- On success of a single-file restore, re-request `checkpoint_file_diff` to refresh the diff pane.

## Acceptance criteria
- `r/R` is a no-op unless the Files pane is focused AND the selected checkpoint has files; otherwise it
  opens a single-file restore confirmation naming the selected file.
- `t/T` opens a restore-ALL confirmation naming the count of files (or "all files") for the selected
  checkpoint.
- Confirming a restore dispatches the matching transport call; the status dialog shows
  restoring → complete on success and restoring → error (with the error message) on failure.
- Cancelling a confirmation dialog returns to the view with no transport call made.
- After a successful single-file restore, the diff pane reloads for the (now restored) file.
- After any successful restore, the board checkpoint count / changed-files state is refreshed.
- No `unwrap/expect/panic` in production paths; files < 300 lines.

## Open question to resolve in Example Mapping
- Exact risk levels / whether restore-all needs a typed confirmation (TS used plain yes/no for restore,
  typed only for delete-all). Default: restore uses yes/no confirmation; delete-all (RPC-366) uses typed.

## Key files
- `codelet/fspec-tui/src/views/checkpoints/` (key handling + dialog state + render)
- New/modified dialog component under `components/`
- `app/dispatch_checkpoints.rs`, `components/mod.rs` (Action variants)
- `transport/*` restore methods from RPC-362
- Feature: extend `spec/features/rust-checkpoints-view.feature` (or a dedicated restore feature)
