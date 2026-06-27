# AST Research — Checkpoints view integration surface (RPC-361 umbrella)

Survey of the delivered `codelet/fspec-tui/src/views/checkpoints/` module public/`pub(super)`
surface (AstGrep `pub fn $NAME($$$ARGS) -> $RET`), confirming the umbrella capability is wired
end-to-end across its children.

## Public surface by file
- **checkpoint_row.rs** — `checkpoint_label(&CheckpointInfo) -> String` (auto `{id}: {Phase}` / manual raw name) [RPC-364]
- **mod.rs** — state machine: `new`, `handle_event`, `focused_pane`, `selected_checkpoint`,
  `selected_file`, `diff_scroll`, `is_empty`, `selected_checkpoint_info`, `selected_file_path`,
  `first_file_path`, `dialog`, `delete_dialog` accessors [RPC-364/365/366]
- **keys.rs** — `handle_key`, `handle_mouse` (pane-aware arrows; wheel emits load actions) [RPC-364]
- **navigation.rs** — `scroll_focused`, `move_checkpoint_selection`, `move_file_selection`,
  `page_step` (reuse WheelVelocity/ensure_visible) [RPC-364]
- **restore.rs** — `open_restore_single`, `open_restore_all`, `handle_dialog_key`,
  `on_restore_result` [RPC-365]
- **dialog.rs** — `RestoreDialog::confirm`, `title`, `body_lines` [RPC-365]
- **delete.rs** — `open_delete_single`, `open_delete_all`, `handle_delete_dialog_key`,
  `on_delete_result`, `delete_dialog` [RPC-366]
- **delete_dialog.rs** — `DeleteDialog::confirm_single`, `confirm_all`, `all_confirm_ready`
  (typed `DELETE ALL` gate), `title`, `body_lines` [RPC-366]

## Wiring (verified)
board `C` → `Action::OpenCheckpointsView` → `ViewMode::Checkpoints` (navigator) →
`app/dispatch_checkpoints.rs` lazy loads via RPC-362 transport (`list_checkpoints` →
`checkpoint_diff_files` → `checkpoint_file_diff`); restore/delete dispatch via
`dispatch_checkpoint_restore.rs` / `dispatch_checkpoint_delete.rs` → RPC-362 mutating methods →
result actions folded back into the view; rendering reuses RPC-363 `views/diff_common` helpers.

## Conclusion
The umbrella capability is fully realized and integrated; acceptance is covered by the children's
feature files (checkpoint-transport, shared-diff-view-components, rust-checkpoints-view,
checkpoint-restore, checkpoint-delete), all at 100% scenario coverage.
