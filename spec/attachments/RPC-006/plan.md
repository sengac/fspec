# RPC-006 — Real work-units backing + first streaming envelope variant (WorkUnitsUpdate)

**Parent:** RPC-002 (Rust ratatui frontend with dual transport over tarpc)
**Predecessor:** RPC-005 (foundation — fixture-backed `list_work_units`)
**Successor in chain:** RPC-007 (session RPCs + StreamChunk events)

## What we want

After RPC-005, `list_work_units()` returns a hard-coded two-element fixture
that lives in `codelet/rpc/src/lib.rs::default_fixture()`. The next card
on the path to a real ratatui frontend has two coupled goals:

1. Make `list_work_units()` return **real** project work units.
2. Prove the **streaming/push** half of the dual-transport architecture
   by adding the first envelope variant beyond `Rpc(_)`:
   `Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>)`.

Together these unblock the work-units **list view** (RPC-009) — which
needs both an initial RPC fetch and live updates when the source file
changes.

## Why this card

- `default_fixture()` was deliberate (RPC-005 rule [10]) so the spike was
  not coupled to NAPI runtime state. That guard rail has served its
  purpose; the next slice needs real data.
- The RPC-002 feasibility doc (§5.2) calls out five envelope variants —
  `Rpc`, `Event`, `LogEvent`, `WorkUnitsUpdate`, `CmdReq`, `CmdRes`. Only
  `Rpc` was implemented in RPC-005; the others were deliberately reserved
  and rejected by the server. We implement `WorkUnitsUpdate` first because
  it has the simplest semantics (full snapshot replacement, no
  correlation IDs, no per-session fan-out) and exercises the full push
  pipeline end-to-end.

## Existing RPC-005 artifacts this card builds on

This card EXTENDS the four crates RPC-005 created. It MUST NOT duplicate
or fork any of the symbols below — every change is an in-place edit or
an additive extension.

| Existing artifact | Path | RPC-006 action |
|---|---|---|
| `WorkUnitInfo` struct (lifted from NAPI in RPC-005) | `codelet/rpc-types/src/lib.rs` | Reuse verbatim. Carried unchanged in the new `Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>)` payload. |
| `FspecService` tarpc trait | `codelet/rpc/src/lib.rs:25-28` | Unchanged. `list_work_units()` keeps the same signature; only its body changes. |
| `SharedFspecService` (constructor takes `Vec<WorkUnitInfo>`) | `codelet/rpc/src/lib.rs:36-59` | Constructor signature changes to `new(watcher: Arc<WorkUnitsWatcher>)`. The existing `list_work_units_calls` parity counter is retained. |
| `FspecServiceImpl` (cloneable adapter) | `codelet/rpc/src/lib.rs:67-82` | Unchanged. Body of `list_work_units` becomes `self.inner.watcher.snapshot()`. |
| `default_fixture()` | `codelet/rpc/src/lib.rs:93-114` | Becomes `#[cfg(test)] pub(crate) fn test_fixture()` (or moved to a `#[cfg(test)] mod`). All RPC-005 tests that import it switch to the test path. |
| `Envelope` enum | `codelet/rpc-server/src/envelope.rs:26-41` | `WorkUnitsUpdate` variant changes shape from unit `WorkUnitsUpdate` to `WorkUnitsUpdate(Vec<WorkUnitInfo>)`. Other reserved variants unchanged. `Envelope::variant_name()` updated accordingly. |
| `ServerStats` | `codelet/rpc-server/src/lib.rs:46-71` | Extended: `WorkUnitsUpdate` no longer increments `rejected_envelopes` / `rejected_variants` (it is now a legitimate variant). Existing reserved-variants regression tests adjust their expected list. |
| `bind_and_serve(addr, service)` | `codelet/rpc-server/src/server.rs:23-50` | Unchanged signature. Internally `handle_connection` gains a fan-out task that subscribes to `service.watcher_rx()` and pushes `Envelope::WorkUnitsUpdate(...)` frames onto the WS sink. |
| `run_envelope_pump` + `InboundHandler` (`ServerInbound`, `ClientInbound`) | `codelet/rpc-server/src/pump.rs` | Re-used as-is. Server-side: `WorkUnitsUpdate` arrives outbound from the fan-out task and is encoded by the existing `Envelope::Rpc`-style code path generalised to "any envelope" — see Step 3. Client-side: pump grows a second outbound channel that demuxes inbound `WorkUnitsUpdate` envelopes onto a `broadcast::Sender<Vec<WorkUnitInfo>>`. |
| `ChannelTransport<Item, SinkItem>` | `codelet/rpc-server/src/transport.rs` | Unchanged. Continues to bridge tarpc bytes only. The new push channel is wired alongside, not through, this transport. |
| `EmbeddedTransport::new(handle, service)` + `EmbeddedTransport::client()` | `codelet/rpc-embedded/src/lib.rs:42-66` | Constructor unchanged (still takes a `Handle` per Q9). Adds a sibling method `EmbeddedTransport::work_units_rx() -> broadcast::Receiver<Vec<WorkUnitInfo>>` that returns a fresh subscription cloned from the watcher inside `SharedFspecService`. |
| `ws_client_connect(ws)` | `codelet/rpc-server/src/client.rs` | Returns a richer struct (e.g. `FspecWsClient { rpc: FspecServiceClient, work_units_rx: broadcast::Receiver<Vec<WorkUnitInfo>> }`) instead of a bare `FspecServiceClient`. Existing call sites in tests update to `.rpc`. |
| `codelet-rpc-server` binary | `codelet/rpc-server/src/main.rs` | Updated to construct a real `WorkUnitsWatcher` (or accept a `--workspace` arg) instead of `default_fixture()`. Output contract (single port line + ctrl_c shutdown) stays the same. |
| RPC-005 source-shape regression tests | `codelet/rpc-embedded/tests/architecture_invariants.rs` | Stay green: `WorkUnitInfo` still defined exactly once; embedded transport still requires a `Handle`; rpc-server still binds 127.0.0.1; `codelet/rpc/Cargo.toml` still has no `codelet-core` dep — see Step 1 for why the lift goes to `codelet/core` not into `codelet/rpc`. |
| Vitest smoke test | `src/__tests__/napi-workunitinfo-shape.test.ts` | Must still pass after the watcher lift; gain one additional assertion that the existing NAPI `startWorkUnitsWatcher` callback continues to fire. |

## Architecture conformance with RPC-002

This card is purely transport-layer; it adds NO ratatui code. The
RPC-002 decisions it must respect are the transport-side ones from the
feasibility doc.

| RPC-002 decision / pattern | Source | RPC-006 obligation |
|---|---|---|
| Push events ride a sibling `tokio::sync::broadcast` channel, NOT a tarpc stream return | feasibility §5.1, §5.2, §5.3 | `WorkUnitsUpdate` flows over `tokio::sync::broadcast::Sender<Vec<WorkUnitInfo>>`. Tarpc keeps its strict req/res semantics. NO `tarpc::server::ChannelGroup` workaround, NO custom stream-return shim — `WorkUnitsUpdate` is a sibling envelope variant per feasibility §5.2 verbatim. |
| Embedded mode hands the app a raw `broadcast::Receiver`, no envelope multiplexing | feasibility §5.1 ("the embedded mode just hands the ratatui app a `tokio::sync::broadcast::Receiver<StreamChunk>` directly") | `EmbeddedTransport::work_units_rx() -> broadcast::Receiver<Vec<WorkUnitInfo>>` returns the watcher's sender's subscription verbatim. NO bincode encoding on the embedded path. |
| Wire format default = bincode for the WS transport | feasibility §5.2, RPC-005 architecture rule [5] | The new `Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>)` variant rides the SAME bincode-of-Envelope pump that RPC-005 established. No JSON debug envelope in this card. |
| Q9: embedded transport requires host runtime `Handle`, never spawns its own | feasibility §6, RPC-005 architecture rule [4] | The new fan-out task in `EmbeddedTransport` is `self.handle.spawn(...)` — the host's runtime, not a new `Runtime::new`. The RPC-005 source-shape regression test stays green. |
| Single source of truth for shared types (`rpc-types`) | RPC-005 architecture rule [1] | `Vec<WorkUnitInfo>` payload reuses the existing lifted type. No NAPI fork, no shadowed type. |
| Single source of truth for business logic (shared service impl) | RPC-005 architecture rule [3] | The watcher lift goes to `codelet/core/src/work_units.rs` (NOT into `codelet/rpc/`) so the dependency arrow stays right-to-left: `rpc → core` is allowed; `rpc → napi` is NOT. RPC-005 source-shape regression `scenario_10_*` (no `codelet-core` dep on the rpc crate) WIDENS in this card to "no codelet-napi import in rpc; codelet-core import is permitted". |

## How we get there

### Step 1 — Lift the work-units watcher into pure-Rust

`codelet/napi/src/work_units_watcher.rs` already implements the
`notify`-based file watcher. Today it lives behind the NAPI feature gate
and emits via a `ThreadsafeFunction` callback into Node.

Move the watcher out of `codelet/napi/` into a new module under
`codelet/core/` (tentatively `codelet/core/src/work_units.rs`) so it is
reachable from the shared service crate without pulling NAPI in. Three
required surfaces:

```rust
// codelet/core/src/work_units.rs
pub fn read_snapshot(workspace: &Path) -> Result<Vec<WorkUnitInfo>>;

pub struct WorkUnitsWatcher {
    pub fn new(workspace: &Path) -> Result<Self>;
    pub fn snapshot(&self) -> Vec<WorkUnitInfo>;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Vec<WorkUnitInfo>>;
}
```

`codelet/napi/` keeps a thin shim that wraps `WorkUnitsWatcher` and
forwards each broadcast event into the existing `ThreadsafeFunction`
callback so the TS frontend behaviour is unchanged.

### Step 2 — Wire the watcher into the shared service

```rust
// codelet/rpc/src/lib.rs
pub struct SharedFspecService {
    watcher: Arc<WorkUnitsWatcher>,
    // counter retained from RPC-005 for parity tests
}

impl SharedFspecService {
    pub fn new(watcher: Arc<WorkUnitsWatcher>) -> Self { ... }
}

impl FspecService for FspecServiceImpl {
    async fn list_work_units(self, _ctx: Context) -> Vec<WorkUnitInfo> {
        self.inner.watcher.snapshot()
    }
}
```

`default_fixture()` survives but as `#[cfg(test)] pub fn test_fixture()` —
test-only.

### Step 3 — Add the WorkUnitsUpdate envelope variant

Today `Envelope` has six variants; only `Rpc(_)` is implemented. Light
up `WorkUnitsUpdate(Vec<WorkUnitInfo>)`:

- **Server side (`codelet/rpc-server`):** alongside the per-connection
  tarpc pump, spawn a fan-out task that subscribes to
  `WorkUnitsWatcher::subscribe()` and emits a bincode-encoded
  `Envelope::WorkUnitsUpdate(snapshot)` frame every time the watcher
  fires. Initial snapshot sent immediately on connection (no need for
  an explicit subscribe RPC at this stage — keep it simple).
- **Client side:** the existing `pump.rs` already demultiplexes
  envelopes; teach it to forward `WorkUnitsUpdate` frames to a
  `tokio::sync::broadcast::Sender<Vec<WorkUnitInfo>>` exposed via the
  client struct.
- **Embedded:** no envelope at all — `EmbeddedTransport::work_units_rx()`
  returns the watcher's broadcast receiver directly. Same trait signature,
  zero-cost path.

### Step 4 — Backend trait shape preview

To keep RPC-008 straightforward we expose a uniform shape now:

```rust
// codelet/rpc-embedded
pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;

// codelet/rpc-server (client side)
impl FspecWsClient {
    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>;
}
```

Both transports have the same signature; the UI never knows which it
holds.

### Step 5 — Tests

- **Embedded:** mutate a temp `spec/work-units.json`, observe the
  receiver, assert payload.
- **WebSocket:** spawn rpc-server bound to a temp workspace, connect a
  client, mutate the file, assert the client receives a
  `WorkUnitsUpdate` frame with the new snapshot.
- **Parity:** same mutation, both transports, payload bytes equal.
- **NAPI smoke:** existing `getAllWorkUnits()` Vitest test still passes
  + a new test that the NAPI watcher callback still fires after the lift.
- **Reserved-variants:** `Event`, `LogEvent`, `CmdReq`, `CmdRes` remain
  rejected by the server (regression of RPC-005 scenario 6).

## Out of scope — explicitly deferred

- Other envelope variants (`Event`, `LogEvent`, `CmdReq`, `CmdRes`) →
  RPC-007 picks up `Event` + `LogEvent`.
- Per-subscription filtering, ack/nack, back-pressure — broadcast channel
  with a bounded capacity is fine for now.
- Authentication on the WS connection — deferred to a daemon-topology
  card.
- Full `codelet/core` reorganisation — only the work-units watcher
  module moves.

## Acceptance — done when

1. `default_fixture()` is `#[cfg(test)]`-only; the production
   `SharedFspecService` reads from a real `WorkUnitsWatcher`.
2. `Envelope::WorkUnitsUpdate(Vec<WorkUnitInfo>)` flows on the wire
   (bincode), demuxed by the client into a broadcast channel.
3. Both transports expose an identical `work_units_rx()` API surface.
4. NAPI re-exports + Vitest smoke unchanged.
5. New integration tests cover both transport push paths + parity.
6. RPC-005 source-shape and reserved-variant tests still pass.

## Estimate guidance

5 points. Watcher lift is mechanical (~200 LoC of code already exists in
NAPI form). Envelope variant is small (~80 LoC). Test plan is the
expensive part (~6 new integration tests).
