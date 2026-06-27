# RPC-362 — Checkpoint transport methods

## Goal
Expose the existing `codelet/git` ghost-commit checkpoint helpers to the Rust TUI through the
`FspecBackend` transport trait, so the CheckpointsView (RPC-364/365/366) can list checkpoints, load
per-checkpoint changed files and per-file diffs, and perform restore/delete — all WITHOUT
reimplementing any git logic.

## Pattern to mirror
`FspecBackend::checkpoint_counts()` is the existing template:
- Trait method: `codelet/fspec-tui/src/transport/mod.rs:99`
- Embedded impl: `codelet/fspec-tui/src/transport/embedded.rs:112` → `self.client.checkpoint_counts(context::current())`
- WebSocket impl: `codelet/fspec-tui/src/transport/websocket.rs:277` → tarpc call
- tarpc service: `codelet/rpc/src/lib.rs:919` (`FspecService::checkpoint_counts` → `count_checkpoints`)
- rpc-types: `codelet/rpc-types/src/lib.rs` (`CheckpointCounts`)

## Backend helpers to delegate to (already exist — do NOT rewrite)
`codelet/git/src/ghost_commit.rs`:
- `count_checkpoints(dir) -> Result<CheckpointCounts>` (51)
- `restore_ghost_commit(...)` (447)
- `list_ghost_checkpoints(dir, work_unit_id) -> Result<Vec<String>>` (552)
- `delete_ghost_checkpoint(...)` (582)
- `get_checkpoint_diff_files(...)` (614)
Plus the existing checkpoint index reader in `codelet/fspec-core` (the `.git/fspec-checkpoints-index/*.json`
parsing used by the CLI `list-checkpoints` command) and the same per-file diff helper the changed_files
transport uses (`file_diff`).

## New transport methods to add
Add to the `FspecBackend` trait (+ embedded + websocket impls + tarpc service + rpc-types DTOs):

1. `list_checkpoints() -> Result<Vec<CheckpointInfo>>`
   - `CheckpointInfo { work_unit_id, name, timestamp, is_automatic }` (new rpc-type).
   - Reads the checkpoint index across all work units, resolves refs, sorts most-recent-first,
     caps at 200. `is_automatic = name.contains("-auto-")`.
2. `checkpoint_diff_files(work_unit_id: String, name: String) -> Result<Vec<ChangedFile>>`
   - Delegates to `get_checkpoint_diff_files`. Reuse the existing `ChangedFile` rpc-type from the
     changed_files transport (status + path).
3. `checkpoint_file_diff(work_unit_id, name, path) -> Result<Option<String>>`
   - Unified diff of one file against the checkpoint ref (reuse the existing checkpoint-file-diff
     git helper). `None`/empty when no diff.
4. `restore_checkpoint_file(work_unit_id, name, path) -> Result<()>`
5. `restore_checkpoint_all(work_unit_id, name) -> Result<()>` → `restore_ghost_commit`.
6. `delete_checkpoint(work_unit_id, name) -> Result<()>` → `delete_ghost_checkpoint` + index removal.
7. `delete_all_checkpoints() -> Result<()>` → delete every checkpoint across work units + unlink index files.

## Default trait impls
Follow the `changed_files()`/`file_diff()` precedent (mod.rs:108/115): provide default impls that
return empty/Ok so non-git backends compile, with the real work in the embedded/websocket impls.

## Acceptance criteria
- All seven methods exist on the trait with embedded + websocket implementations and a tarpc service
  method each; rpc-types DTOs (`CheckpointInfo`) added and serializable.
- `list_checkpoints` returns checkpoints sorted most-recent-first, capped at 200, with correct
  `is_automatic` derivation.
- `checkpoint_diff_files` / `checkpoint_file_diff` return the same shapes the view expects
  (`ChangedFile` / `Option<String>`), delegating to the existing git helpers.
- restore/delete methods call the corresponding ghost_commit helpers and propagate errors via
  `Result` (no `unwrap`/`expect`/`panic` in production paths).
- Unit/integration tests cover: list sorting+cap+automatic flag, diff-files delegation, file-diff
  delegation, and at least one restore + one delete happy path (use a temp git repo helper like the
  existing `codelet/git/tests`).

## Key files
- `codelet/fspec-tui/src/transport/mod.rs`, `embedded.rs`, `websocket.rs`
- `codelet/rpc/src/lib.rs` (tarpc service), `codelet/rpc-types/src/lib.rs` (DTOs)
- `codelet/git/src/ghost_commit.rs` (delegate only), `codelet/fspec-core` checkpoint index reader
- Tests: `codelet/git/tests/` and/or transport-level tests
