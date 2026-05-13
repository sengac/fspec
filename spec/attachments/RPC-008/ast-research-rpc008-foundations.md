# AST Research — RPC-008 Foundations

**Work Unit:** RPC-008 — FspecBackend trait + transport selector + ratatui app shell (`codelet/fspec-tui` crate)
**Date:** 2026-05-10
**Scope:** Survey of existing Rust source in `codelet/` to ground every architecture rule and scenario in actual code that already exists. RPC-008 is a NEW crate — this research enumerates the *consumed* surfaces (trait targets, constructors, source-shape invariants) and confirms there is no pre-existing `fspec-tui` crate to merge with.

---

## 1. Workspace registration baseline (`codelet/Cargo.toml`)

`[workspace] members` currently lists exactly:

```
"cli", "common", "core", "git", "napi", "providers",
"rpc", "rpc-embedded", "rpc-server", "rpc-types",
"tools", "tui"
```

**Existing `tui` crate** (`codelet/tui`) is the legacy markdown/diff renderer for `codelet-cli`; it depends on `crossterm` only and contains NO ratatui code. **Not the same crate as RPC-008's `fspec-tui`** — they coexist.

`[workspace.dependencies]`:
- `tokio-tungstenite = { version = "0.26", features = ["rustls-tls-webpki-roots"] }` — **already present** (BRIDGE-001). RPC-008 reuses this.
- `tarpc = { version = "0.34", features = ["full"] }` — **already present** (RPC-005).
- `crossterm = { version = "0.28", features = ["event-stream"] }` — **already present**, RPC-008 will re-export from `event-stream` for `EventStream`.
- `ratatui` — **NOT present**. RPC-008 must add it.
- `tui-popup` — **NOT present**. RPC-008 must add it (`= "0.6"` per architecture note).
- `insta` — **NOT present**. RPC-008 must add as `[dev-dependencies]`.
- `async-trait = "0.1"` — **already present**.
- `anyhow = "1"` / `tracing = "0.1"` — **already present**.

**RPC-008 net Cargo.toml deltas:** add `ratatui`, `tui-popup`, `insta` to `[workspace.dependencies]`; register `"fspec-tui"` member.

---

## 2. The five workspace crates RPC-008 depends on

### 2.1 `codelet-rpc-types` (re-exported types)

`codelet/rpc-types/src/lib.rs` exports the cross-transport types RPC-008's trait surface returns:

- `pub struct WorkUnitInfo { id, title, work_type, status, description, estimate, epic }`
- `pub struct SessionInfo { id, role, ... }`
- `pub type SessionId = uuid::Uuid` (verify exact alias when wiring trait)
- `pub struct StreamChunk { ... }`
- `pub struct LogRecord { ... }`

These are the **exact** parameter/return types the FspecBackend trait must use — so the embedded and WS impls round-trip identical payloads.

### 2.2 `codelet-rpc` (shared service + tarpc client)

`codelet-rpc` exports `FspecServiceClient` (the tarpc-generated request/response client) and `SharedFspecService` (the singleton service impl that both transports share). Re-exported transitively through `codelet-rpc-embedded`'s prelude (`pub use codelet_rpc::{FspecServiceClient, FspecServiceImpl, SharedFspecService}`), so `EmbeddedFspecBackend` only needs `codelet-rpc-embedded` + `codelet-rpc-types` to compile.

### 2.3 `codelet-rpc-embedded::EmbeddedTransport` (the wrapped target)

`codelet/rpc-embedded/src/lib.rs` lines 39–106 — the constructor RPC-008's `EmbeddedFspecBackend` will wrap:

```rust
pub struct EmbeddedTransport {
    handle: tokio::runtime::Handle,
    service: Arc<SharedFspecService>,
}

impl EmbeddedTransport {
    pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self { ... }
    pub fn client(&self) -> FspecServiceClient { ... }
    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> { ... }
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> { ... }
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> { ... }
}
```

**Key shape locked by RPC-002 Q9:** `new` takes `tokio::runtime::Handle` as a non-defaulted argument. RPC-008's `EmbeddedFspecBackend::new(handle, service)` MUST mirror this signature so the source-shape regression for `fspec-tui` (the widened `scenario_7_*`) passes.

The three `*_rx` methods return broadcast receivers DIRECTLY — no envelope, no fan-out task. RPC-008's `EmbeddedFspecBackend::{work_units_rx, chunks_rx, logs_rx}` simply delegates: `self.transport.work_units_rx()` etc.

### 2.4 `codelet-rpc-server::{ws_client_connect, FspecWsClient, bind_and_serve}`

`codelet/rpc-server/src/client.rs` lines 35–177 — the consumed surface for `WebSocketFspecBackend`:

```rust
pub struct FspecWsClient {
    pub rpc: FspecServiceClient,
    work_units_watch_tx: watch::Sender<Option<Vec<WorkUnitInfo>>>,
    chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>,
    logs_tx: broadcast::Sender<LogRecord>,
}

impl FspecWsClient {
    pub fn client(&self) -> &FspecServiceClient { ... }
    pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> { ... }
    pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> { ... }
    pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> { ... }
}

pub async fn ws_client_connect<S>(ws: WebSocketStream<S>) -> anyhow::Result<FspecWsClient>
where S: AsyncRead + AsyncWrite + Unpin + Send + 'static
```

**RPC-008 wiring:** `WebSocketFspecBackend::connect(url)` calls `tokio_tungstenite::connect_async(url).await?`, takes `.0` (the `WebSocketStream`), passes it to `ws_client_connect`, and stores the resulting `FspecWsClient` on the struct. Trait impl forwards every method.

**Confirmed (rule [22]):** `FspecWsClient::work_units_rx()` already replays the cached snapshot before forwarding — so the WS smoke test scenario "initial WorkUnitsUpdate snapshot frame from RPC-006 is observed within 5 seconds" is guaranteed by EXISTING code in client.rs:73–104, not by anything RPC-008 must add.

`codelet/rpc-server/src/lib.rs` lines 30–33 confirms re-exports:

```rust
pub use client::{ws_client_connect, FspecWsClient};
pub use server::bind_and_serve;
```

So RPC-008's WS smoke integration test imports `bind_and_serve` and `ws_client_connect` from `codelet_rpc_server` and `EmbeddedFspecBackend` from the new crate.

### 2.5 NO existing `fspec-tui` or ratatui code in `codelet/`

Confirmed greenfield: no file in `codelet/` references `ratatui::`, `tui_popup::`, `Component`, `Compositor`, or `FspecBackend`. RPC-008 is genuinely new code; nothing to merge or refactor.

---

## 3. Source-shape invariant baseline (RPC-005 architecture_invariants.rs)

`codelet/rpc-embedded/tests/architecture_invariants.rs::scenario_7_embedded_transport_requires_tokio_handle_at_construction` (lines 22–68) is the test that RPC-008 widens (rule [scenario "scenario_7_* widened to scan fspec-tui"]).

Current behaviour:

```rust
let src_dir = workspace_root().join("rpc-embedded").join("src");
let rs_files = collect_rs_files(&src_dir);
// scans for: tokio::runtime::Builder, runtime::Builder::new_multi_thread,
//            runtime::Builder::new_current_thread, tokio::runtime::Runtime::new,
//            Runtime::new()
```

**RPC-008 widens this to:**

```rust
for crate_dir in ["rpc-embedded", "fspec-tui"] {
    let src_dir = workspace_root().join(crate_dir).join("src");
    // ...same forbidden-pattern scan...
    // assertion message must identify which crate violated the rule
}
```

The five forbidden substrings are reused verbatim; that's why the corresponding RPC-008 scenario lists them out (Background=100..Critical=1000 are RPC-008's own enum, NOT related to these substrings).

**Reusable helper:** `codelet/rpc-embedded/tests/source_helpers/mod.rs` exports `collect_rs_files`, `read_to_string_or_panic`, `strip_rust_comments`, `workspace_root` — RPC-008's own source-shape integration test in `codelet/fspec-tui/tests/` should depend on the same helpers via `path = "../../rpc-embedded/tests/source_helpers"` OR (cleaner) have a tiny duplicate; we will decide during implementation. Architecture note already commits to per-crate test trees.

---

## 4. Existing Cargo invariants RPC-008 inherits

`[workspace.lints.clippy]` from `codelet/Cargo.toml` already denies:

```
expect_used = "deny"
unwrap_used = "deny"
panic = "deny"
```

with `unsafe_code = "deny"` at the rust level. **RPC-008 code MUST NOT use `.unwrap()`, `.expect()`, `panic!()`, or `unsafe`** — every fallible path needs `?` propagation or an `anyhow::Result` return. Test files already pre-allow these via `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` at the top of test crates (see architecture_invariants.rs line 15).

**Implication for RPC-008:** integration tests under `codelet/fspec-tui/tests/` should mirror the same `#![allow(...)]` header. Production code in `codelet/fspec-tui/src/` MUST use `anyhow::Result<T>` / `?` everywhere.

---

## 5. Reference patterns RPC-008 will mirror

### 5.1 `EmbeddedTransport`'s `*_rx` direct delegation (rpc-embedded/src/lib.rs:88–106)

```rust
pub fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>> {
    self.service.watcher_rx()
}
pub fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)> {
    self.service.chunks_rx()
}
pub fn logs_rx(&self) -> broadcast::Receiver<LogRecord> {
    self.service.logs_rx()
}
```

**RPC-008's `EmbeddedFspecBackend` adopts an identical delegation shape** but goes one layer up — wraps `EmbeddedTransport` rather than `SharedFspecService`. The `*_rx` methods on the trait return the same broadcast::Receiver types so a `dyn FspecBackend` consumer is type-identical across both impls.

### 5.2 `FspecWsClient::work_units_rx` snapshot replay (rpc-server/src/client.rs:79–105)

The WS impl already handles the "subscriber arriving after the initial frame" case via a `watch::Sender` that holds the latest snapshot and a per-subscriber forwarder task. RPC-008's `WebSocketFspecBackend::work_units_rx` is a one-line forward to `self.client.work_units_rx()` — the heavy lifting lives in client.rs.

### 5.3 `bind_and_serve` test pattern (rpc-embedded/tests/embedded_happy_path.rs and embedded_push.rs)

Every existing WS integration test follows the shape:

```rust
let (addr, _stats, _join) = bind_and_serve("127.0.0.1:0", service).await?;
let ws_url = format!("ws://{}", addr);
let (ws, _) = tokio_tungstenite::connect_async(&ws_url).await?;
let client = ws_client_connect(ws).await?;
// ... exercise client ...
```

**RPC-008's WS smoke test scenario** ("WebSocketFspecBackend smoke test round-trips list_work_units across the WS wire") replaces the bottom three lines with:

```rust
let backend = WebSocketFspecBackend::connect(&ws_url).await?;
// ... exercise backend.list_work_units().await + backend.work_units_rx() ...
```

This proves transport-agnostic parity with the embedded smoke (rules [16][17] of the feature file).

---

## 6. Out-of-scope confirmations

Searched for any existing references to functionality RPC-008 explicitly DEFERS:

- **`MouseTrackingToggle`** — not present. Mouse capture is enabled at `TerminalGuard::init()` time only; per-component opt-out lands in RPC-009.
- **`VirtualList` / `MultiLineInput`** — not present. The placeholder `HelloComponent` renders static text; real list/REPL widgets land in RPC-009.
- **Binary entry points** — confirmed by `codelet/fspec-tui/Cargo.toml` containing only `[lib]`. The `fspec-embedded-tui` and `fspec-remote-tui` binaries land in RPC-010.
- **`get_session_status`** — intentionally absent from FspecBackend trait surface for RPC-008; added later if RPC-009 needs it (per architecture-locked Q9 wording).

---

## 7. Summary of consumed surfaces

| Consumed item | Source crate | Source path | RPC-008 usage |
|---|---|---|---|
| `WorkUnitInfo`, `SessionInfo`, `SessionId`, `StreamChunk`, `LogRecord` | `codelet-rpc-types` | `src/lib.rs` | Trait method signatures |
| `EmbeddedTransport::new(handle, service)` | `codelet-rpc-embedded` | `src/lib.rs:46–55` | `EmbeddedFspecBackend` wraps |
| `EmbeddedTransport::{work_units_rx,chunks_rx,logs_rx}` | `codelet-rpc-embedded` | `src/lib.rs:88–106` | Trait impl forwards |
| `FspecServiceClient` (RPCs) | `codelet-rpc` (re-export) | — | Underlying tarpc client |
| `ws_client_connect`, `FspecWsClient` | `codelet-rpc-server` | `src/client.rs:121, 43` | `WebSocketFspecBackend::connect` |
| `bind_and_serve` | `codelet-rpc-server` | `src/server.rs:49` | WS smoke test fixture |
| `tokio_tungstenite::connect_async` | crate dep | — | Inside `WebSocketFspecBackend::connect` |
| `collect_rs_files`, `strip_rust_comments`, `workspace_root` | `rpc-embedded/tests/source_helpers` | `mod.rs` | Source-shape regression test |
| `scenario_7_embedded_transport_requires_tokio_handle_at_construction` | `rpc-embedded/tests/architecture_invariants.rs:22` | — | **Widened** to scan `fspec-tui/src/` too |

**No RPC-008 production code modifies any existing crate.** The only existing-file edits are:
1. `codelet/Cargo.toml` — register `fspec-tui` member, add `ratatui`/`tui-popup`/`insta` to `[workspace.dependencies]`.
2. `codelet/rpc-embedded/tests/architecture_invariants.rs::scenario_7_*` — widen directory-scan to include `fspec-tui/src/`.

Everything else lives under `codelet/fspec-tui/` (new directory).
