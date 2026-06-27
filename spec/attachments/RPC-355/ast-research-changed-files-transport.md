# AST Research — RPC-355 changed-files transport

AST analysis performed with the AstGrep tool to confirm the reuse map and the
integration template before writing tests.

## codelet/git — existing status helpers (reuse, do not reimplement)

Pattern: `pub fn get_staged_files($$$ARGS) -> $RET { $$$BODY }`
- `codelet/git/src/status.rs:15` — `pub fn get_staged_files(dir: impl AsRef<Path>) -> Result<Vec<String>>`

Sibling helpers confirmed in the same file by reading: `get_unstaged_files`,
`get_untracked_files`. These return paths only. The new
`get_staged_files_with_change_type` / `get_unstaged_files_with_change_type`
helpers wrap the SAME index/HEAD-tree/workdir inspection already used here to
derive A/M/D without shelling out.

## codelet/rpc — FspecService gating template (RPC-015)

Pattern: `async fn checkpoint_counts($$$ARGS) -> $RET { $$$BODY }`
- `codelet/rpc/src/lib.rs:896` — `async fn checkpoint_counts(self, _ctx: Context) -> CheckpointCounts`
  body matches on `self.inner.cwd()` → delegate when `Some(cwd)`, return the
  zero default when `None`.

`changed_files` / `file_diff` follow this exact gating shape (empty Vec / None
when no cwd).

## Transport surface

`FspecBackend` trait (transport/mod.rs) carries default impls per the
RPC-037 / PROV-109 convention; `embedded.rs` uses one-line
`self.client.<rpc>(context::current(), ...).await` delegates; `websocket.rs`
uses the `client.read().await` + `BackendError::Disconnected` guard. Confirmed
by reading all three files.
