# AST Research for RPC-049 — `/resume` durable restore

**Date:** 2026-05-22
**Card:** RPC-049
**Scope:** Locate every site that must be touched to wire a new `resume_session(SessionId) -> Result<(), String>` aggregate RPC that performs the durable-restore round-trip (`load_session` → `get_session_message_envelopes` → `restore_session_messages` → `restore_session_token_state`).

---

## 1. Existing trait surface (RPC-037 widening — `restore_session_messages` + `restore_session_token_state` already present)

AST query: `fn restore_session_messages(&self, $$$ARGS) -> Result<(), String> { $$$BODY }` over `codelet/core/src/session_manager_handle.rs`:

| Hit | Line | Notes |
|---|---|---|
| `SessionManagerHandle` trait default | 278 | Default returns `Ok(())`; takes `Vec<String>` envelopes. |
| `StubSessionManagerHandle` impl | 808 | Override returns `Ok(())` deterministically. |

Same pattern for `restore_session_token_state` (line 288 / 816), `clear_history` (259 / 782), `compact_session` (266 / 793). All five RPC-049 prerequisites are already in place.

**Production impl** in `codelet/sessions/src/handle_impl.rs:245-267`: minimal placeholders that look up the session via `self.get_session` and return `Ok(())` if found. They do NOT actually feed envelopes/state into the BackgroundSession yet — the full port of the NAPI `session_restore_messages` body (~170 LoC, `codelet/napi/src/session_bindings.rs:2392`) is deferred to a later card (likely RPC-068 verification + targeted follow-ups). RPC-049 scope: add the `resume_session` aggregate that wires the round-trip; production-side envelope-to-chunk translation stays at parity with the current placeholders. The aggregate works end-to-end against the `MockBackend` test fixture by exercising the action-bus + chunk-replay path.

---

## 2. RPC service surface

`codelet/rpc/src/lib.rs`:

| Line | Symbol | Role |
|---|---|---|
| 229 | `async fn clear_history(session_id) -> Result<(), String>` | trait sibling pattern |
| 232 | `async fn compact_session(session_id) -> Result<CompactionResult, String>` | trait sibling pattern |
| 235 | `async fn restore_session_messages(session_id, envelopes) -> Result<(), String>` | already wired (RPC-037) |
| 241 | `async fn restore_session_token_state(session_id, state) -> Result<(), String>` | already wired (RPC-037) |
| 1022 | `FspecServiceImpl::restore_session_messages` | `self.inner.session_manager()? ... .restore_session_messages(...)` shape |
| 1034 | `FspecServiceImpl::restore_session_token_state` | mirror shape |

**Insertion target:** add `async fn resume_session(session_id: SessionId) -> Result<(), String>` to the `FspecService` tarpc trait alongside line 244, and add `FspecServiceImpl::resume_session` alongside line 1044 — same delegate shape (`Some(handle) => handle.resume_session(&session_id) / None => Ok(())`).

---

## 3. FspecBackend trait + EmbeddedFspecBackend + WebSocketFspecBackend

`codelet/fspec-tui/src/transport/mod.rs`:
- Trait default body for `restore_session_messages` at line 306 — pattern: `async fn ... { Ok(()) }`.
- Insertion target: add `async fn resume_session(&self, session_id: SessionId) -> Result<()>` next to line 321 with default `Ok(())` so test mocks can override only what they need.

`codelet/fspec-tui/src/transport/embedded.rs:315`:
- `restore_session_messages` one-line delegate: `self.client.restore_session_messages(context::current(), session_id, envelopes).await?.map_err(|e| anyhow::anyhow!("{e}"))`.
- Insertion target: add `async fn resume_session(&self, session_id: SessionId) -> Result<()>` after `restore_session_token_state` (line 326) with the same shape.

`codelet/fspec-tui/src/transport/websocket.rs:595`:
- Same pattern with `let guard = self.client.read().await; let client = guard.as_ref().ok_or(BackendError::Disconnected)?;` and a one-line delegate. Insertion target: mirror after line 618.

---

## 4. Dispatch wiring — App::dispatch + dispatch_rpc026

`codelet/fspec-tui/src/app/dispatch_rpc026.rs:56` — `handle_attach_to_session(&mut self, session: SessionId)` currently:
1. Drops `resume_view`.
2. Moves current_session_index OR appends new SessionContext.
3. Publishes to `active_session_tx`.
4. Calls `refresh_session_chrome(session)`.

**Modification:** after step (4), spawn a tokio task awaiting `backend.resume_session(session_id)`; on Ok dispatch `Action::SessionResumeComplete(id)`, on Err dispatch `Action::EmitSessionNotice(id, format!("[error] /resume failed: {e}"))`.

**New helper:** `pub(crate) fn handle_session_resume_complete(&mut self, id: SessionId)` — spawns a second tokio task that calls `backend.get_buffered_output(id, 1000).await.unwrap_or_default()` and for each chunk dispatches `Action::ChunkReceived(id, chunk)` via `self.action_tx.send(...)`.

`codelet/fspec-tui/src/app/dispatch.rs:289` — currently the last RPC-026 arm is `Action::ConfirmDeleteSession`. Add one new arm: `Action::SessionResumeComplete(id) => self.handle_session_resume_complete(id.clone())` next to the existing `AttachToSession` arm (line 284).

LoC analysis:
- `dispatch.rs` currently 299 LoC — adding one new match arm = ~1 LoC; stays under 300.
- `dispatch_rpc026.rs` currently 163 LoC — adding the helper + tokio spawn = ~30 LoC; final ~193 LoC, well under 300.

---

## 5. Action enum

`codelet/fspec-tui/src/components/mod.rs:97` — `pub enum Action`. Latest variant in source order is `EmitSessionNotice` (line 400). Insertion target: add `SessionResumeComplete(codelet_rpc_types::SessionId)` next to it. No change to derive list — `SessionId` already implements `Debug + Clone`.

---

## 6. Lift target — `get_session_message_envelopes`

AST query: `pub fn persistence_get_session_message_envelopes(...) -> Result<Vec<String>> { $$$BODY }` over `codelet/napi/src/persistence/napi_bindings.rs`:
- Hit at line 729. The body uses:
  - `load_session(uuid)` — already lives in `codelet_core::persistence::manifest::load_session` (line 582).
  - `get_session_messages(&session)` — already lives in `codelet_core::persistence::manifest::get_session_messages` (line 902).
  - `rehydrate_envelope_blobs(...)` — already lives in `codelet_core::persistence::blob_processing`.
  - Synthetic-compaction-summary fast path for `Uuid::nil()` entries.
  - All `napi::Error::from_reason` wrappings.

**Lift action:** port the body into a new free function `pub fn get_session_message_envelopes(uuid: Uuid) -> Result<Vec<String>, String>` in `codelet/core/src/persistence/manifest.rs` near `get_session_messages` (insert after line 956 `get_session_messages_full`). The NAPI binding becomes a one-line delegate `codelet_core::persistence::get_session_message_envelopes(uuid).map_err(Error::from_reason)`.

Sibling functions to lift in the same pass (parity with the NAPI binding's three companions) — DEFERRED if not needed by RPC-049. Confirmed: RPC-049 only requires the rehydrated/compaction-aware variant (`get_session_message_envelopes`). The `_full` / `_raw` / `_raw_full` siblings (lines 777 / 803 / 846) can stay in the NAPI layer until a future card needs them server-side.

---

## 7. MockBackend extensions

`codelet/fspec-tui/tests/common/mod.rs`:
- Existing pattern (RPC-046 `set_clear_history_error` at line 674; RPC-047 `set_compact_session_result_ok` at line 693).
- Insertion targets:
  - Fields on `MockBackend` struct: `resume_session_calls: AtomicU64`, `last_resume_session: Mutex<Option<SessionId>>`, `resume_session_error: Mutex<Option<String>>`, `buffered_output: Mutex<Vec<StreamChunk>>`.
  - Accessor helpers: `resume_session_calls() -> usize`, `last_resume_session() -> Option<SessionId>`, `set_resume_session_error(msg)`, `set_buffered_output(chunks)`, `get_buffered_output_calls() -> usize`.
  - Async impl: `async fn resume_session(&self, id: SessionId) -> Result<()>` honouring the scripted error; `async fn get_buffered_output(&self, id: SessionId, limit: u32) -> Result<Vec<StreamChunk>>` returning the scripted chunks.

---

## 8. Cross-transport parity test surface

`codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs` (already exists at ~51 kB) already exercises every RPC-037 method against both transports. RPC-049 adds one scenario at the bottom of that file (or a new dedicated `tests/rpc049_resume_session.rs`):
- Build `SharedFspecService` with `Arc::new(StubSessionManagerHandle::new())`.
- Construct an `EmbeddedFspecBackend` and a `WebSocketFspecBackend` over the same service.
- Call `backend.resume_session(SessionId::new("stub-1"))` on each; assert both `Ok(())`.

The stub's `resume_session` will use the default trait body, which calls `restore_session_messages` + `restore_session_token_state` (both already deterministic on the stub). No counter is needed if the assertion is just on the return type — but the attachment recommends a call counter so the stub gets a new `AtomicU64 resume_session_calls` and a `resume_session_calls()` accessor.

---

## 9. RPC-002 source-shape invariants to preserve

- No new file > 300 LoC.
- `codelet/fspec-tui/src/app/dispatch.rs` < 300 LoC (currently 299; the +1 arm keeps it at 300 — refactor risk; if it would push over, move the dispatch dispatch to `try_dispatch_rpc022` fallback OR factor into a tiny new file).
- No `codelet_napi` reference anywhere under `codelet/fspec-tui/src/`. The lift in §6 actually REMOVES NAPI from the data path entirely — production resumes can now go straight through `codelet_core::persistence`.

---

## Summary of files touched

| Layer | File | Action |
|---|---|---|
| Core trait | `codelet/core/src/session_manager_handle.rs` | Add `resume_session` trait method + StubSessionManagerHandle override + counter |
| Core persistence | `codelet/core/src/persistence/manifest.rs` | Lift `get_session_message_envelopes(uuid) -> Result<Vec<String>, String>` |
| Core persistence re-export | `codelet/core/src/persistence/mod.rs` | Already wildcards from `manifest::*` — no change needed |
| NAPI binding | `codelet/napi/src/persistence/napi_bindings.rs` | Reduce `persistence_get_session_message_envelopes` to a thin delegate |
| Sessions impl | `codelet/sessions/src/handle_impl.rs` | (Optional) override `resume_session` if the default doesn't suffice for the production path |
| RPC service | `codelet/rpc/src/lib.rs` | Add `FspecService::resume_session` + impl |
| Backend trait | `codelet/fspec-tui/src/transport/mod.rs` | Add `resume_session` default-Ok method |
| Embedded backend | `codelet/fspec-tui/src/transport/embedded.rs` | One-line delegate |
| WebSocket backend | `codelet/fspec-tui/src/transport/websocket.rs` | Disconnected-guarded delegate |
| Action enum | `codelet/fspec-tui/src/components/mod.rs` | Add `SessionResumeComplete(SessionId)` variant |
| Dispatch routing | `codelet/fspec-tui/src/app/dispatch.rs` | Add `Action::SessionResumeComplete` arm |
| Dispatch helper | `codelet/fspec-tui/src/app/dispatch_rpc026.rs` | Extend `handle_attach_to_session` + add `handle_session_resume_complete` |
| MockBackend | `codelet/fspec-tui/tests/common/mod.rs` | Counters + accessors + impl |
| Integration test | `codelet/fspec-tui/tests/slash_resume_rpc049.rs` (new) | Test scenarios 1-4 + 7 from the feature file |
| Parity test | `codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs` (extend) | Scenario 5 (parity for resume_session) |
| Core unit test | `codelet/core/src/persistence/tests.rs` (extend) | Scenario 6 (get_session_message_envelopes lift) |
