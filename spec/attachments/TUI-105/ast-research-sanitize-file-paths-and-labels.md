# AST Research: Sanitize File Paths and Labels in Changed Files and Checkpoint Views

## Key Functions Found

### `file_row()` — `views/diff_common/row.rs:51-68`
- Takes `ChangedFile` with `path` and `change_type` fields
- Renders raw `file.path` via `truncate_path()` then `Span::styled()`
- Renders raw `file.change_type` via `Span::styled()`
- **No sanitization applied**

### `checkpoint_line()` — `views/checkpoints/render.rs:180-198`
- Takes `CheckpointInfo` and builds a label via `checkpoint_label()`
- Renders raw label via `truncate_path()` then `Span::styled()`
- **No sanitization applied**

### `sanitize_for_terminal()` — `store/agent_view/sanitize.rs:33-50`
- Now re-exported from `lib.rs` as `pub use store::agent_view::sanitize::sanitize_for_terminal`
- Accessible from `views/` modules via `crate::sanitize_for_terminal`

## Implementation Plan

1. Modify `file_row()` in `row.rs` to sanitize `file.path` and `file.change_type`
2. Modify `checkpoint_line()` in `checkpoints/render.rs` to sanitize the label
3. Write tests for both functions
