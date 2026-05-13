# AST Research — RPC-007: Session RPCs + StreamChunk/LogEvent push channels

This document records the AST/code surface that RPC-007 must touch, gathered via
`AstGrep` and `Grep` over `codelet/`. It is the basis for the failing tests
written in the testing phase and for the implementation in the implementing
phase. Every entry below has been confirmed to exist at the cited line.

## 1. Existing FspecService trait (extension target)

- **File:** `codelet/rpc/src/lib.rs:33`
- **Symbol:** `pub trait FspecService { ... }`
- **Why it matters:** RPC-007 adds five new async methods to this trait
  (`list_sessions`, `create_session`, `send_input`, `interrupt`,
  `get_session_status`). Defined ONCE here per RPC-005 invariant.

## 2. Existing SharedFspecService struct (extension target)

- **File:** `codelet/rpc/src/lib.rs:44`
- **Symbol:** `pub struct SharedFspecService { ... }`
- **Why it matters:** Grows three new fields per RPC-007 architecture note [2]:
  `Arc<dyn SessionManagerHandle>` (or concrete, pending Q1 — answered:
  trait+handle), `broadcast::Sender<(SessionId, StreamChunk)>` chunks_tx,
  `broadcast::Sender<LogRecord>` logs_tx. Existing `list_work_units_calls`
  counter and `Arc<WorkUnitsWatcher>` from RPC-006 retained.

## 3. Existing Envelope enum (extension target)

- **File:** `codelet/rpc-server/src/envelope.rs:29`
- **Symbol:** `pub enum Envelope { ... }`
- **Why it matters:** RPC-007 changes `Event` from unit variant to
  `Event { session_id: SessionId, chunk: StreamChunk }` and `LogEvent` from
  unit to `LogEvent(LogRecord)`. `CmdReq` and `CmdRes` remain unit and
  reserved-and-rejected (rule [3]).

## 4. Existing handle_connection fan-out site (extension target)

- **File:** `codelet/rpc-server/src/server.rs:71`
- **Symbol:** `async fn handle_connection(...)`
- **Why it matters:** Currently spawns the `work_units_fanout` task introduced
  in RPC-006 (called at server.rs:61). RPC-007 adds two siblings —
  `chunks_fanout` and `logs_fanout` — using the same RecvError::Lagged
  debug-log-and-resync pattern.

## 5. Existing pump module (extension target)

- **File:** `codelet/rpc-server/src/pump.rs`
- **Why it matters:** `run_envelope_pump` match arms grow two new
  forward-to-inbound-handler cases for `Event` and `LogEvent`. Server-side
  rejects inbound; client-side demuxes onto `chunks_tx`/`logs_tx`. Same shape
  as `ClientInbound::on_work_units_update` referenced in pump.rs comments at
  line 3.

## 6. Existing embedded transport push surface (template for new methods)

- **File:** `codelet/rpc-embedded/src/lib.rs:89`
- **Symbol:** `pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>`
- **Why it matters:** RPC-007 adds two sibling methods with identical shape:
  `pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>`
  and `pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord>`. Zero-cost
  path — no envelope encoding.

## 7. Existing rpc-types pattern (target for type lifts)

- **File:** `codelet/rpc-types/src/lib.rs:27`
- **Symbol:** `pub struct WorkUnitInfo { ... }`
- **Why it matters:** Established `#[cfg_attr(feature = "napi",
  napi_derive::napi(...))]` pattern. RPC-007 lifts five new types here using
  the same pattern: `SessionId` (newtype String), `SessionInfo`,
  `SessionStatus` enum, `StreamChunk` tagged enum (all 23 variants verbatim
  per Q2 answer), `LogRecord`.

## 8. Source enum for the StreamChunk lift

- **File:** `codelet/napi/src/types.rs:219`
- **Symbol:** `pub enum StreamChunk { ... }`
- **Why it matters:** This is the 23-variant enum being lifted verbatim into
  rpc-types (Q2 answer). Carries `#[napi(discriminant="type")]` and many
  `#[napi(js_name=...)]` renames that MUST be preserved on the lifted enum so
  the TypeScript shape sees zero change. NAPI then re-exports.

## 9. SessionManager (DI target for SessionManagerHandle trait)

- **File:** `codelet/napi/src/session_manager.rs:3162`
- **Symbol:** `pub struct SessionManager { ... }` (8649 LOC file)
- **Why it matters:** Per Q1 answer (trait+Handle, no full lift), this struct
  stays in codelet/napi but implements a new `SessionManagerHandle` trait
  defined in `codelet/core` (or `codelet/rpc-types` if circular). The host
  (`rpc-server` main, `EmbeddedTransport::new`) injects an
  `Arc<dyn SessionManagerHandle>` into `SharedFspecService`.

## 10. Existing global chunk callback (parity target)

- **File:** `codelet/napi/src/session_manager.rs:58, 6352`
- **Symbols:**
  - `static GLOBAL_CHUNK_CALLBACK: OnceCell<GlobalChunkCallback>`
  - `pub fn session_set_global_chunk_callback(callback: ThreadsafeFunction<...>) -> Result<()>`
- **Other call sites:** session_manager.rs lines 989, 3542, 3796, 5077, 5357,
  6224, 6490 all read GLOBAL_CHUNK_CALLBACK
- **Why it matters:** RPC-007 must preserve this path verbatim while ALSO
  feeding the same chunks to the new `SharedFspecService::chunks_tx`
  broadcast. The existing tests at
  `codelet/napi/tests/global_chunk_callback_napi_test.rs` (lines 69, 95, 148,
  341) define the parity invariants that must remain green.

## 11. NAPI tracing layer (parity target for the new rpc-server Layer)

- **File:** `codelet/napi/src/lib.rs:152-205`
- **Symbol:** `TypeScriptLayer` (custom `tracing_subscriber::Layer`)
- **Why it matters:** RPC-007 introduces a sibling Layer in
  `codelet-rpc-server/src/main.rs` (and optionally
  `EmbeddedTransport::new`) that captures `level/target/message/timestamp` into
  a `LogRecord` and pushes onto `SharedFspecService::logs_tx`. NAPI's existing
  `setRustLogCallback` path stays unchanged.

## 12. StubProvider absence (test infrastructure to be created)

- **Searched:** `codelet/providers/src/`
- **Finding:** The plan claimed a "test-stub-provider" already exists in
  `codelet/providers`, but the only stub-like artifacts found are:
  - `TestProvider` at `codelet/providers/src/adapter.rs:243` — gated to
    `#[cfg(test)]` inside that module, not reusable across crates
  - Various `wiremock`-based HTTP stubs for OAuth flows (not LLM streaming)
  - `LIMITS-004: Lightweight resolver stub` at
    `codelet/providers/src/manager.rs:157` (config resolver, not a streaming
    provider)
- **Conclusion:** Q3 answer is correct — RPC-007 must create a new
  `StubProvider` in `codelet/providers/src/` gated behind a `test-support`
  Cargo feature, emitting `[StreamChunk::Text("hi back"), StreamChunk::Done]`
  on any `send_input`. The pre-existing `TestProvider` in adapter.rs stays
  untouched.

## 13. Existing source-shape regression test (extension target)

- **File:** `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` (per RPC-006
  context; verified as the shape-regression scaffold)
- **Why it matters:** RPC-007 widens the no-bincode-on-embedded-push assertion
  to cover the new `chunks_rx()` and `logs_rx()` paths, and adds a sibling
  type-uniqueness assertion that `StreamChunk`, `SessionInfo`, `SessionStatus`,
  `SessionId`, `LogRecord` are each defined exactly once across the workspace
  (in codelet/rpc-types).

## 14. Existing reserved-variants regression test (narrowing target)

- **File:** `codelet/rpc-server/tests/ws_reserved_variants_after_rpc006.rs`
  (per RPC-007 architecture note [9])
- **Why it matters:** Currently asserts the rejected set is `{Event, LogEvent,
  CmdReq, CmdRes}` after RPC-006. RPC-007 narrows it to `{CmdReq, CmdRes}`.

## Summary of file-level changes implied by this AST research

- **Modify:** `codelet/rpc/src/lib.rs`,
  `codelet/rpc-server/src/{envelope.rs, server.rs, client.rs, pump.rs}`,
  `codelet/rpc-server/src/main.rs` (Layer registration),
  `codelet/rpc-embedded/src/lib.rs`,
  `codelet/rpc-types/src/lib.rs`,
  `codelet/napi/src/{session_manager.rs, types.rs, lib.rs}`,
  `codelet/providers/src/lib.rs` (StubProvider export under test-support),
  `codelet/rpc-server/tests/ws_reserved_variants_after_rpc006.rs`,
  `codelet/rpc-embedded/tests/rpc_006_source_shape.rs`,
  `src/__tests__/napi-workunitinfo-shape.test.ts` (Vitest sibling)

- **Create:** `codelet/core/src/session_manager_handle.rs` (or co-locate in
  `codelet/rpc-types`),
  `codelet/providers/src/stub_provider.rs`,
  Rust integration tests:
  `codelet/rpc-embedded/tests/{embedded_session_repl.rs, embedded_log_event.rs}`,
  `codelet/rpc-server/tests/{ws_session_repl.rs, ws_log_event.rs, ws_multi_client_chunks.rs, cross_transport_chunk_parity.rs}`
