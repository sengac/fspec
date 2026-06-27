# RPC-362 AST Research — Checkpoint transport helper delegation

Confirms the existing `codelet_git::ghost_commit` public helpers the transport
methods delegate to (no git logic reimplemented). Pattern:
`pub fn $NAME($$$ARGS) -> Result<$RET> { $$$BODY }` over
`codelet/git/src/ghost_commit.rs`.

| Line | Helper |
|------|--------|
| 51 | `count_checkpoints` (existing checkpoint_counts plumbing — pattern to mirror) |
| 126 | `create_ghost_commit` (used by tests to build real checkpoints) |
| 447 | `restore_ghost_commit` → `restore_checkpoint_all` |
| 527 | `restore_ghost_commit_file` (NEW, RPC-362) → `restore_checkpoint_file` |
| 603 | `list_ghost_checkpoints` (per-work-unit) |
| 640 | `list_all_ghost_checkpoints` (NEW, RPC-362) → `list_checkpoints` |
| 673 | `delete_ghost_checkpoint` → `delete_checkpoint` |
| 705 | `get_checkpoint_diff_files` → `checkpoint_diff_files` |

Plus `codelet_git::get_checkpoint_file_diff` (diff.rs:158) → `checkpoint_file_diff`.

Transport plumbing mirrored from `checkpoint_counts()`:
- trait `FspecBackend` — `codelet/fspec-tui/src/transport/mod.rs`
- embedded — `embedded.rs` (one-line `self.client.<rpc>(context::current()).await`)
- websocket — `websocket.rs` (guarded `BackendError::Disconnected` delegate)
- tarpc service — `codelet/rpc/src/lib.rs` (`SharedFspecService`, gated on `self.inner.cwd()`)
- rpc-types DTO — `CheckpointInfo` in `codelet/rpc-types/src/lib.rs`
- shared business logic — new `codelet/rpc/src/checkpoints.rs` (like `changed_files.rs`)
