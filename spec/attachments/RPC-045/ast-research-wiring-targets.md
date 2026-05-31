# RPC-045 — AST research: wiring targets for chunks + status broadcast consumption

This research locates every Rust source location the implementation must touch
to consume the existing `chunks_rx` + `status_changes_rx` broadcasts in
`App::run` and route the 5 new `StreamChunk` variants into per-session store
state. Recorded BEFORE writing tests/code per RPC-045 rules.

## 1. App::run's `tokio::select!` loop — single match arm to extend

AST query (pattern `tokio::select! { $$$BRANCHES }`, scope
`codelet/fspec-tui/src/app/`):

| File | Line | Notes |
|------|------|-------|
| `codelet/fspec-tui/src/app/events.rs` | 165 | The only `tokio::select!` in `App::run`. Currently selects over `events.next()`, `action_rx.recv()`, and `tick.tick()`. RPC-045's chunks/status subscribers ALREADY live in `bootstrap.rs::spawn_subscriber_tasks` (separate tokio tasks pushing onto `action_tx`), so we do NOT need to extend this select arm — we extend the subscriber loops instead. The attachment's illustrative `chunks_rx.recv()` + `status_rx.recv()` arms are equivalently realised by the existing subscriber tasks. |

Decision: extend `bootstrap.rs::spawn_subscriber_tasks` (the per-rx tokio task
pattern is already established for `work_units_rx`, `chunks_rx`, `logs_rx`).
ADD a fourth task for `status_changes_rx`. MUTATE the existing `chunks_rx`
task to drop its active-session filter.

## 2. Existing subscriber-task pattern in bootstrap

AST query (pattern `tokio::spawn(async move { $$$BODY })`, scope
`codelet/fspec-tui/src/app/bootstrap.rs`):

| File | Line | Subscriber |
|------|------|------------|
| `codelet/fspec-tui/src/app/bootstrap.rs` | 64 | work_units_rx → `Action::WorkUnitsLoaded` |
| `codelet/fspec-tui/src/app/bootstrap.rs` | 86 | **chunks_rx → `Action::ChunkReceived` (currently filtered by `active_session_rx.borrow()`)** |
| `codelet/fspec-tui/src/app/bootstrap.rs` | 108 | logs_rx → `tracing::debug` |

RPC-045 edits:

- Drop the `let active = active_rx.borrow().clone();` guard inside the
  `chunks_rx.recv()` arm so every `(SessionId, StreamChunk)` is forwarded
  regardless of focus.
- Add a fourth `tokio::spawn` for `status_changes_rx` → `Action::SessionStatusChanged`.

## 3. FspecBackend trait methods already present

AST query (pattern `fn status_changes_rx(&self) -> $RET { $$$BODY }`, scope
`codelet/fspec-tui/src/transport`):

| File | Line | Impl |
|------|------|------|
| `codelet/fspec-tui/src/transport/mod.rs` | 467 | Default (closed receiver — graceful no-op) |
| `codelet/fspec-tui/src/transport/embedded.rs` | 513 | Overrides — forwards `SharedFspecService::status_changes_rx` |
| `codelet/fspec-tui/src/transport/websocket.rs` | 269 | Overrides — wires the WS-side broadcast bridge |

Conclusion: NO trait change required; the surface already exists. RPC-045 only
consumes it.

The runner also depends on `FspecBackend::send_fspec_result` (line ~438 of
`transport/mod.rs`, default `Ok(())`) — overridden in both real transports.
Likewise no trait change required.

## 4. AgentViewStore — host for new per-session HashMaps

AST query (pattern `impl AgentViewStore { $$$ITEMS }`):

| File | Line | Hosts |
|------|------|-------|
| `codelet/fspec-tui/src/store/agent_view.rs` | 100 | Core multi-session container + nav + chrome (workspace) |
| `codelet/fspec-tui/src/store/agent_view/chrome_state.rs` | 19 | `model_info_for / set_model_info / thinking_level_for / set_thinking_level / token_state_for / set_token_state / apply_chunk_to_token_state / workspace / set_workspace` |
| `codelet/fspec-tui/src/store/agent_view/role_state.rs` | 18 | `role_for / set_role` |

RPC-045 edits:

- Add three private fields to the struct definition in `agent_view.rs`:
  `session_status_by_session: HashMap<SessionId, SessionStatus>`,
  `isolation_state_by_session: HashMap<SessionId, IsolationState>`,
  `debug_enabled_by_session: HashMap<SessionId, bool>`. The current file is
  280 LoC and capped at 300 by the source-shape invariant
  `agent_view_store_stays_under_300_loc_with_history_fields` — adding 3 field
  lines (~5 LoC) keeps it safely under the ceiling.
- Add accessor `impl` block in a new sibling
  `store/agent_view/isolation_state.rs` (mirroring chrome/role pattern) for
  the new state. The new `IsolationState` struct lives in the same file
  alongside its accessors. Keeps every existing sub-module under 300 LoC.
- Re-export `IsolationState` from `store/agent_view.rs` (pub use sibling).

## 5. StreamChunk variants — what's already present

The `IsolationStateChange` etc. variants ALREADY exist in `codelet-rpc-types`
(verified by reading `codelet/rpc-types/src/lib.rs:702..813` directly; AST
pattern `IsolationStateChange { $$$FIELDS }` did not match because variant
declarations are not standalone AST nodes in tree-sitter-rust — they are part
of the enum-variant list). Variants confirmed present:

- `StreamChunk::SessionStateChange { state: SessionState }` (line 741)
- `StreamChunk::IsolationStateChange { is_isolated, worktree_path, base_commit }` (line 790)
- `StreamChunk::DebugStateChange { enabled }` (line 810)
- `StreamChunk::FooterStateUpdate { cwd, display_path, is_git_repo, branch }` (line 802)
- `StreamChunk::FspecCommandRequest { fspec_request: FspecRequest }` (line 778)

`SessionStatus` enum is at `codelet/rpc-types/src/lib.rs:134` with variants
`Idle, Running, Interrupted, Paused, Compacting, Cleared`.

`SessionState` → `SessionStatus` mapping: both have `Idle / Running /
Interrupted / Paused / Compacting / Cleared`, identical variant order. A
small `From<SessionState> for SessionStatus` helper is the cleanest bridge
for the `SessionStateChange` → `set_session_status` path.

## 6. Action enum — additions

`Action::ChunkReceived(SessionId, StreamChunk)` already exists
(`codelet/fspec-tui/src/components/mod.rs:119`). RPC-045 ADDS:

- `Action::SessionStatusChanged(SessionId, SessionStatus)` — emitted by the
  new status_changes_rx subscriber and dispatched in App::dispatch.

The existing `ChunkReceived` arm in `app/dispatch.rs:29` is extended (or
delegated) to also branch on the new variants.

## 7. Per-card dispatch helper file (300-LoC rule)

Existing precedent for split: `app/dispatch_rpc018.rs`,
`app/dispatch_rpc020.rs`, `app/dispatch_rpc022.rs`, `app/dispatch_rpc024.rs`,
`app/dispatch_rpc025.rs`, `app/dispatch_rpc026.rs` all extend `impl App`
with `pub(crate) fn` helpers invoked from `app/dispatch.rs`'s catch-all arm
via `try_dispatch_rpcNNN(&action) -> bool`.

RPC-045 creates `app/dispatch_rpc045.rs` with:

- `pub(crate) fn handle_stream_chunk_state_updates(&mut self, session_id, chunk)`
  — invoked from the existing `Action::ChunkReceived` arm AFTER
  `record_chunk` + `apply_chunk_to_token_state`. Branches on the new chunk
  variants and writes the new store state.
- `pub(crate) fn handle_session_status_changed(&mut self, session_id, status)`
  — invoked from a new `Action::SessionStatusChanged` arm.
- `pub(crate) fn spawn_fspec_command_runner(&mut self, session_id, request)`
  — `tokio::spawn` task that executes the limited command set and routes
  the result via `backend.send_fspec_result`.

## 8. Existing chunk-routing tests (regression risk)

Files that drive `Action::ChunkReceived` through `App::dispatch`:

- `tests/app_dispatch_rpc024.rs` (lines 30-150): asserts background chunks
  accumulate into the correct SessionContext — RPC-045 KEEPS THIS BEHAVIOUR.
  No regression expected, and we strengthen it by also asserting the
  per-session status/isolation/debug HashMaps update.

## 9. FspecResult shape

`codelet/rpc-types/src/lib.rs:465-475`:

```rust
pub struct FspecResult {
    pub success: bool,
    pub data: String,
    pub error: Option<String>,
    pub system_reminder: Option<String>,
    pub tool_call_id: String,
}
```

Note `data` is `String` (not `serde_json::Value`). The runner must
`serde_json::to_string(&list_of_units)` and assign to `data`. For
`unsupported command: <name>`, `data` is `""`.

## 10. Test plumbing — MockBackend

`tests/common/mod.rs` already exposes:

- `MockBackend::seed_work_units` — for the list-work-units runner happy path.
- `MockBackend::chunks_tx` (via `push_chunk`) — for driving chunks through
  the App's subscriber.
- A method `send_fspec_result` MUST be added (mirror existing trait method).
  We add it to MockBackend with an internal `last_fspec_result: Mutex<Option<FspecResult>>`
  and a `last_fspec_result()` accessor for the test to assert.
- A method `status_changes_tx` MUST be exposed so tests can push synthetic
  status broadcasts. Currently the trait default returns a closed receiver
  — MockBackend must override `status_changes_rx` and a sibling
  `push_status_change()` helper.

## 11. Source-shape invariants to honour

- `tests/source_shape_rpc024.rs::agent_view_store_stays_under_300_loc_with_history_fields`
  — `store/agent_view.rs` must stay ≤ 300 LoC. Current 280 LoC; add ≤ 20.
- `tests/source_shape_rpc025.rs` — `app/dispatch.rs` ≤ 300 LoC. Currently
  295 LoC. We can ONLY add 1 new match arm
  (`Action::SessionStatusChanged`) which costs ~3 LoC; all heavy logic goes
  into `app/dispatch_rpc045.rs`.
- No equivalent invariant exists for `bootstrap.rs` (123 LoC currently).
  Adding the 4th subscriber adds ~20 LoC.

## 12. Wiring summary (commit-checklist)

1. `codelet/rpc-types` — nothing (all wire shapes already present).
2. `codelet/fspec-tui/src/components/mod.rs` — add `Action::SessionStatusChanged(SessionId, SessionStatus)` variant + docs.
3. `codelet/fspec-tui/src/store/agent_view.rs` — add 3 HashMap fields + re-export `IsolationState`.
4. `codelet/fspec-tui/src/store/agent_view/isolation_state.rs` — NEW file: `IsolationState` struct + accessors for session_status, isolation_state, debug_enabled.
5. `codelet/fspec-tui/src/app/dispatch.rs` — extend `Action::ChunkReceived` arm to call `self.handle_stream_chunk_state_updates(...)` and add `Action::SessionStatusChanged` arm calling `self.handle_session_status_changed(...)`.
6. `codelet/fspec-tui/src/app/dispatch_rpc045.rs` — NEW file: the three helpers (incl. FspecCommandRequest runner).
7. `codelet/fspec-tui/src/app/mod.rs` — register new sub-module.
8. `codelet/fspec-tui/src/app/bootstrap.rs` — drop active-session filter on `chunks_rx` task; add `status_changes_rx` 4th task.
9. `codelet/fspec-tui/tests/common/mod.rs` — extend MockBackend with `send_fspec_result` override + `last_fspec_result` accessor + `status_changes_tx` override + `push_status_change` helper.
10. `codelet/fspec-tui/tests/agent_view_chunk_dispatch_rpc045.rs` — NEW integration test driving all 8 scenarios from the feature file.

End of research.
