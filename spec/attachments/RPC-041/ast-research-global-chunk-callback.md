# RPC-041 AST Research — GLOBAL_CHUNK_CALLBACK call sites + sender-side helpers

Date: 2026-05-21

## 1. Every napi-side `GLOBAL_CHUNK_CALLBACK.get()` call site

Driven by `ast-grep --lang rust --pattern 'GLOBAL_CHUNK_CALLBACK.get()'` against
`codelet/napi/src/session_manager.rs`:

| File:Line:Col | Surrounding function | Purpose |
|---|---|---|
| codelet/napi/src/session_manager.rs:3005:20 | `FspecHandler` closure registered inside the agent_loop (line ~3001) | Early-return gate — bail out if no JS subscriber has registered. Returns `FspecHandlerResult { success: false, error: Some("Global chunk callback not registered - cannot execute fspec command"), ... }`. |
| codelet/napi/src/session_manager.rs:3285:20 | bridge `command_emitter` closure (line ~3283) | Sanity-check before emitting an `FspecCommandRequest` chunk via `session_for_command.handle_output(...)`. `tracing::warn!` + early return when missing. |
| codelet/napi/src/session_manager.rs:4152:46 | `spawn_footer_poller`'s emit site (inside the `if first_run || cwd_changed || ...` gate) | Emits a `StreamChunk::footer_state_update(...)` directly to the JS callback. |
| codelet/napi/src/session_manager.rs:4433:30 | `emit_block_notification_to_tui` (line ~4430) | Builds `StreamChunk::user_notification(...)` and calls `global_cb.call(session_id_str, chunk)`. |
| codelet/napi/src/session_manager.rs:6689:34 | `NapiSessionManagerHooks::emit_isolation_state_change` (lines 6683-6693) | Delegates to `GLOBAL_CHUNK_CALLBACK` from inside the SessionManagerHooks trait impl. The trait method itself is invoked from inside the moved SessionManager in `codelet-sessions` at lines ~641 and ~876 (create_session_with_id / create_isolated_session_with_id). |

## 2. The static + the struct + the unsafe impls

```text
codelet/napi/src/session_manager.rs:73:  static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback> = OnceCell::new();
codelet/napi/src/session_manager.rs:77:  struct GlobalChunkCallback { callback: ThreadsafeFunction<GlobalChunkCallbackArgs> }
codelet/napi/src/session_manager.rs:101: unsafe impl Send for GlobalChunkCallback {}
codelet/napi/src/session_manager.rs:102: unsafe impl Sync for GlobalChunkCallback {}
```

`#[napi(object)] pub struct GlobalChunkCallbackArgs { session_id, chunk }` at lines 82-87
**stays** — it is the TS-facing wire shape.

## 3. The session_set_global_chunk_callback napi free function

```text
codelet/napi/src/session_manager.rs:4290  pub fn session_set_global_chunk_callback(callback: ThreadsafeFunction<GlobalChunkCallbackArgs>) -> Result<()>
```

Current body:
1. Wraps callback in `GlobalChunkCallback::new(callback)` and stores in `GLOBAL_CHUNK_CALLBACK.set(...)`.
2. Calls `init_block_notification_callbacks()` (BLOCK-006).
3. Calls `install_napi_session_manager_hooks()` (RPC-040).
4. Calls `init_bridge_metadata_providers()` (BRIDGE-SESSION).
5. Calls `init_bridge_session_and_terminal_creators()` (SESS-017).

RPC-041 replaces (1) with: store TSFN in `CHUNK_FANOUT_TSFN: OnceCell<parking_lot::Mutex<Option<ThreadsafeFunction<GlobalChunkCallbackArgs>>>>` and `tokio::spawn` a long-running subscriber task on `SessionManager::instance().chunks_tx().subscribe()`. Steps 2-5 stay.

## 4. BackgroundSession::handle_output (the sender side)

Pattern: `pub fn handle_output(&self, chunk: StreamChunk) { $$$BODY }`

```text
codelet/sessions/src/background_session.rs:757:5
```

Current body (relevant excerpt at lines 757-801):
- Assigns correlation_id (already done).
- Tags pending observed_correlation_ids (already done).
- Pushes into `output_buffer` (already done).
- Calls `supervisor_broadcast.send(chunk.clone())` (already done).
- Conditional `if let Some(tx) = &self.chunks_tx { let _ = tx.send((SessionId::from(self.id.to_string()), chunk.clone())); }`.

RPC-041 strips the `Option` and the `if let Some(tx)` wrapper — `self.chunks_tx` becomes a mandatory `broadcast::Sender<...>` field.

## 5. BackgroundSession::set_status

Pattern: `pub fn set_status(&self, status: SessionStatus) { $$$BODY }`

```text
codelet/sessions/src/background_session.rs:732:5
```

Current body (lines 732-752):
1. `let old_status = self.status.swap(status as u8, Ordering::AcqRel);`
2. `if old_status != status as u8 { ... }` guard.
3. Map `SessionStatus` → `SessionState`, then `self.handle_output(StreamChunk::session_state_change(state))`.
4. `codelet_tools::broadcast_metadata_update()`.

RPC-041 inserts `let _ = self.status_changes_tx.send((SessionId::from(self.id.to_string()), status));` between (2) and (3).

## 6. SessionManagerHooks::emit_isolation_state_change

| File:Line | Item |
|---|---|
| codelet/sessions/src/session_manager.rs:122-127 | Trait method declaration |
| codelet/sessions/src/session_manager.rs:160-166 | `NoopSessionManagerHooks` no-op impl |
| codelet/sessions/src/session_manager.rs:641 | Call site in `create_session_with_id` |
| codelet/sessions/src/session_manager.rs:876 | Call site in `create_isolated_session_with_id` |
| codelet/napi/src/session_manager.rs:6683-6693 | `NapiSessionManagerHooks` impl (delegates to GLOBAL_CHUNK_CALLBACK) |

RPC-041 deletes all five of those locations and replaces the two call sites in `codelet-sessions` with direct `self.chunks_tx.send(...)` calls.

## 7. BackgroundSession::new call sites

Pattern: `BackgroundSession::new`

```text
codelet/sessions/src/session_manager.rs:557  (create_session_with_id)
codelet/sessions/src/session_manager.rs:797  (create_isolated_session_with_id)
codelet/sessions/src/background_session.rs:334+ (signature definition)
```

Both call sites must be widened to pass `self.chunks_tx.clone()` + `self.status_changes_tx.clone()`. No other construction sites exist (verified by exhaustive grep — every other `BackgroundSession::new` hit is in test helpers under `codelet/napi/tests/` that mock the type and are not the production constructor).

## 8. Existing tests that need updating

- `codelet/sessions/tests/background_session_shape.rs` lines 388-470 — the `scenario_handle_output_uses_the_new_chunks_tx_broadcast_and_no_longer_touches_global_chunk_callback` test currently REQUIRES `GLOBAL_CHUNK_CALLBACK` to still live in `codelet/napi/src/session_manager.rs`. Must be inverted.
- `codelet/sessions/tests/session_manager_shape.rs` lines 1001-1009 — `emit_isolation_state_change must delegate to GLOBAL_CHUNK_CALLBACK` — must be removed.
- `codelet/sessions/tests/session_manager_shape.rs` lines ~263-379 — forbidden token list still references `GLOBAL_CHUNK_CALLBACK` for the moved file (the moved file already doesn't reference it; the test stays green). The "every former GLOBAL_CHUNK_CALLBACK call resolves to `self.hooks.emit_isolation_state_change(...)`" step must be updated to "every former GLOBAL_CHUNK_CALLBACK call resolves to `self.chunks_tx.send(...)`".
- `codelet/napi/tests/global_chunk_callback_napi_test.rs` — the assertions in `handle_output_uses_global_callback` and the static-singleton checks need inversion (assert ABSENCE of GLOBAL_CHUNK_CALLBACK, not presence).

## 9. Downstream call-site discovery via `BackgroundSession::new`

Verified there is exactly one construction surface — `BackgroundSession::new` — in production code, with the two call sites inside `SessionManager` already listed. No mocks/tests in `codelet-sessions` call it, so widening the signature is safe and isolated.

## 10. Out of scope for RPC-041

- `SessionManagerHandle` trait implementation on `SessionManager` → RPC-042.
- Deletion of the rest of `codelet/napi/src/session_manager.rs` → RPC-043.
- Subscribers in `codelet-fspec-tui` → RPC-045.
- Removing the remaining `set_active_session` / `clear_active_session` napi singletons that the future fspec binary won't use → also RPC-042/043.
