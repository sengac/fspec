# RPC-002 — Distributed Rust Frontend (tarpc + ratatui): Feasibility & Architecture

> **Status:** Backlog. This document is exploratory. No rules, examples, or
> scenarios have been committed yet — the work unit must be broken down
> into smaller stories through Example Mapping before any implementation
> begins.

## 1. Goal

Stand up a Rust-based ratatui frontend for fspec/codelet that talks to the
existing codelet functionality through a single tarpc service trait. The
trait must be reachable through **two interchangeable transports**:

1. **Embedded** — the ratatui client and the codelet service are compiled
   into the same binary and communicate through an in-memory tarpc channel
   (no network, no serialization on the hot path).
2. **Remote** — the ratatui client connects to a long-running `fspec-daemon`
   process over a WebSocket; the daemon hosts codelet and exposes the same
   tarpc trait.

The TypeScript + Ink + NAPI stack stays fully operational throughout. The
ratatui client is built up screen by screen alongside the existing TUI and
only replaces it when parity is reached and explicitly accepted.

## 2. Why this is feasible (current-state context)

The fspec / codelet repository is already most of the way there:

- `codelet/` is an existing Rust workspace with `cli`, `common`, `core`,
  `git`, `napi`, `providers`, `tools`, `tui` crates.
- `codelet/tui/` is a thin crossterm-based crate — the slot for a richer
  UI library (ratatui) is already reserved.
- `tokio-tungstenite` is already a workspace dependency (`BRIDGE-001`),
  and `bridge/relay-server.ts` already speaks a WebSocket envelope format
  compatible with this design.
- The existing NAPI surface is large but very tarpc-shaped:
  - ~191 exported functions
  - ~76 exported interfaces / types
  - The hot type — `StreamChunk` — is already a tagged enum that
    serde-derives cleanly with no NAPI-specific concerns.
- Crates required by tarpc (`tokio` full, `serde`, `tracing`, `futures`)
  are already pulled in.

There is no green-field cost to setting up the workspace; this is purely
new-crate work inside an existing layout.

## 3. Mapping the NAPI surface to tarpc

Roughly partitioning the ~191 NAPI functions:

| Bucket | Approx. count | Fits tarpc? |
|---|---:|---|
| Pure request/response, sync (`blocklistCheck`, `glob`, `astGrepSearch`, `gitStatus`, …) | ~120 | Yes — drop-in `async fn rpc(...) -> R` |
| Promise-returning async (`sessionCompact`, `claudeOauthBrowserLogin`, `astGrepRefactor`) | ~50 | Yes — drop-in `async fn` |
| Callback-driven streaming (`sessionSetGlobalChunkCallback`, `setRustLogCallback`, `startWorkUnitsWatcher`) | ~5 | **No** — tarpc is single-shot req/res; needs a parallel push channel |
| Reverse callbacks (`callFspecCommand` — Rust calls back into the host) | ~2 | **No** — modeled separately |

The first two buckets (≈170 functions) translate mechanically. The last
two need an explicit transport-level design (see §5).

## 4. Proposed crate layout

All new crates live in the existing `codelet/` workspace:

```
codelet/
├── rpc-types/       # NEW: pure-serde types (StreamChunk, WorkUnitInfo, …)
├── rpc/             # NEW: #[tarpc::service] trait FspecService { … }
├── rpc-server/      # NEW: daemon binary; impl FspecService over codelet-core
├── rpc-embedded/    # NEW: in-process tarpc transport + impl FspecService
├── tui-client/      # NEW: ratatui app; transport-agnostic over FspecService
├── napi/            # UNCHANGED: existing NAPI bindings for the TS frontend
├── core/ providers/ tools/ git/ session_manager/ …  # UNCHANGED business logic
```

`rpc-types` is the seam: it lifts the existing NAPI structs (`StreamChunk`,
`WorkUnitInfo`, `SessionInfo`, `ToolCallInfo`, …) into a NAPI-free crate
with pure `serde` derives. `codelet-napi` then re-exports / wraps them.
`rpc` declares `#[tarpc::service] trait FspecService { … }` whose
signatures mirror the NAPI surface 1:1 but use `rpc-types` types.

Both `rpc-server` (remote daemon) and `rpc-embedded` (in-process) provide
implementations of `FspecService` that delegate to the same shared
business logic in `codelet-core`/`session_manager`/`tools`/etc.

`tui-client` is the ratatui app; it depends only on the `rpc` trait and
on a small `Transport` abstraction that resolves to either
`rpc-embedded::EmbeddedTransport` or
`rpc-remote-ws::WebSocketTransport`.

## 5. Transport architecture

### 5.1 Embedded mode

```
┌────── codelet binary (single process) ──────┐
│  ratatui main loop                          │
│       │                                     │
│       ▼ FspecServiceClient (tarpc)          │
│  in-memory channel (tarpc::transport)       │
│       ▲                                     │
│       │ FspecServiceServer                  │
│  codelet-core / session_manager / tools     │
└─────────────────────────────────────────────┘
```

This is exactly the pattern in `tarpc/example-service` but using
`tarpc::transport::channel::unbounded()` instead of TCP. No serialization
on the hot path — request/response are passed by value across an
in-memory `Stream`/`Sink`.

For events (chunks/logs/work-unit watcher), the embedded mode just
hands the ratatui app a `tokio::sync::broadcast::Receiver<StreamChunk>`
directly; no envelope multiplexing needed.

### 5.2 Remote mode

```
┌── ratatui client ──┐         ┌── fspec-daemon ──┐
│ FspecServiceClient │         │ FspecServiceImpl │
│         │          │         │         │        │
│         ▼          │   WS    │         ▲        │
│   Envelope mux ────┼────────▶│   Envelope demux │
│         ▲          │ binary  │         │        │
│         │          │ frames  │         ▼        │
│   broadcast::rx    │◀────────│  StreamChunk fan-out
│   (chunks/logs)    │         │  (broadcast)     │
└────────────────────┘         └──────────────────┘
```

The WebSocket carries a single tagged envelope:

```rust
enum Envelope {
    Rpc(tarpc::Request | tarpc::Response),
    Event(StreamChunk { session_id, chunk }),
    LogEvent(LogRecord),
    WorkUnitsUpdate(Vec<WorkUnitInfo>),
    CmdReq(FspecCommandRequest),   // reverse callback
    CmdRes(FspecCommandResult),
}
```

A small adapter splits incoming frames:
- `Rpc(_)` is forwarded into a tarpc transport (the standard
  `Stream<Item=Request> + Sink<Response>` pair tarpc consumes — see
  `tarpc/src/serde_transport.rs:90`).
- `Event(_)` / `LogEvent(_)` / `WorkUnitsUpdate(_)` are pushed onto
  `tokio::sync::broadcast::Sender`s that the ratatui app subscribes to.
- `CmdReq(_)` / `CmdRes(_)` are correlated by ID for the reverse-callback
  case.

Wire format defaults to **bincode** for performance; a JSON envelope
mode is available behind a feature flag for debugging and for non-Rust
clients (Telegram bridge, mobile app).

### 5.3 Why this split (vs. encoding events as RPC return streams)

tarpc is strictly single-shot req/res — it has no `stream Foo()` like
gRPC. Confirmed in `/tmp/tarpc/tarpc/src/lib.rs`. Multiplexing events as
a sibling channel rather than forcing them through tarpc is the
established pattern: tarpc keeps its clean cancellation/deadline
semantics, and event traffic gets back-pressured `broadcast` channels
sized appropriately for chunk volume.

## 6. The session-singleton concern

`SessionManager` is a process-wide singleton (`SessionManager::instance()`)
holding live tokio tasks per session, in-memory streaming state,
ChainOfCommand graph, role overlays, pause state, etc.

- **Embedded:** the singleton is co-located with the ratatui app; no
  ownership change.
- **Remote:** the singleton lives in `fspec-daemon`. Multiple ratatui
  clients (or other clients — Telegram bridge, mobile app, IDE
  extension) can attach to the same session. The current callback model
  (`session_set_global_chunk_callback`) is single-listener; in the
  daemon it becomes a `tokio::sync::broadcast::Sender<(SessionId, StreamChunk)>`
  with one subscriber per attached client.

Lifecycle questions to resolve in subsequent work units:

- Daemon-per-project (CWD-scoped) vs. daemon-per-user with project
  routing.
- Daemon lifetime: spawn on first client connect / shut down on last
  disconnect, or persistent system service.
- Authentication / authorization for remote clients (out of scope of
  this document — flagged as a future work unit).

## 7. OAuth flows in remote mode

The `claudeOauthBrowserLogin` / `codexOauthBrowserLogin` /
`copilotOauthDeviceLoginStart` family runs a loopback HTTP server and
opens the user's browser. In a remote daemon scenario, the daemon's
host might not be the user's host. The NAPI surface already exposes a
**headless** flow (`claudeOauthHeadlessStart` / `headlessComplete`) for
exactly this case — the daemon returns the auth URL, the user opens it
locally, and pastes the redirect code back through the client. Remote
mode uses the headless flow; embedded mode uses the full browser-loopback
flow as today.

## 8. Reverse callbacks (`callFspecCommand`)

Today the Rust side calls back into Node.js via NAPI to execute fspec
subcommands. Two viable patterns for the tarpc world:

1. **Two services, both directions.** The client implements a tiny
   `FspecExecutor` tarpc trait that the daemon calls into. Cleanest
   semantics, slightly more boilerplate.
2. **CmdReq/CmdRes envelope frames with correlation IDs.** Reuses the
   same WS, no second tarpc trait. Simpler crate graph.

Pick one when this work unit is broken down. Embedded mode collapses
this to a direct function call; only remote mode needs the protocol.

## 9. Migration plan (incremental, screen-by-screen)

The TS+Ink+NAPI stack stays running the entire time. Each milestone is
a separate work unit, independently shippable behind a CLI flag like
`--frontend=ratatui`:

1. **Phase 0 — Spike.** WS+tarpc transport adapter + a 5-RPC service
   trait + a single-screen ratatui client showing the work-unit list.
   Validates the design end-to-end. Throwaway code, time-boxed (~3-5
   days).
2. **Phase 1 — `rpc-types` crate.** Lift NAPI structs into a
   serde-only crate. NAPI re-exports them. Zero behaviour change for
   the TS frontend.
3. **Phase 2 — `rpc` trait.** Mirror the NAPI surface 1:1 in a
   `#[tarpc::service]` trait. Mostly mechanical.
4. **Phase 3 — `rpc-embedded` + `rpc-server`.** Both implement the
   trait by delegating to existing business logic. Daemon binary
   listens on WebSocket; embedded transport is wired into a stub
   ratatui app.
5. **Phase 4 — Read-only board view.** Port `UnifiedBoardLayout.tsx`
   (517 lines) to ratatui. Read-only — no mutations. Keyboard
   navigation, work-unit details panel, board column scrolling. Both
   transports working.
6. **Phase 5 — AgentView REPL.** Port the core REPL loop from
   `AgentView.tsx` (5,624 lines): subscribe to chunks, render the
   conversation, accept input, send to session. The complex modals
   (model selector, provider settings, file-search popup, attachment
   dialog) are *not* in scope here — they get their own later work
   units.
7. **Phase 6+ — Surface coverage.** Each remaining screen is its own
   work unit: model selector, provider settings, checkpoints, diff
   viewer, multiline input, slash commands, file search popup,
   attachments, etc. Order driven by user value.
8. **Phase 7 — Cutover.** When the ratatui frontend reaches parity,
   the Ink TUI is retired. The NAPI crate may stay for non-fspec
   embedders (e.g. external Node tooling) but the fspec CLI defaults
   to ratatui.

## 10. Effort estimate (rough, pre-Example-Mapping)

| Phase | Risk | Order-of-magnitude |
|---|---|---|
| 0. Spike (WS transport, 5 RPCs, 1 screen) | Low | 3–5 days |
| 1. `rpc-types` crate | Low | 2–3 days |
| 2. `rpc` trait (~191 fns mirrored) | Low (mechanical) | 3–5 days |
| 3. Embedded + daemon servers | Medium | 1–2 weeks |
| 4. Read-only board view (ratatui) | Medium | 1 week |
| 5. AgentView REPL minimum | Medium-High | 1–2 weeks |
| 6. Remaining screens to parity | High (volume) | 3–6 weeks |
| 7. Cutover & deprecation | Medium | 1 week |

The bottleneck is **not** the RPC plumbing — it is reproducing a rich
React/Ink component tree (~40+ components, including non-trivial inputs
and modal compositions) in ratatui. ratatui has no equivalent of
React's state model or Ink's focus management; we will have to develop
a small in-house "ratatui-forms" pattern early in Phase 5.

## 11. Risks & open questions (for Example Mapping later)

These are deliberately listed as open questions, not as decisions:

- **Daemon topology.** Per-project vs. per-user? Persistent service vs.
  on-demand spawn?
- **Authentication.** Does remote mode require auth? Token-based?
  Local-socket-only by default with explicit opt-in for network
  binding?
- **Multi-client semantics.** When two clients attach to the same
  session, what does input arbitration look like? Read-only observer
  mode?
- **Distribution.** Pure-Rust client breaks the npm install story.
  `cargo install fspec`? Signed binaries per platform? Reuse the
  existing SEA packaging?
- **Test strategy.** `vitest` integration tests against NAPI go away
  for the ratatui frontend. Replace with `tokio::test` daemon+client
  tests; harness the ratatui app with `ratatui::TestBackend`.
- **ratatui state management.** Pick a discipline up front — a
  redux-shaped enum + `tokio::sync::watch`, or a more ad-hoc model.
- **Cancellation semantics.** tarpc cancels server work on client
  drop via `context::Context`. Ensure this maps cleanly onto
  `session_interrupt`.
- **Telemetry.** tarpc instruments with `tracing` + OpenTelemetry; do
  we want to ship this on by default in the daemon for production
  debugging?

## 12. What this work unit explicitly does NOT do

- It does not delete anything from `codelet-napi`.
- It does not change the TypeScript Ink TUI.
- It does not commit to a daemon topology or auth model.
- It does not promise wire-format stability across crates that don't
  share a Cargo workspace version.

## 13. Reference material checked while writing this

- `/tmp/tarpc/` (cloned) — tarpc 0.37; `serde_transport.rs`,
  `example-service/`. Confirmed: tarpc is single-shot req/res, no
  built-in stream return, transport is any `Stream<Item=Request> +
  Sink<Response>`.
- `/tmp/ratatui/` (cloned) — ratatui workspace; example apps cover
  most of what we'd need (`async-github`, `input-form`, `demo2`).
- `codelet/Cargo.toml` — confirmed `tokio-tungstenite`, `tokio` full,
  `serde`, `tracing` already present.
- `codelet/napi/index.d.ts` — 2,625 lines, ~191 `export declare
  function` entries, ~76 `export interface` entries. Streaming
  callbacks: `sessionSetGlobalChunkCallback`, `startWorkUnitsWatcher`,
  `setRustLogCallback`. Reverse callback: `callFspecCommand`.
- `codelet/napi/src/lib.rs` — confirmed module layout and
  `noop`-feature gating (already a precedent for splitting NAPI from
  pure-Rust core).
- `src/tui/components/UnifiedBoardLayout.tsx` (517 lines) — Phase 4
  target.
- `src/tui/components/AgentView.tsx` (5,624 lines) — Phase 5+ target;
  size confirms why "AgentView REPL minimum" is its own multi-week
  work unit and modals are deferred.
