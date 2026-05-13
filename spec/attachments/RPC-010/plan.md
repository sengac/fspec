# RPC-010 — `fspec` binary: combined frontend+server, plus `fspec daemon` and `fspec client` subcommands

**Parent:** RPC-002
**Predecessor:** RPC-009 (basic UI)
**Successor:** RPC-011 (rough edges + production hardening)

## What we want

A single Rust binary called **`fspec`**, built by `cargo build` inside
`codelet/`, that the user runs three ways:

1. `fspec` — start in **combined mode**: the embedded shared service
   AND the WebSocket RPC server AND the ratatui frontend, all in one
   process. The WebSocket server is **always on** in this mode (rule
   from the user request) so external clients (or the chrome extension,
   or the bridge, or the next `fspec client` invocation) can attach.
2. `fspec daemon` — start in **headless mode**: shared service +
   WebSocket server only, no terminal UI. Suitable for systemd /
   launchd / dev-container background process.
3. `fspec client` — connect to an already-running daemon and start the
   ratatui frontend pointed at its WS endpoint. No embedded service in
   this process — pure WebSocket client.

This card is the moment the user-facing path the request describes
finally exists. From now on `codelet/target/release/fspec` is the
artifact that progressively replaces the TS `fspec` CLI.

## Why this card

- All the pieces exist after RPC-009: a backend trait, two transport
  implementations, a TUI app, a WS server binary
  (`codelet-rpc-server`), a real watcher.
- The user's mental model is one `fspec` binary with three modes; the
  internals are already three separate crates. A small CLI multiplexer
  in a new `codelet/fspec/` crate ties them together.
- Naming: long-term we want `codelet/target/release/fspec` to *be* the
  fspec binary — not `codelet-rpc-server` and not `codelet`. Now is the
  right moment to claim that name.

## Existing RPC-005..009 artifacts this card builds on

RPC-010 is the wiring card: it composes the bricks from the previous
five cards into a single binary. It must NOT introduce a second WS
server, a second tarpc trait, a second backend, or a second app shell.
The `codelet-rpc-server` binary from RPC-005 is REPLACED by `fspec
daemon` — it is allowed to remain as a development helper but is no
longer the production entry point.

| Existing artifact | Path / origin | How RPC-010 uses it |
|---|---|---|
| `bind_and_serve(addr, service) -> (SocketAddr, ServerStats, JoinHandle)` | `codelet/rpc-server/src/server.rs` (RPC-005) | Called directly from `combined.rs` AND from `daemon.rs`. Both modes use the SAME function — there is no "embedded server" vs "daemon server" code path. The `ServerStats` returned is what `fspec status` (RPC-011) and the future health RPC report on. |
| `SharedFspecService::new(watcher)` | `codelet/rpc/src/lib.rs:36-59` (constructor reshaped in RPC-006) | Constructed exactly once per `fspec` invocation, in `common.rs::build_service(workspace)`. The same `Arc<SharedFspecService>` is passed to `bind_and_serve` AND to `EmbeddedFspecBackend::new` in combined mode. Single-instance invariant from RPC-005 is preserved at the binary boundary. |
| `EmbeddedTransport` / `EmbeddedFspecBackend` | `codelet/rpc-embedded/src/lib.rs` (RPC-005) + `codelet/fspec-tui/src/transport/embedded.rs` (RPC-008) | `combined.rs` builds `EmbeddedFspecBackend::new(tokio::runtime::Handle::current(), service.clone())` and hands it to `App::new`. The `Handle` is passed explicitly per RPC-005 Q9 — the binary owns the runtime via `#[tokio::main]`. |
| `WebSocketFspecBackend::connect(url)` | `codelet/fspec-tui/src/transport/websocket.rs` (RPC-008) | `client.rs` calls this and passes the result to `App::new`. No envelope handling here — RPC-008 already encapsulated it. |
| `App::new(backend).run()` | `codelet/fspec-tui/src/app/mod.rs` (RPC-008) + views from RPC-009 | Same `App` for combined mode (with `EmbeddedFspecBackend`) and client mode (with `WebSocketFspecBackend`). Daemon mode does NOT instantiate `App` at all — it just blocks on signals. |
| Existing `codelet-rpc-server` binary main.rs (port-line contract) | `codelet/rpc-server/src/main.rs` | `daemon.rs` reproduces the same stdout port-line contract (single line, `<n>\n`, flushed) so existing test harnesses work unmodified. `combined.rs` writes the SAME line to STDERR instead, plus a `daemon.json` file. The original binary's `tracing` setup pattern is also lifted into `common.rs`. |
| `WorkUnitsWatcher` (lifted in RPC-006) | `codelet/core/src/work_units.rs` | `common.rs::build_service` constructs the watcher rooted at `--workspace` (default CWD). All three modes share this code path. |
| `tracing_subscriber::Layer` populating `LogEvent` broadcast | introduced in RPC-007 | `common.rs::init_tracing` registers it once for combined and daemon modes (the WS server fans out logs to clients). Client mode registers a passthrough subscriber that ALSO writes logs to `~/.fspec/client.log` — it has no broadcast sender of its own. |
| `ServerStats` rejection counters | `codelet/rpc-server/src/lib.rs` (RPC-005) | `fspec status` (RPC-011) reads these via the future `health` RPC. Wiring is a no-op in this card — the stats already exist, RPC-010 just keeps the handle alive in `combined.rs`/`daemon.rs` so RPC-011 can attach to it. |
| `Envelope` framing + reserved variants | `codelet/rpc-server/src/envelope.rs` (RPC-005, extended in RPC-006/007) | Untouched. Loopback-only bind invariant (RPC-005 architecture rule [13]) is preserved by `--bind` defaulting to `127.0.0.1:0`; non-loopback support is explicitly out of scope. |
| Vitest smoke + RPC-005 source-shape tests | `src/__tests__/napi-workunitinfo-shape.test.ts`, `codelet/rpc-embedded/tests/architecture_invariants.rs` | Both still pass after this card. `architecture_invariants.rs` source-shape test gains: `codelet/fspec/Cargo.toml` does NOT depend on `codelet-napi` (the production binary path is NAPI-free), and `codelet/fspec/src/` contains no `tokio::runtime::Builder` / `Runtime::new` calls (#[tokio::main] is the only runtime). |

## Architecture conformance with RPC-002

This card composes existing pieces; it does not introduce new
ratatui-side patterns. The only architectural concern is making sure
`fspec` (combined mode) does not corrupt the alt-screen canvas with
log lines or port banners.

| RPC-002 decision / pattern | Source | RPC-010 obligation |
|---|---|---|
| Q1: alt-screen mode | doc 11 §Q1 | Combined mode and client mode both call `ratatui::init()` (alt-screen + raw mode). Daemon mode does NOT. |
| Terminal restoration on panic | doc 07 §2 | All three modes install a `std::panic::set_hook` that calls `ratatui::restore()` plus `crossterm::execute!(stdout, DisableMouseCapture, DisableBracketedPaste)` BEFORE panicking. Same hook is wired in `Drop` on the terminal guard owned by `App` (already established in RPC-008). |
| Log routing must not touch alt-screen | doc 07 §6 implication, doc 12 §cross-cutting | `init_tracing` in combined mode routes `tracing` events to a `tracing_appender::rolling` file under `~/.fspec/logs/` AND to the `LogEvent` broadcast inside `SharedFspecService` (RPC-007). It does NOT register a stderr / fmt subscriber — stderr in combined mode is reserved for the single `PORT=<n>` line and panic backtraces. Daemon mode keeps a stderr fmt subscriber (no TUI to corrupt). |
| Stdout/stderr split | RPC-005 binary contract (`codelet/rpc-server/src/main.rs`) | Daemon mode keeps the existing single-line stdout port contract verbatim so existing `ChildGuard` + `BufReader::read_line` test harnesses (`codelet/rpc-server/tests/websocket_transport.rs::spawn_rpc_server`) work without modification on the new binary. Combined mode emits the same line on stderr; client mode emits nothing on either stream. |
| `mpsc::UnboundedSender<Action>` action bus | doc 07 §4 | Already in `codelet/fspec-tui` from RPC-008. `combined.rs` and `client.rs` create the App via `App::new(backend)`; they do not see the action bus directly. |
| Q9: embedded transport requires host runtime `Handle` | doc 11 §Q9, RPC-005 invariant | `combined.rs` uses `tokio::runtime::Handle::current()` (driven by `#[tokio::main]`). NEVER `tokio::runtime::Builder` or `Runtime::new`. The RPC-005 source-shape regression is widened in this card to scan `codelet/fspec/src/` for forbidden runtime construction. |
| daemon vs in-process server | RPC-005 §Embedded mode (single-process pattern); doc 12 §cross-cutting "fspec-tui standalone binary entry point" | A SINGLE `bind_and_serve` function from `codelet/rpc-server` (RPC-005) is used by both modes. There is NO second WS server, no parallel daemon binary in production. The original `codelet-rpc-server` binary stays as a development helper but is no longer the production entry point. |
| Distribution / npm swap | doc 12 §cross-cutting "ratatui standalone binary entry point" + RPC-002 §12 (does not delete codelet-napi) | Out of scope for this card. The TS Ink TUI keeps running on the existing npm path; `cargo build -p fspec --release` produces a parallel artifact at `codelet/target/release/fspec` and an `npm run build:rust:fspec` script copies it to `dist/fspec` for hand testing. The npm `bin` entry stays on the TS shim. |

## Scope

### New binary crate `codelet/fspec/`

```
codelet/fspec/
  Cargo.toml          # [[bin]] name = "fspec", path = "src/main.rs"
  src/
    main.rs           # clap dispatcher
    combined.rs       # default mode: server + frontend
    daemon.rs         # headless mode
    client.rs         # remote-frontend mode
    common.rs         # shared startup: tracing, panic hook, workspace detection
```

Workspace member added; no other crate changes its name.

### CLI surface (clap)

```
fspec [--workspace <path>]
fspec daemon [--bind 127.0.0.1:0] [--workspace <path>] [--pidfile <path>]
fspec client [--connect ws://127.0.0.1:<port>] [--workspace <path>]
fspec --version
fspec --help
```

Notes:
- `--workspace` defaults to CWD; the `WorkUnitsWatcher` is rooted there.
- `--bind` for `daemon` defaults to `127.0.0.1:0` (RPC-005 invariant —
  loopback only). Configurable bind address is allowed in this card
  (escapes the RPC-005 hardcode), but auth and non-loopback binds are
  still future-card territory.
- `--pidfile` for `daemon` writes the pid + ephemeral port to a file so
  scripts (and `fspec client` autodetect, see below) can find it.
- `--connect` for `client` defaults to reading `~/.fspec/daemon.json`
  (or `$XDG_RUNTIME_DIR/fspec/daemon.json`) so plain `fspec client`
  works without arguments when a daemon is running.

### Mode internals

**`fspec` (combined):**
1. Detect tokio runtime; build a `WorkUnitsWatcher` rooted at workspace.
2. Construct `Arc<SharedFspecService>` from the watcher.
3. Spawn the WS server task with `bind_and_serve("127.0.0.1:0", service)`.
4. Print `PORT=<n>` to stderr (NOT stdout — stdout would corrupt the
   alt-screen TUI). Also write `~/.fspec/daemon.json` so a sibling
   `fspec client` works.
5. Construct `EmbeddedFspecBackend(handle, service)`.
6. `App::new(Arc::new(embedded_backend)).run().await`.
7. On exit: tear down WS server cleanly, remove daemon.json.

**`fspec daemon`:**
1. Same steps 1–4 above (port goes to stdout this time, ready for shell
   capture by sysv-style supervisors).
2. Block on `tokio::signal::ctrl_c()` + SIGTERM (extending RPC-005's
   ctrl_c-only handling).
3. On exit: clean up pidfile.

**`fspec client`:**
1. Resolve `--connect` (explicit URL or daemon.json autodiscovery).
2. Construct `WebSocketFspecBackend(url).await`.
3. `App::new(Arc::new(ws_backend)).run().await`.
4. On WS disconnect mid-session: surface a critical-priority dialog
   "daemon disconnected — q to quit, r to reconnect". Reconnect logic
   is best-effort in this card (full retry/backoff is RPC-011).

### Build artifact

`cargo build -p fspec --release` produces `codelet/target/release/fspec`.
Add a top-level `Makefile` or `npm` script (`build:rust:fspec`) that
runs the cargo build and copies the artifact to `dist/fspec` for
parity with the existing TS dist layout, but does NOT yet replace the
TS CLI on the npm install path — that swap is a separate
distribution-strategy card later.

### Tests

- Combined-mode smoke: spawn `fspec` as a subprocess, parse its
  stderr `PORT=<n>` line, connect a second `fspec client`-style
  WebSocket client to it, list work units, assert non-empty result.
- Daemon-mode smoke: spawn `fspec daemon`, parse stdout port, connect
  WS client, assert the same.
- Client-mode smoke: spawn `fspec daemon`, then `fspec client
  --connect ws://...` (with stdin/stdout piped to a `tui-test`-style
  harness), drive a few keypresses, assert work-units list renders.
- Autodiscovery smoke: spawn `fspec daemon` (writes daemon.json), spawn
  bare `fspec client`, assert it discovers the URL.

## Out of scope

- Auth, TLS, non-loopback binds, multi-tenancy.
- `fspec daemon` install-as-service helpers.
- Replacing the TS `fspec` on the npm distribution path (separate
  follow-up — needs platform-specific binary distribution).
- Daemon-per-user vs daemon-per-project routing — both modes are
  process-per-workspace in this card.

## Acceptance — done when

1. `fspec`, `fspec daemon`, `fspec client` all run.
2. Combined and daemon modes always start the WebSocket server.
3. Client mode can autodiscover a running daemon via daemon.json.
4. Three subprocess-spawn smoke tests cover all three modes.
5. The WebSocket port is reported on the right channel for each mode
   (stderr for combined, stdout for daemon).
6. NAPI smoke and existing RPC-005/006/007/008/009 tests still pass.

## Estimate guidance

8 points. The subcommand multiplexer and clap surface are small. The
fiddly parts: stderr-vs-stdout port reporting (so combined mode doesn't
trash the TUI), daemon.json handshake, and the three subprocess tests.
