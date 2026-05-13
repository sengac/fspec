# RPC-006 AST Research — Watcher lift + WorkUnitsUpdate envelope

Source-shape findings collected via AstGrep + Grep + Read against the
RPC-005 artifacts that RPC-006 must extend in place.

## 1. Existing watcher in codelet/napi

File: `codelet/napi/src/work_units_watcher.rs`

- Uses `notify::{RecommendedWatcher, RecursiveMode}` and
  `notify_debouncer_mini::{new_debouncer, DebouncedEventKind}`.
- Has a global `lazy_static!` `WATCHER_STATE: Arc<RwLock<WatcherState>>`
  and `WATCHER_HANDLE: Arc<RwLock<Option<Debouncer<RecommendedWatcher>>>>`.
- `start_work_units_watcher(project_root, ThreadsafeFunction<StreamChunk>)`
  resolves `<root>/spec/work-units.json`, loads via `serde_json`, fires
  the TS callback with a `StreamChunk::work_units_update(Vec<WorkUnitInfo>)`.
- Filter prevents the proper-lockfile feedback loop: only fires when an
  event's `path.file_name() == "work-units.json"` (lines 138–141).
- Module also exports `stop_work_units_watcher`, `get_work_unit_status`,
  `get_work_unit`, `get_all_work_units`, `is_work_units_watcher_active`.

The notify+debouncer logic is portable; the only NAPI-specific surface
is the `ThreadsafeFunction<StreamChunk>` callback. The lift target —
`codelet/core/src/work_units.rs` — replaces the callback with
`tokio::sync::broadcast::Sender<Vec<WorkUnitInfo>>`.

## 2. RPC-005 artifacts to mutate

### codelet/rpc/src/lib.rs
- `pub struct SharedFspecService { fixture: Vec<WorkUnitInfo>, list_work_units_calls: Arc<AtomicU64> }`
- `SharedFspecService::new(fixture: Vec<WorkUnitInfo>) -> Self` (line 43)
- `SharedFspecService::fixture(&self) -> Vec<WorkUnitInfo>` increments counter (line 51)
- `FspecServiceImpl { pub inner: Arc<SharedFspecService> }` cloneable (line 67)
- `impl FspecService for FspecServiceImpl::list_work_units` body =
  `self.inner.fixture()` (line 79–81)
- `pub fn default_fixture() -> Vec<WorkUnitInfo>` returns 2 hard-coded
  AUTH-001/AUTH-002 records (line 93)
- Crate has ZERO codelet-core dep currently (Cargo.toml lines 11–14)

### codelet/rpc-server/src/envelope.rs
- `Envelope` enum: `Rpc(Vec<u8>) | Event | LogEvent | WorkUnitsUpdate | CmdReq | CmdRes`
  (lines 26–41) — all five non-Rpc variants are unit variants today.
- `Envelope::variant_name()` returns `"WorkUnitsUpdate"` (line 51).

### codelet/rpc-server/src/lib.rs
- `pub use codelet_rpc::default_fixture;` (line 29) — re-export propagates
  to main.rs.
- `ServerStats { service, rejected_envelopes, rejected_variants }` —
  reserved-variants log is `Vec<&'static str>` (line 64).

### codelet/rpc-server/src/server.rs
- `bind_and_serve(addr, service: Arc<SharedFspecService>) -> (SocketAddr, ServerStats, JoinHandle)`
- `handle_connection(stream, service, stats)` constructs:
  - `rpc_bytes_tx/rx` mpsc<Vec<u8>> — incoming Rpc payloads
  - `server_out_tx/rx` mpsc<Vec<u8>> — outgoing tarpc bytes
  - `ChannelTransport::new(rpc_bytes_rx, server_out_tx)`
  - `BaseChannel::with_defaults(transport).execute(service_impl.serve())`
  - `run_envelope_pump(ws_sink, ws_stream, rpc_bytes_tx, server_out_rx, ServerInbound { stats })`
- `tokio::select!` between `serve_fut` and `pump_fut` (lines 94–97).

### codelet/rpc-server/src/pump.rs
- `run_envelope_pump<S, H: InboundHandler>` is the shared select-loop.
- Outbound loop wraps `Vec<u8>` in `Envelope::Rpc(bytes)` and bincode-
  serialises for `Message::Binary` (lines 81–86).
- Inbound match at lines 99–108: `Envelope::Rpc(inner) => rpc_bytes_tx.send(inner)`,
  `other => inbound.on_reserved(other.variant_name())`.
- `ServerInbound::on_reserved` increments `rejected_envelopes`, pushes
  variant name into `rejected_variants`, emits `tracing::warn!`.
- `ClientInbound::on_reserved` is debug-only.

### codelet/rpc-server/src/client.rs
- `pub async fn ws_client_connect<S>(ws: WebSocketStream<S>) -> anyhow::Result<FspecServiceClient>`
- Today returns a bare `FspecServiceClient`. RPC-006 must change this to
  a struct exposing both the rpc client AND a broadcast receiver.
- Constructs `rpc_in_tx/rx`, `rpc_out_tx/rx` mpsc channels and spawns
  `run_envelope_pump(sink, stream, rpc_in_tx, rpc_out_rx, ClientInbound)`.

### codelet/rpc-server/src/main.rs
- `let service = Arc::new(SharedFspecService::new(default_fixture()));`
  (line 28) — must become `WorkUnitsWatcher::new(workspace)?`.
- Bind addr literal `"127.0.0.1:0"` (line 29) — must remain.
- Stdout contract: `println!("{}", addr.port())` then flush (line 34).

### codelet/rpc-embedded/src/lib.rs
- `EmbeddedTransport { handle, service: Arc<SharedFspecService> }`
- `pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self`
  (line 42) — signature unchanged in RPC-006.
- `pub fn client(&self) -> FspecServiceClient` spawns server task on
  `self.handle.spawn(...)` (line 58). RPC-006 adds a sibling
  `pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>`.

## 3. RPC-005 source-shape regression tests

File: `codelet/rpc-embedded/tests/architecture_invariants.rs`

| Scenario | Action in RPC-006 |
|---|---|
| 7 EmbeddedTransport requires Handle | unchanged, must stay green |
| 8 WorkUnitInfo defined once in rpc-types | unchanged, must stay green |
| 9 Embedded uses only tarpc::transport::channel | EXTEND — embedded must still NOT contain `bincode::serialize` on the read path; the new `work_units_rx()` returns a raw broadcast receiver |
| 10 codelet-rpc has no codelet-core or codelet-napi dep | WIDEN — now permits codelet-core, still forbids codelet-napi |
| 11 rpc-server binds 127.0.0.1 only | unchanged, must stay green |

Reserved-variants test
(`codelet/rpc-server/tests/websocket_transport.rs::scenario_6_*`):
- Today the reserved list is `[Event, LogEvent, WorkUnitsUpdate, CmdReq, CmdRes]` (5 variants).
- RPC-006 narrows to `[Event, LogEvent, CmdReq, CmdRes]` (4 variants);
  `WorkUnitsUpdate` is now legitimate.

## 4. NAPI shim surface

`codelet/napi/index.d.ts` exports:
```ts
export declare function startWorkUnitsWatcher(
  projectRoot: string,
  callback: (chunk: import('./index').StreamChunk) => void,
): void;
export declare function stopWorkUnitsWatcher(): void;
```
The TS shape MUST remain identical after the lift; the Rust body of
`start_work_units_watcher` becomes a thin wrapper that:
1. Constructs `codelet_core::work_units::WorkUnitsWatcher::new(<root>)`.
2. Sends an initial `StreamChunk::work_units_update(watcher.snapshot())`
   via the existing `ThreadsafeFunction` callback.
3. Spawns a task that drains `watcher.subscribe()` and forwards each
   `Vec<WorkUnitInfo>` payload as a new `StreamChunk::work_units_update`
   into the same callback.

`StreamChunk::WorkUnitsUpdate { work_units }` (types.rs:350, 539, 715)
remains unchanged — that's the JS-facing variant; Envelope is the
WS-facing one.

## 5. Test harness already in place (re-usable)

- `codelet/rpc-server/tests/common/mod.rs::connect_with_retry` —
  20ms-poll WS connect against ephemeral port. Re-used directly.
- `codelet/rpc-embedded/tests/source_helpers/mod.rs` —
  `workspace_root()`, `read_to_string_or_panic()`,
  `strip_rust_comments()`, `collect_rs_files()`. Re-used directly.
- `codelet/rpc-server/tests/websocket_transport.rs::ChildGuard` and
  `spawn_rpc_server()` — RAII wrapper around the binary, reads the
  port line off stdout. Pattern re-used; new tests need a `--workspace`
  flag because the binary now requires a workspace path.

## 6. Crate dep deltas

| Crate | Add to Cargo.toml |
|---|---|
| codelet/core | `notify = "6"`, `notify-debouncer-mini = "0.4"`, `serde_json` (already present), `tokio` (already present), `codelet-rpc-types = { workspace = true }` |
| codelet/rpc | `codelet-core = { workspace = true }`, `tokio = { workspace = true, features = ["sync", "rt"] }` |
| codelet/rpc-embedded | (no change — re-uses codelet-rpc) |
| codelet/rpc-server | `codelet-core = { workspace = true }` for `--workspace` resolution in main.rs only |
| codelet/napi | drop direct `notify` / `notify-debouncer-mini`, add `codelet-rpc-types` (already present), continue to use `codelet-core` |

## 7. Broadcast capacity decision

Per `architectureNotes[12]`: bounded `broadcast::channel(64)`. Snapshot
replacement payload — lagging subscribers receive `RecvError::Lagged`
and skip ahead. Acceptable because each new event makes the previous
one obsolete (full-snapshot semantics, not incremental delta).

## 8. New / updated test files

NEW:
- `codelet/rpc-embedded/tests/embedded_push.rs` — embedded `work_units_rx()` happy path + multiple subscribers
- `codelet/rpc-server/tests/ws_initial_snapshot.rs` — initial-frame-on-connect
- `codelet/rpc-server/tests/ws_push_on_mutation.rs` — file-mutation → WS frame
- `codelet/rpc-server/tests/cross_transport_push_parity.rs` — bincode byte-equality across transports
- `codelet/core/src/work_units.rs` unit tests (snapshot reading, `subscribe()` reliability)

UPDATED:
- `codelet/rpc-embedded/tests/architecture_invariants.rs` — scenario_10 widened
- `codelet/rpc-server/tests/websocket_transport.rs::scenario_6_*` — 4-variant reserved list
- `codelet/rpc-embedded/tests/embedded_happy_path.rs` — switch to live snapshot + cfg-test fixture
- `codelet/rpc-server/tests/parity.rs` — switch from `default_fixture()` to a temp workspace
- `src/__tests__/napi-workunitinfo-shape.test.ts` — add the watcher-callback-still-fires assertion
