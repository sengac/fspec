# AST Research — Existing API Surface for RPC-010

**Work unit:** RPC-010 — `fspec` binary (combined / daemon / client subcommands)
**Date:** 2026-05-11
**Method:** AstGrep + ripgrep across `codelet/` to verify the existing API
surfaces that RPC-010 wires together, so the cross-references in rules /
examples / architecture notes are accurate to the millisecond and the
testing-phase scaffold can compile against the real signatures.

## 1. `codelet_rpc_server::bind_and_serve` (RPC-005)

**File:** `codelet/rpc-server/src/server.rs:49`

```rust
pub async fn bind_and_serve(
    bind_addr: &str,
    service: Arc<SharedFspecService>,
) -> anyhow::Result<(SocketAddr, ServerStats, tokio::task::JoinHandle<()>)>
```

- `bind_addr: &str` — directly callable with `"127.0.0.1:0"` or any
  loopback `SocketAddr`-parsable string.
- Returns `(SocketAddr, ServerStats, JoinHandle<()>)`. The `JoinHandle` is
  what combined-mode's `Drop` / shutdown sequence aborts before removing
  daemon.json (rule [23]).
- `ServerStats { service: Arc<SharedFspecService>, rejected_envelopes,
  rejected_variants }` — the embedded `service` field gives test access to
  `service.list_work_units_calls()` for daemon-side observability
  assertions (rule [22]).

## 2. `codelet_rpc::SharedFspecService` (RPC-006)

**File:** `codelet/rpc/src/lib.rs:83-131`

- `SharedFspecService::new(watcher: Arc<WorkUnitsWatcher>) -> Self`
- `list_work_units_calls(&self) -> u64` — atomic counter used in
  scenario "Client bootstrap observability — daemon-side counter
  increments by 1".
- `logs_tx() -> broadcast::Sender<LogRecord>` — the sender pushed onto
  the codelet-rpc global Vec by `register_log_layer`.

## 3. `codelet_rpc::register_log_layer` (RPC-007)

**File:** `codelet/rpc/src/log_layer.rs:142-162`

```rust
pub fn register_log_layer(
    service: Arc<SharedFspecService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
```

Critical ordering finding (logged as RPC-010 architecture note #7):

- `register_log_layer` does TWO things atomically:
  1. **Unconditionally** pushes `service.logs_tx()` onto the codelet-rpc
     process-global `Vec<broadcast::Sender<LogRecord>>` (line 146).
  2. Conditionally installs a `tracing_subscriber::registry().with(
     BroadcastLogLayer).try_init()` (no-op if any global subscriber is
     already present — line 157-159).

- Implication: `init_tracing_daemon()` cannot call `register_log_layer`
  *after* `tracing_subscriber::fmt().init()` — the second `init` is a
  no-op and the `BroadcastLogLayer` never gets installed. Solution:
  build the registry directly with BOTH layers, then call
  `register_log_layer` to push the sender. `BroadcastLogLayer` is already
  a public unit struct, so no codelet-rpc refactor is required.

## 4. `codelet_fspec_tui::EmbeddedFspecBackend` (RPC-008)

**File:** `codelet/fspec-tui/src/transport/embedded.rs:45`

```rust
pub fn new(handle: tokio::runtime::Handle, service: Arc<SharedFspecService>) -> Self
```

- Non-defaulted `handle: tokio::runtime::Handle` — the RPC-005 Q9
  invariant. `combined.rs` MUST source the handle from
  `tokio::runtime::Handle::current()` driven by `#[tokio::main]`; no
  separate runtime construction is permitted.

## 5. `codelet_fspec_tui::WebSocketFspecBackend` (RPC-008)

**File:** `codelet/fspec-tui/src/transport/websocket.rs:43`

```rust
pub async fn connect(url: url::Url) -> Result<Self>
```

- Takes `url::Url` (NOT `&str`) — `client.rs` must parse the resolved
  connect URL via `url::Url::parse(&s)?`. The `url` crate is already in
  workspace.dependencies (line 134 of `codelet/Cargo.toml`).

## 6. `codelet_fspec_tui::App` (RPC-008 + RPC-009)

**File:** `codelet/fspec-tui/src/app.rs`

- `App::new(backend: Arc<dyn FspecBackend>) -> Self` (line 95)
- `App::bootstrap(&mut self) -> Result<()>` (line 213) — performs the
  RPC-009 three-step bootstrap (`list_work_units` + `create_session(None)`
  + `spawn_subscriber_tasks`).
- `App::run(self) -> Result<()>` (line 392) — initializes
  `TerminalGuard::init()?` internally; the App owns the alt-screen.
  Neither `combined.rs` nor `client.rs` calls `ratatui::init()` directly.
- `App::subscriber_task_count(&self) -> usize` (line 197) — used by the
  reconnect-bootstrap assertion (example [16] / scenario "Pressing `r`
  performs a full reconnect bootstrap").
- Implication: the binary code must call `new` → `bootstrap` → `run` as
  three SEPARATE statements, not chained as `App::new(...).run()`.

## 7. Existing `codelet-rpc-server` binary contract (RPC-006)

**File:** `codelet/rpc-server/src/main.rs:55-60`

```rust
println!("{}", addr.port());
std::io::stdout().flush()?;
```

- Single line, bare integer port, newline-terminated, flushed.
- Read on the test side by `BufReader::read_line` in
  `codelet/rpc-server/tests/websocket_transport.rs::spawn_rpc_server`.
- `fspec daemon` reproduces this verbatim so the existing helper works
  unchanged against the new binary (rule [4]).

## 8. Existing source-shape regression (RPC-005 → widened RPC-008)

**File:** `codelet/rpc-embedded/tests/architecture_invariants.rs:33-72`

- Already scans `codelet/rpc-embedded/src/` and `codelet/fspec-tui/src/`
  for forbidden runtime construction calls.
- RPC-010 widens it again to include `codelet/fspec/src/` (rule [13],
  scenario "The RPC-005 source-shape invariant is widened to scan
  codelet/fspec/src/").
- `scenario_11_rpc_server_binary_binds_only_to_127_0_0_1` currently only
  scans `codelet/rpc-server/src/main.rs`. RPC-010's `fspec daemon.rs` and
  the source-shape test under `codelet/fspec/tests/source_shape.rs` will
  ALSO assert the loopback-only invariant via clap-arg validation (rule
  [21]).

## 9. Workspace members in `codelet/Cargo.toml`

Current alphabetical order:
```
cli, common, core, fspec-tui, git, napi, providers, rpc, rpc-embedded,
rpc-server, rpc-types, tools, tui
```

RPC-010 inserts `fspec` between `core` and `fspec-tui`:
```
cli, common, core, fspec, fspec-tui, git, napi, providers, rpc,
rpc-embedded, rpc-server, rpc-types, tools, tui
```

## 10. Priority enum + Compositor (RPC-008)

**File:** `codelet/fspec-tui/src/components/mod.rs:26`

- `pub enum Priority { ... Critical, ... }` — used by the disconnect
  dialog (`Priority::Critical` per rule [12] and example [8]).

## Conclusions affecting feature files

Two small corrections applied during this re-review:

1. **`fspec-binary-cargo-shape-rpc010.feature`** scenario "codelet/fspec
   is registered as a workspace member": fixed alphabetical-position
   step to correctly place `fspec` between `core` and `fspec-tui`.

2. **`fspec-binary-client-mode-rpc010.feature`** scenario "Client mode
   does not call ratatui::init directly": expanded the assertion from a
   single chained `App::new(...).run()` call to the actual three-step
   sequence (`new`, `bootstrap`, `run`).

Two architecture notes added:

- Architecture note #7: `init_tracing_daemon` ORDER GOTCHA (codelet-rpc
  global subscriber semantics).
- Architecture note #8: App ownership pattern (new → bootstrap → run).
