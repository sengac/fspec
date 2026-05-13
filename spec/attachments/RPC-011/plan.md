# RPC-011 — `fspec` rough-edge polish: reconnect, lifecycle, parity hardening

**Parent:** RPC-002
**Predecessor:** RPC-010 (`fspec` / `fspec daemon` / `fspec client` exist)

## What we want

A small "make it actually pleasant" follow-up to RPC-010 that closes the
obvious rough edges before the team starts daily-driving the new
binary. This card is intentionally separate from RPC-010 to keep that
card focused on the subcommand surface; everything here is post-MVP
polish that becomes apparent only once the binary is in real use.

## Why this card

The user flow described in the request — `fspec` / `fspec daemon` /
`fspec client` — works at the end of RPC-010, but only just. Several
known gaps would make it frustrating to use day-to-day:

- `fspec client` against a daemon that goes away offers a one-shot
  "quit or retry" dialog with no automatic reconnect.
- `fspec daemon` only handles SIGINT/Ctrl+C; SIGTERM, SIGHUP, and
  non-graceful client disconnects are unhandled.
- The daemon.json autodiscovery has no staleness check — a crashed
  daemon leaves a stale file that misroutes the next client.
- The combined `fspec` mode prints `PORT=<n>` to stderr but offers no
  ergonomic way for a sibling shell to learn that port (you have to
  parse the stderr stream).
- No tests yet for "two clients attached to one daemon at the same
  time" — multi-client semantics are claimed by the design but not
  exercised.

## Existing RPC-005..010 artifacts this card builds on

RPC-011 polishes existing surfaces. It must NOT replace `bind_and_serve`,
fork `WebSocketFspecBackend`, introduce a second `App`, or define a
parallel envelope format. Every change is additive to the structures
RPC-005 stood up.

| Existing artifact | Path / origin | How RPC-011 extends it |
|---|---|---|
| `WebSocketFspecBackend::connect(url)` | `codelet/fspec-tui/src/transport/websocket.rs` (RPC-008) | Wraps the existing connect in a reconnect supervisor task. The supervisor owns an `Arc<RwLock<Option<FspecWsClient>>>`; on disconnect it re-runs `tokio_tungstenite::connect_async` with backoff. `FspecBackend` impl methods grab a read-lock and delegate; if the client is `None` they return `Err(BackendError::Disconnected)` so the UI can render the auto-reconnect dialog. The `ws_client_connect` factory function is itself unchanged. |
| `bind_and_serve(addr, service) -> (SocketAddr, ServerStats, JoinHandle)` | `codelet/rpc-server/src/server.rs` (RPC-005) | Signature unchanged. The returned `JoinHandle` (currently used only for "abort to shut down") is now used by `daemon.rs` for the graceful drain: on SIGTERM, the daemon calls `handle.abort()` after the in-flight RPCs finish and broadcasts the `ServerGoingAway` envelope. No new bind function. |
| `Envelope` enum | `codelet/rpc-server/src/envelope.rs` (RPC-005) | Optionally adds a `ServerGoingAway` variant. Alternative: piggyback on `tokio_tungstenite::tungstenite::protocol::CloseFrame` with reason `"going_away"` — preferred because it does NOT require a wire-format bump. Decision made during Example Mapping. |
| `ServerStats` (`rejected_envelopes`, `rejected_variants`, `service`) | `codelet/rpc-server/src/lib.rs:46-71` (RPC-005) | Extended with `connected_clients: AtomicU64`, `last_watcher_event_at: Mutex<Option<Instant>>`, and per-broadcast lag counters. The new `health()` RPC reads these. No new struct — `ServerStats` is the existing channel. |
| `FspecService` tarpc trait | `codelet/rpc/src/lib.rs:25-28` (RPC-005, extended in RPC-007) | Adds one method: `async fn health() -> HealthInfo`. `HealthInfo` is a new lifted type in `codelet/rpc-types` following the established cfg-gated `napi(object)` pattern. |
| `FspecBackend` trait | `codelet/fspec-tui/src/transport/mod.rs` (RPC-008) | Adds `async fn health(&self) -> Result<HealthInfo>`. Both `EmbeddedFspecBackend` and `WebSocketFspecBackend` implement it; embedded reads `ServerStats` directly via `self.service`, WS goes through the tarpc client. |
| `daemon.json` autodiscovery | `codelet/fspec/src/common.rs` (RPC-010) | RPC-010 wrote `{ url, port }`. RPC-011 adds `{ pid, started_at, version }`. `fspec client` and `fspec status` BOTH gain a `verify_daemon_alive(pid)` step that uses `nix::sys::signal::kill(pid, None)` (or platform equivalent) before trusting the URL; on failure they delete the file and treat as no-daemon. |
| Action enum + Compositor | `codelet/fspec-tui/src/app/{action,compositor}.rs` (RPC-008/009) | Adds `Action::Reconnecting(attempt: u32)`, `Action::Reconnected`, `Action::ServerGoingAway`. The disconnect dialog from RPC-010 grows reconnect-state rendering — same `Priority::Critical` dialog component, no new widget. |
| Broadcast channel sizing | created in RPC-006/007 inside `SharedFspecService` | Promoted from `tokio::sync::broadcast::channel(default)` to explicit capacities (work-units 256, chunks-per-session 1024, logs 4096). `tracing::warn!` on `RecvError::Lagged` already comes for free; the warning surfaces in `LogEvent` envelopes (RPC-007) so connected clients see lag in real time. |
| `codelet-rpc-server` binary | `codelet/rpc-server/src/main.rs` (RPC-005, replaced as production entry by RPC-010 `fspec daemon`) | Stays as a development helper. NOT updated for SIGTERM/SIGHUP handling — that goes into `fspec daemon` only. |
| RPC-005 source-shape tests | `codelet/rpc-embedded/tests/architecture_invariants.rs` | All still pass: types still defined exactly once; embedded still requires `Handle`; rpc-server still binds 127.0.0.1; rpc crate has no `codelet-core` dep. RPC-011 does NOT loosen any of these. |
| Vitest smoke | `src/__tests__/napi-workunitinfo-shape.test.ts` + the watcher/streaming smokes from RPC-006/007 | All still pass. NAPI's listener-on-broadcast pattern (RPC-007) means broadcast capacity tuning here cannot break it as long as listeners drain fast enough. |

## Architecture conformance with RPC-002

| RPC-002 decision / pattern | Source | RPC-011 obligation |
|---|---|---|
| Disconnect / reconnect dialog | doc 11 §Q5 (resolved: tui-popup) + doc 09 §B | The "auto-reconnecting…" dialog is a thin Component that wraps `tui_popup::Popup` via the SAME `Dialog` widget RPC-008 introduced. NO new dialog framework; NO `rat-dialog`. The dialog is pushed at `Priority::Critical` so it captures all keystrokes (j/k stay no-op, only `r` and `q` are honoured) — exactly the contract from doc 09 §A.6. |
| `mpsc::UnboundedSender<Action>` action bus | doc 07 §4 | The reconnect supervisor task feeds the App via `Action::Reconnecting(attempt)`, `Action::Reconnected`, `Action::ServerGoingAway` — no direct mutation of dialog state from outside the App task. Pattern matches doc 06 §Async work. |
| Codex inline-mode pattern is NOT used | doc 04 §1, doc 01 §"Codex is not the model to copy" | The reconnect dialog renders inside the alt-screen canvas; it does NOT scroll into terminal scrollback. |
| Bare ratatui (Q3) + Compositor (Q2) | doc 11 §Q2/Q3 | The reconnect supervisor lives at the transport layer (`codelet/fspec-tui/src/transport/websocket.rs`); the dialog is the only UI artifact and is a regular `Component`. No framework upgrade. |
| Mouse-tracking 5-second debounce (deferred from RPC-008) | doc 02 §2.3, doc 12 §Slice 02 | Still deferred. `MouseTrackingToggle` arrives in the VirtualList card (Slice 03/04) where text-selection passthrough actually matters. |
| `tracing_subscriber::Layer` for `LogEvent` broadcast | introduced RPC-007 | The new health metrics — connected clients, last watcher event, broadcast lag — feed via `tracing::info!`/`warn!` calls and ride the existing layer into the broadcast channel. No new event pipeline. |

## Scope

### Reconnect & connection lifecycle

- `WebSocketFspecBackend` gains exponential backoff reconnect (250 ms,
  500 ms, 1 s, 2 s, 5 s cap; reset on first successful frame).
- On reconnect: re-issue `list_work_units` + `list_sessions` + resubscribe
  to push channels; emit a `Reconnected` `Action` so the TUI can flash
  a status indicator.
- Disconnect dialog from RPC-010 becomes "auto-reconnecting (attempt
  N)…" with manual `r` (retry now) and `q` (quit) options.

### Daemon hardening (extends RPC-005 rule [11])

- Handle SIGTERM, SIGINT (already covered), and SIGHUP (re-read
  workspace path).
- Graceful drain on shutdown: stop accepting new connections, finish
  in-flight RPCs, send a `ServerGoingAway` envelope frame to all
  connected clients (new envelope variant in this card or piggyback on
  WS Close), then exit.
- `daemon.json` handshake gains a `pid` field; clients verify the pid
  is alive before trusting the URL, otherwise they delete the stale
  file and treat it as no-daemon.
- Optional `--foreground` flag (default true) and a real
  fork-and-detach mode behind `--background` for ad-hoc shell use.

### Multi-client & broadcast hardening

- Backpressure: the broadcast channels chosen in RPC-006/007 use
  default `tokio::sync::broadcast` capacity — explicitly size them
  (e.g. 1024 chunks per session, 256 work-units snapshots, 4096 logs)
  with `tracing::warn!` on lag.
- Two-client integration test: spawn `fspec daemon`, attach two
  `WebSocketFspecBackend`s simultaneously, exercise create_session +
  send_input from one, verify both clients see the same chunk stream
  in order.
- Read-only second-attach mode: NOT in this card — flagged as a future
  question (mirrors RPC-002 doc 11 multi-client risk).

### Observability

- `fspec daemon` exposes a `health` RPC (added to `FspecService`):
  process uptime, connected clients, last watcher event timestamp,
  broadcast lag counters.
- `fspec status` subcommand: pretty-print `health` against the
  autodiscovered or `--connect`'d daemon. Exits 0 if alive, 1 if no
  daemon found.
- Tracing spans annotated with `client_id` and `session_id` for filter
  ergonomics.

### Tests

- Reconnect: spawn daemon, connect client, kill daemon, restart daemon,
  assert client reconnects within 5 s and emits `Reconnected`.
- Multi-client: described above.
- Stale daemon.json: write a daemon.json pointing at a dead pid, run
  `fspec client`, assert it deletes the file and falls back gracefully.
- SIGTERM: send SIGTERM to running `fspec daemon`, assert clients
  receive `ServerGoingAway` (or WS Close) before the process exits.

## Out of scope

- Authentication / TLS / non-loopback binds — own card.
- Read-only / observer-mode multi-client semantics — own card.
- Replacing the TS `fspec` on npm install — distribution card.
- Cancellation propagation via tarpc `context::Context` — own card.

## Acceptance — done when

1. `WebSocketFspecBackend` automatically reconnects.
2. `fspec daemon` handles SIGINT, SIGTERM, SIGHUP and drains
   gracefully.
3. daemon.json includes pid; stale entries are detected and pruned.
4. `fspec status` works.
5. Multi-client and reconnect integration tests pass.
6. All earlier RPC-00x tests still pass; NAPI smoke still passes.

## Estimate guidance

5 points. Mostly small fixes; the multi-client integration test and
the reconnect state machine are the heaviest pieces.

## Note from RPC-010 review (2026-05-11)

The RPC-010 review pass (see `spec/attachments/RPC-010/review-findings.md`,
finding **CR-1**) flagged that rule [12]/[25] from RPC-010's example
map — "daemon goes away → disconnect dialog with `r` to reconnect /
`q` to quit" — was satisfied only by a Q2-style daemon-side surrogate
test ("client subprocess didn't crash after daemon kill"). The actual
UI artifacts are missing from `codelet/fspec-tui/`:

- No "daemon disconnected" string anywhere in the crate.
- `WebSocketFspecBackend` has no disconnect signal — when the WS drops,
  RPC calls just return `Err`; nothing emits an `Action`.
- No `r`-key handler bound to a reconnect path.
- No `DisconnectDialog` component (nor any other `Priority::Critical`
  dialog from the dialog framework).

**This is not a new finding for RPC-011 — it is a precondition the
RPC-011 plan above assumes already exists.** Specifically:

- Scope §"Reconnect & connection lifecycle" says "Disconnect dialog
  from RPC-010 *becomes* 'auto-reconnecting…'". The baseline dialog
  must be built first.
- The "Action enum + Compositor" row in the conformance table says
  "The disconnect dialog from RPC-010 *grows* reconnect-state
  rendering — same `Priority::Critical` dialog component, no new
  widget." Same baseline assumption.

**RPC-011 absorbs CR-1 implicitly** — the reconnect supervisor and
auto-reconnect dialog cannot exist without first building the
baseline disconnect signal + dialog + `r`/`q` handlers. No new card
is needed. Implementors should treat the baseline as the first sub-
slice of this card, before layering exponential backoff and
`ServerGoingAway` on top.
