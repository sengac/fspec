# RPC-007 — Session RPCs + StreamChunk/LogEvent push channels (REPL backend)

**Parent:** RPC-002
**Predecessor:** RPC-006 (real work-units backing + first push variant)
**Successor:** RPC-008 (FspecBackend trait + transport selector)

## What we want

Stand up the **minimal** set of session-level RPCs and push channels
required to drive a basic agent REPL on top of the dual-transport
architecture. After this card the shared service crate exposes the
business logic an agent REPL needs (create a session, send a user
message, observe streaming chunks, observe log events) and both
transports carry the corresponding envelope variants end-to-end.

This is the second of three "fill out the RPC surface" cards. RPC-005
proved the architecture with a single read RPC, RPC-006 proved the push
half with `WorkUnitsUpdate`, RPC-007 proves the high-volume streaming
case with `StreamChunk` fan-out per session.

## Why this card

The frontend slice (RPC-009) requires:
- A list view of work units (covered by RPC-005 + RPC-006).
- An agent REPL: input → session → streaming chunks back → render.

The REPL cannot be wired up without these RPCs and the streaming push
channel, regardless of whether the frontend is embedded or remote.
Doing this here, behind the same dual-transport seam, means the basic
ratatui frontend in RPC-009 is transport-agnostic from day one.

## Existing RPC-005 + RPC-006 artifacts this card builds on

This card EXTENDS the same crates RPC-005 stood up and RPC-006 grew. No
new transports, no new envelope framing — just additional methods on the
existing `FspecService` trait and additional payload-carrying envelope
variants.

| Existing artifact | Path | RPC-007 action |
|---|---|---|
| `FspecService` tarpc trait | `codelet/rpc/src/lib.rs:25-28` | Add five new methods (`list_sessions`, `create_session`, `send_input`, `interrupt`, `get_session_status`). Single trait, single source of truth — the tarpc macro regenerates `FspecServiceClient` for both transports. |
| `SharedFspecService` | `codelet/rpc/src/lib.rs:36-59` | Adds `Arc<SessionManager>`, a `broadcast::Sender<(SessionId, StreamChunk)>`, and a `broadcast::Sender<LogRecord>` (the one populated by the `tracing_subscriber::Layer` registered at startup). Existing `list_work_units_calls` counter retained for parity tests. |
| `FspecServiceImpl` | `codelet/rpc/src/lib.rs:67-82` | Unchanged adapter shape — still `Arc<SharedFspecService>`. New trait methods delegate to `self.inner.session_manager()`. |
| `WorkUnitInfo` (rpc-types) | `codelet/rpc-types/src/lib.rs` | Existing pattern (cfg-gated `napi(object)` derive; `js_name = "workType"` for camelCase) is the template for every new type lifted in this card: `SessionId`, `SessionInfo`, `SessionStatus`, `StreamChunk`, `LogRecord`. |
| `Envelope` enum | `codelet/rpc-server/src/envelope.rs:26-41` | `Event` variant changes from unit to `Event { session_id: SessionId, chunk: StreamChunk }`. `LogEvent` changes from unit to `LogEvent(LogRecord)`. `CmdReq` / `CmdRes` remain reserved. `Envelope::variant_name()` updated. |
| `ServerStats` reserved-variant accounting | `codelet/rpc-server/src/lib.rs:46-71` | Removes `Event` and `LogEvent` from the rejected list (they are now legitimate). RPC-005 scenario 6 regression test adjusts to expect only `CmdReq` and `CmdRes` rejections. |
| `run_envelope_pump` + `InboundHandler` | `codelet/rpc-server/src/pump.rs` | RPC-006 already taught the pump to demux non-`Rpc` envelopes onto broadcast senders. RPC-007 adds two more sender wires: `chunks_tx` (server→client) and `logs_tx` (server→client). The `ServerInbound` / `ClientInbound` structs gain those senders as fields. |
| `bind_and_serve(addr, service)` | `codelet/rpc-server/src/server.rs:23-50` | Unchanged signature. `handle_connection` gains two more fan-out tasks (chunks and logs) alongside the work-units fan-out from RPC-006. All three fan-outs subscribe to `broadcast::Receiver`s exposed by `SharedFspecService`. |
| `ChannelTransport<Item, SinkItem>` | `codelet/rpc-server/src/transport.rs` | Unchanged. Tarpc-only bytes still flow through here. Push events bypass it entirely (same architecture as RPC-006). |
| `EmbeddedTransport` | `codelet/rpc-embedded/src/lib.rs` | Unchanged constructor. Adds two sibling subscription methods `chunks_rx()` and `logs_rx()` matching the WS client's signatures so the UI is transport-agnostic. |
| `FspecWsClient` (introduced in RPC-006) | `codelet/rpc-server/src/client.rs` | Adds `chunks_rx()` and `logs_rx()` matching the embedded API. Internal pump task already demuxes per RPC-006 generalisation. |
| Existing NAPI streaming surface | `codelet/napi/src/session_manager.rs`, `codelet/napi/src/lib.rs` (`setRustLogCallback`, `sessionSetGlobalChunkCallback`) | NAPI keeps re-registering its `ThreadsafeFunction` callbacks on the same broadcast senders inside `SharedFspecService`. The TS frontend's chunk/log handlers see no behaviour change — the broadcast channel is just a multi-listener fan-out, NAPI is one of the listeners. |
| RPC-005 source-shape regression tests | `codelet/rpc-embedded/tests/architecture_invariants.rs` | Stay green: trait + types still defined exactly once; embedded still requires `Handle`; loopback-only bind unchanged; rpc crate still has no `codelet-core` import (the session manager dependency lives behind a re-export shim — same pattern as the watcher in RPC-006). |
| Vitest smoke tests | `src/__tests__/napi-workunitinfo-shape.test.ts` + the watcher smoke from RPC-006 | Both still pass. New TS smoke (optional but recommended): drive `sessionManagerCreate` + `sessionSendInput` and assert chunks still arrive — codifies the NAPI invariant for streaming. |

## Architecture conformance with RPC-002

| RPC-002 decision / pattern | Source | RPC-007 obligation |
|---|---|---|
| Streaming events ride a sibling `tokio::sync::broadcast`, NOT a tarpc stream return | feasibility §5.1, §5.2, §5.3 ("tarpc is strictly single-shot req/res — it has no `stream Foo()`") | Both `Event(SessionStreamChunk)` and `LogEvent(LogRecord)` flow over broadcast channels. `send_input` returns `()` immediately; the caller observes streaming output via `chunks_rx()`. NO custom tarpc stream return type, NO long-poll RPC. |
| Multi-client semantics on the daemon | feasibility §6 ("`session_set_global_chunk_callback` is single-listener; in the daemon it becomes a `tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>` with one subscriber per attached client") | This is exactly the implementation. NAPI keeps its `ThreadsafeFunction` callback as one of the broadcast subscribers — same `Sender`, multiple receivers. |
| Reverse callbacks (`callFspecCommand`) explicitly out of scope | feasibility §8 ("Pick one when this work unit is broken down. Embedded mode collapses this to a direct function call; only remote mode needs the protocol.") | `CmdReq` / `CmdRes` envelope variants stay reserved-and-rejected. Decision between "two services" vs "envelope frames with correlation IDs" is made in RPC-011 or its own dedicated card, NOT here. |
| OAuth flows in remote mode | feasibility §7 (headless flow for remote, browser-loopback for embedded) | Out of scope for this card. None of the five RPCs touch OAuth. |
| Cancellation via tarpc `context::Context` | feasibility §11 risks list ("Ensure this maps cleanly onto `session_interrupt`") | Out of scope for this card per RPC-005 architecture rule [14] ("Cancellation testing deferred to the streaming/long-running-RPC card"). `interrupt(session_id)` is a normal RPC, not a `context::Context.deadline` story. |
| Single source of truth for types and impl | RPC-005 architecture rules [1]–[3] | All five new types lifted into `codelet/rpc-types`; impl lives once in `codelet/rpc::SharedFspecService`; both transports delegate. |
| Q9 embedded runtime invariant | feasibility §6, RPC-005 rule [4] | Two new fan-out tasks (chunks, logs) spawn on the host `Handle`. NO `Runtime::new`. RPC-005 `scenario_7_*` source-shape stays green. |
| Wire format default = bincode | feasibility §5.2, RPC-005 rule [5] | `Event` and `LogEvent` ride the same bincode pump. |
| Multi-client subscription filters | feasibility §11 risks list ("When two clients attach to the same session, what does input arbitration look like? Read-only observer mode?") | Explicitly NOT this card. Every connected client receives every session's chunks. Per-client filters arrive in a follow-up card alongside the read-only observer-mode question. |

## Scope (minimum viable)

### RPC methods (added to `FspecService` in `codelet/rpc/src/lib.rs`)

```rust
async fn list_sessions() -> Vec<SessionInfo>;
async fn create_session(role: Option<String>) -> SessionId;
async fn send_input(session_id: SessionId, text: String) -> ();
async fn interrupt(session_id: SessionId) -> ();
async fn get_session_status(session_id: SessionId) -> SessionStatus;
```

These mirror existing NAPI functions verbatim (`sessionManagerCreate`,
`sessionManagerList`, `sessionSendInput`, `sessionInterrupt`,
`sessionGetStatus`). Mechanical lift; the implementations delegate to
the same `SessionManager::instance()` business logic the NAPI bindings
already use.

Lifted types added to `codelet/rpc-types`:
- `SessionId` (newtype around `String`)
- `SessionInfo` (mirror of NAPI struct: id, role, status, created_at, ...)
- `SessionStatus` enum
- `StreamChunk` (already a tagged enum on the NAPI side; serde-clean)
- `LogRecord` (level, target, message, timestamp)

NAPI re-exports preserved per the RPC-005 invariant.

### Envelope variants implemented

Building on RPC-006:
- `Envelope::Event(SessionStreamChunk { session_id, chunk })` — per-session
  streaming output (LLM text deltas, tool calls, tool results, status
  transitions).
- `Envelope::LogEvent(LogRecord)` — process-wide structured log feed
  (replaces `setRustLogCallback`).

`CmdReq` / `CmdRes` remain reserved (deferred to RPC-011 if needed).

### Push pipeline

Server side: `SessionManager`'s existing per-session output is wrapped
in a `tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>` housed in
`SharedFspecService`. The `rpc-server` per-connection task adds a
subscriber that forwards every chunk as `Envelope::Event(...)`. Fan-out
is unfiltered in this card — every connected client receives every
session's chunks. Per-client subscription filters arrive in a later card.

Embedded transport: same `broadcast::Receiver<(SessionId, StreamChunk)>`
exposed via `chunks_rx()`. No envelope wrapping.

LogEvent uses a separate broadcast channel populated by a custom
`tracing_subscriber::Layer` registered at `codelet-rpc-server` startup
(and at `EmbeddedTransport::new` when the host opts in).

### Tests

- Both transports: create session, send "hi", assert at least one
  `StreamChunk::AssistantTextDelta` arrives within 5 s (with a stub
  provider so we don't need real model credentials in CI; reuse the
  test-stub-provider that already exists in `codelet/providers`).
- Parity: same input, same stub provider, both transports yield the
  same chunk sequence.
- LogEvent: emit `tracing::info!("hello")` on the server, assert client
  receives a `LogRecord` envelope with matching message.
- Reserved-variants regression: `CmdReq`, `CmdRes` still rejected.

## Out of scope

- Reverse callbacks (`callFspecCommand` / `CmdReq` / `CmdRes`) — RPC-011.
- Per-client subscription filters / ack / replay buffer.
- OAuth flows, authentication.
- The remaining ~180 NAPI functions — own card later, mostly
  mechanical.
- Cancellation propagation via tarpc `context::Context` — separate card
  once long-running RPCs exist.

## Acceptance — done when

1. Five session RPC methods on `FspecService`, both transports.
2. `Event` and `LogEvent` envelope variants flow on the wire.
3. Embedded transport exposes `chunks_rx()` and `logs_rx()` with
   identical signatures to the WS client.
4. Stub-provider integration test sends a message and observes streaming
   chunks on both transports.
5. NAPI behaviour unchanged.
6. RPC-005 + RPC-006 regressions still pass.

## Estimate guidance

8 points. Largest piece is the type lift for `StreamChunk` — the NAPI
struct has many variants. Once lifted, the actual session-RPC delegates
are one-liners.
