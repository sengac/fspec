# RPC-057 — AST research on existing surfaces being extended

This document records AST-level findings used during Example Mapping for
RPC-057, and ties each finding back to a concrete decision captured in
the rule/example/architecture-note set.

---

## codelet-git: the underlying primitives we delegate to

```text
codelet/git/src/session_status.rs:504:1:
  pub fn merge_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<MergeResult>

codelet/git/src/session_status.rs:577:1:
  pub fn discard_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<DiscardResult>

codelet/git/src/session_status.rs:694:1:
  pub fn prune_orphaned(repo_path: impl AsRef<Path>, active_sessions: &HashSet<String>) -> Result<PruneResult>

codelet/git/src/worktree.rs:169:1:
  pub fn list_worktrees(repo_path: impl AsRef<Path>) -> Result<Vec<WorktreeInfo>>

codelet/git/src/session_status.rs:436:1:
  pub fn inspect_session(repo_path, session_id) -> Result<SessionResult>
```

Conflict detection lives at `codelet/git/src/error.rs`:

```rust
GitError::ConflictError { files: Vec<String> }
```

→ Confirms that the codelet-sessions handle_impl can map
`Err(GitError::ConflictError { files })` directly into
`Ok(MergeOutcome { status: Conflict, conflicts: files, merge_commit: None })`,
and any other error variant becomes `Err(format!("{}", err))`.

The `MergeResult` does NOT currently carry a commit SHA, so the wire's
`merge_commit: Option<String>` field will be `None` until a future
codelet-git change surfaces it. This is captured in the architecture
note "Mapping codelet-git results to the new wire types".

`WorktreeInfo` does NOT carry a `base_commit` field — only
`session_id`, `path`, `head_commit`, `is_detached`. The base commit
must be read from the on-disk session manifest
(`codelet_git::read_manifest(session_id)`). This drives the SessionWorktreeInfo
construction in the codelet-sessions handle_impl.

---

## Dispatch_rpcNNN.rs file shape — pattern to mirror

```text
codelet/fspec-tui/src/app/dispatch_rpc056.rs:87:5:
  pub(crate) fn try_dispatch_rpc056(&mut self, action: &Action) -> bool
```

→ Confirms the `try_dispatch_rpcNNN(&mut self, action: &Action) -> bool`
shape. Each helper:
- returns `true` when it consumed the action,
- returns `false` when the action didn't match any of its arms.

The orchestrator's catch-all in `app/dispatch.rs` chains
`try_dispatch_rpc022 → try_dispatch_rpc053 → try_dispatch_rpc054 →
try_dispatch_rpc056` today; the work for RPC-057 extends this chain
with `try_dispatch_rpc057` so the orchestrator file stays under the
300-LoC ceiling — captured in the architecture note "/merge-worktree
slash command wiring lives in a new dispatch_rpc057.rs file".

---

## SessionManagerHandle / Stub conventions inspected

The blocklist arm of the trait (RPC-056, the closest peer to RPC-057)
shows the exact convention to mirror:

- Default-impl on the trait returns a safe empty value (so existing
  handles compile unchanged).
- `StubSessionManagerHandle` overrides the method, increments an
  `AtomicU64` per-call counter (`<method>_calls`), and returns a
  seedable in-memory snapshot (`Arc<Mutex<…>>`).
- The Stub exposes `<method>_calls() -> u64` and `seed_<noun>(value)`
  accessors so cross-transport parity tests can inject state and
  observe per-transport call counts.

→ This shape is what the five RPC-057 trait additions follow.

---

## FspecService / FspecBackend layering inspected

`codelet/rpc/src/lib.rs` declares the tarpc service with
`async fn blocklist_list() -> Vec<BlocklistRuleInfo>` and the impl
routes through `self.inner.session_manager()` with a safe default
when no handle is attached. RPC-057's five methods follow the same
two-layer pattern (service trait + service impl).

`codelet/fspec-tui/src/transport/{mod,embedded,websocket}.rs` declare
the `FspecBackend` trait method, its default `Ok(Vec::new())` (or
equivalent) impl, the embedded forwarder
(`self.client.<method>(context::current()).await?`), and the WebSocket
forwarder (`client.client().<method>(context::current()).await?`).

---

## TS reference parsed (handler shape we are porting)

`src/tui/handlers/mergeWorktreeHandler.ts` is the source-of-truth for
the slash command UX. Key observations:

1. The TS path skips a pre-merge confirm dialog entirely — it calls
   `inspectSessionChanges` then immediately `mergeSessionChanges`,
   showing a post-merge "press Enter to close" action prompt on
   success.
2. The Rust port introduces a pre-merge `MergeConfirmDialog` because
   the Rust frontend lacks the dual-state action-prompt pattern; this
   is a documented departure (see the architecture note).
3. On conflict, the TS calls `injectLlmContext(buildConflictLlmContext(error, worktreePath))`.
   The Rust port maps that to `Action::SeedPendingInput` (RPC-052 plumbing).
   The message format is preserved conceptually but the delivery
   mechanism uses the existing per-session draft seeding rather than
   the pendingAutoSubmitRef pattern.
4. The TS handler emits "Nothing to merge" when inspect returns zero
   total changes — preserved verbatim in the Rust dispatch.

---

## Compositor + dialog conventions inspected

`codelet/fspec-tui/src/views/agent/confirm_dialog.rs` (RPC-026 generic
confirm dialog) is the closest peer for the new MergeConfirmDialog.
Key conventions:

- Built on the shared `dialog_theme::render_dialog` renderer.
- `handle_key(KeyCode, KeyModifiers) -> SomeOutcomeEnum`.
- Tab / Right cycles focus forward; Left / Shift+Tab cycles backward.
- Enter activates the focused button.
- Esc returns the Cancel outcome regardless of focus.

→ MergeConfirmDialog reuses the renderer but is purpose-built because
it carries a `SessionChangesSummary` payload and emits the typed
`MergeConfirmDialogOutcome::{Merge,Discard,Cancel}` rather than the
generic dialog's Primary/Secondary/Cancel.

The compositor's `contains(&str)` / `remove(&str)` API uses a stable
string id — MergeConfirmDialog uses `"merge-confirm-dialog"` so the
push is idempotent on repeated /merge-worktree presses (matches the
HelpDialog + DisconnectDialog conventions).

---

## Conclusion

All five RPC methods, the wire types, the dialog component, and the
dispatch helper map cleanly onto established RPC-{054,055,056} patterns.
No code in the codelet-git crate, codelet-rpc-types crate, or transport
layer needs to be removed — every change is additive (new methods,
new wire types, new dispatch file, new dialog module) with the single
edit to `dispatch_rpc020.rs::handle_slash_command` to replace the
`MergeWorktree` notice arm with a real `self.handle_slash_merge_worktree()`
call.
