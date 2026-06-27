# AST Research — RPC-364 CheckpointsView

Reuse target patterns found via AstGrep + Grep across the fspec-tui crate.

## ChangedFilesView (model to mirror)
- `codelet/fspec-tui/src/views/changed_files/mod.rs:78` — `impl ChangedFilesView` holds:
  state machine (`Pane` enum Files|Diff), `handle_event` → `handle_key`/`handle_mouse`,
  `toggle_pane`, `move_selection` (emits `LoadFileDiff`), `scroll_focused`,
  `apply_diff_scroll`/`max_diff_scroll`, `set_files`/`set_diff` (stale-drop by path),
  cached `last_files_rect`/`last_diff_rect`, `WheelVelocity`.
- `render.rs` — `render_full_screen_scaffold_raw_title`, `pane_header` (focus highlight green/black),
  `render_files_pane`/`render_diff_pane` using shared `file_row`/`diff_line`/`render_pane_scrollbar`,
  scrollbar only on overflow (reclaim gutter column otherwise).

## Shared helpers (RPC-363, no duplication)
- `views/diff_common/mod.rs` exports `diff_line`, `file_row`, `status_color`, `truncate_path`,
  `render_pane_scrollbar`.
- `components/scroll_viewport`: `WheelVelocity`, `ensure_visible`, `WheelDirection`.
- `components/list_scrollbar::render_list_scrollbar`.

## Transport (RPC-362)
- `transport/mod.rs:123` `list_checkpoints() -> Vec<CheckpointInfo>`
- `:129` `checkpoint_diff_files(work_unit_id, name) -> Vec<ChangedFile>`
- `:139` `checkpoint_file_diff(work_unit_id, name, path) -> Option<String>`
- `CheckpointInfo` DTO: `work_unit_id`, `name`, `timestamp`, `is_automatic` (rpc-types/src/lib.rs:117).

## Wiring points
- `views/board.rs:175` — `KeyCode::Char('f')|Char('F') => emit(Action::OpenChangedFilesView)`.
  Add sibling `Char('c')|Char('C') => emit(Action::OpenCheckpointsView)`.
- `components/mod.rs:966-970` — Action variants for changed_files. Add Checkpoints variants.
- `views/navigator.rs:47` — `ViewMode::ChangedFiles`; `:140-145` apply_action flips; `:183` render.
- `views/navigator_events.rs:144` — `handle_changed_files_event` translation.
- `app/dispatch.rs:279` — `try_dispatch_changed_files` in fallback chain.
- `app/dispatch_changed_files.rs` — lazy spawn pattern (tokio::spawn → action_tx.send).
- `app/mod.rs:23` — `pub mod dispatch_changed_files;`.

## Naming for the auto/manual label
- `CheckpointInfo.is_automatic` + `name` like `AUTH-001-auto-testing`. Parse `-auto-<state>`
  suffix → capitalize state → render `"{work_unit_id}: {Phase}"`. Manual → raw `name`.
