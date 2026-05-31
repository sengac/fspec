# RPC-041 — Replace `GLOBAL_CHUNK_CALLBACK` with `tokio::broadcast` sender; rewire NAPI `ThreadsafeFunction` as a subscriber

**Parent:** RPC-030 · **Phase:** 4.4 · **Estimate:** 5 pts · **Depends on:** RPC-040

## Goal

Delete the `OnceCell<GlobalChunkCallback>` static and `unsafe impl Send/Sync for GlobalChunkCallback` from `codelet/napi/src/session_manager.rs`. Replace with a `tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>` owned by `SessionManager` (already added in RPC-040). The NAPI side subscribes once at startup and fans into the JS `ThreadsafeFunction`, preserving the existing `sessionSetGlobalChunkCallback` TS-facing API.

## Source — `codelet/napi/src/session_manager.rs` (current state pre-RPC-041)

| Line | Item |
|---|---|
| 58 | `static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback> = OnceCell::new();` |
| 60–64 | `struct GlobalChunkCallback { callback: ThreadsafeFunction<GlobalChunkCallbackArgs> }` |
| 67–72 | `#[napi(object)] pub struct GlobalChunkCallbackArgs { session_id, chunk }` |
| 74–83 | `impl GlobalChunkCallback { fn new(...), fn call(...) }` |
| 86–87 | `unsafe impl Send for GlobalChunkCallback {}` / `Sync` |
| 6331 | `#[napi] fn session_set_global_chunk_callback(callback: JsFunction) -> Result<()>` |

The `BackgroundSession::handle_output` method (line 931 in original layout) currently does:

```rust
if let Some(cb) = GLOBAL_CHUNK_CALLBACK.get() {
    cb.call(session_id, chunk.clone());
}
```

## Work to do

### Step 1 — In `codelet/sessions/src/session_manager.rs`

`SessionManager::new()` constructs `chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>` (already added field in RPC-040). Wire it into every `BackgroundSession` via constructor injection:

```rust
let session = BackgroundSession::new(
    inner_session,
    session_id,
    self.chunks_tx.clone(),
    self.logs_tx.clone(),
    self.status_changes_tx.clone(),
    /* ... */
);
```

### Step 2 — In `codelet/sessions/src/background_session.rs`

`BackgroundSession` gains fields:

```rust
chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>,
```

`handle_output` becomes:

```rust
pub fn handle_output(&self, chunk: StreamChunk) {
    // 1. assign correlation id (existing)
    // 2. apply pending observed_correlation_ids (existing)
    // 3. buffer chunk (existing)
    // 4. broadcast to supervisors (existing)
    // 5. broadcast on chunks_tx (NEW — replaces GLOBAL_CHUNK_CALLBACK.call)
    let _ = self.chunks_tx.send((SessionId::from(self.id), chunk));
}
```

`set_status` (line 906 in original) emits on `status_changes_tx`:

```rust
pub fn set_status(&self, status: SessionStatus) {
    self.status.store(status.as_u8(), Ordering::SeqCst);
    let _ = self.status_changes_tx.send((SessionId::from(self.id), status));
    // existing: also emit StreamChunk::SessionStateChange via chunks_tx
}
```

### Step 3 — In `codelet/napi/src/session_bindings.rs` (or wherever `session_set_global_chunk_callback` ends up)

Replace the callback storage with a subscriber task spawned at NAPI startup:

```rust
static CHUNK_FANOUT: OnceCell<Mutex<Option<ThreadsafeFunction<GlobalChunkCallbackArgs>>>> = OnceCell::new();

#[napi]
pub fn session_set_global_chunk_callback(callback: JsFunction) -> Result<()> {
    let tsfn: ThreadsafeFunction<GlobalChunkCallbackArgs> =
        callback.create_threadsafe_function(0, |ctx| {
            Ok(vec![ctx.value])
        })?;

    let manager = codelet_sessions::SessionManager::instance();
    let mut rx = manager.chunks_tx().subscribe();

    // Store handle for replacement
    CHUNK_FANOUT.get_or_init(|| Mutex::new(None));
    *CHUNK_FANOUT.get().unwrap().lock() = Some(tsfn.clone());

    tokio::spawn(async move {
        while let Ok((session_id, chunk)) = rx.recv().await {
            if let Some(tsfn) = CHUNK_FANOUT.get().and_then(|m| m.lock().clone()) {
                let args = GlobalChunkCallbackArgs {
                    session_id: session_id.to_string(),
                    chunk: serde_json::to_string(&chunk).unwrap_or_default(),
                };
                tsfn.call(Ok(args), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }
    });

    Ok(())
}
```

### Step 4 — Delete dead code

Remove from `codelet/napi/src/session_manager.rs`:
- Line 58: `GLOBAL_CHUNK_CALLBACK` static
- Lines 60–83: `GlobalChunkCallback` struct + impl
- Lines 86–87: `unsafe impl Send/Sync`

`GlobalChunkCallbackArgs` (lines 67–72) stays — it's the TS-facing wire shape.

## Acceptance criteria

1. `GLOBAL_CHUNK_CALLBACK` static is deleted.
2. `unsafe impl Send/Sync for GlobalChunkCallback` is deleted.
3. `BackgroundSession::handle_output` uses `chunks_tx.send` instead of `GLOBAL_CHUNK_CALLBACK.call`.
4. `BackgroundSession::set_status` emits `(SessionId, SessionStatus)` on `status_changes_tx`.
5. The TS-facing API `sessionSetGlobalChunkCallback(callback)` still works — verify by running the TS frontend against a session and confirming chunks reach the JS callback.
6. `cargo build -p codelet-napi` + `-p codelet-sessions` pass.
7. `cargo test -p codelet-napi` passes — TS-side regression tests in `codelet/napi/tests/` see no behaviour change.
8. Multiple subscribers: spawn 2 tokio tasks that `chunks_tx.subscribe()` and confirm both receive every chunk (NAPI fan-out + Rust frontend).

## Risks

- `ThreadsafeFunction` from `napi-rs` must be cloneable and survive multiple subscribers. Confirm by reading `napi-rs` docs — `ThreadsafeFunction` is `Clone`.
- Re-registering the JS callback (calling `session_set_global_chunk_callback` twice) must replace, not duplicate. The `CHUNK_FANOUT` `Mutex<Option<...>>` pattern handles this.
- Broadcast channel capacity: `DEFAULT_CHUNKS_CAPACITY = 1024` (from `codelet/rpc/src/lib.rs`). For high-volume sessions, this may need tuning. Confirm `lag_chunks` metric in `SharedFspecService` doesn't spike.
- Test ordering: the TS frontend may rely on synchronous-looking callback ordering. Tokio broadcast guarantees per-sender ordering — fine for single-sender (which is the case: `BackgroundSession::handle_output` is the sole writer).

## Out of scope

- Implementing `SessionManagerHandle` on `SessionManager` → RPC-042.
- Reducing NAPI to a thin adapter (delete the rest of `session_manager.rs`) → RPC-043.
