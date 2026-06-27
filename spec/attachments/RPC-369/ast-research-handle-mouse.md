# RPC-369 — AST research for click-to-select in CheckpointsView

Tool: AstGrep (language: rust)
Targets: codelet/fspec-tui/src/views/checkpoints/{keys.rs,navigation.rs,mod.rs}

## Functions located (the click handler will reuse / extend these)

| Pattern | Location | Role |
|---|---|---|
| `pub(super) fn handle_mouse(&mut self, ev: MouseEvent) -> CheckpointsEvent { $$$BODY }` | keys.rs:57 | Mouse entry point. Restore/delete dialog guard returns `Consumed` FIRST; then only ScrollUp/ScrollDown handled, `_ => Ignored` drops `Down(_)`. A `MouseEventKind::Down(_)` arm must be added AFTER the dialog guard, BEFORE the wheel match. |
| `pub(super) fn move_checkpoint_selection(&mut self, delta: i32) -> CheckpointsEvent { $$$BODY }` | navigation.rs:42 | Clamps, ensure_visible, clear_files, emits `Emit(Action::LoadCheckpointFiles)` or `Consumed` when index unchanged. Reused for checkpoint-row clicks. |
| `move_file_selection(delta)` | navigation.rs:74 | Clamps, ensure_visible, emits `Emit(Action::LoadCheckpointFileDiff)` or `Consumed` when unchanged. Reused for file-row clicks. |
| `pane_at(col, row) -> Option<Pane>` | mod.rs:240 | Hit-tests Diff → Files → Checkpoints content rects. |

## Findings
- `last_checkpoints_rect` / `last_files_rect` are CONTENT rects (header/underline excluded by `pane_header`), so `clicked_index = scroll + (row - rect.y)`.
- Dialog guard `self.dialog().is_some() || self.delete_dialog().is_some()` (dialog() mod.rs:226, delete_dialog() delete.rs:203) MUST run before the Down arm so a click is swallowed while a modal is open.
- `focused_pane` is a private field; set directly (mirroring `cycle_pane`). Public reader `focused_pane()` at mod.rs:204.
- No App/Navigator change needed: `Event::Mouse` flows into handle_mouse and `Emit` is relayed by navigator_events.rs.
- Mirrors the RPC-368 pattern established in views/changed_files/mouse.rs.
