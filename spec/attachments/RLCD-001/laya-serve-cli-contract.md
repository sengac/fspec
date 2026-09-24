# JEV-006 — `laya serve` CLI Subcommand

Scope: `Command::Serve` in `src/main.rs` + `ServerHandle`/`start_server` in
`src/server/` (the `start_server`/`ServerHandle` lifecycle pattern from
`architecture.md`) — the entry point that loads a checkpoint, binds the HTTP
listener, and runs until interrupted.

## CLI contract

```
laya serve [--model DIR] | [--model-variant KEY] [--models-root DIR]
           [--host ADDR] [--port N]
           [--max-model-len N] [--max-request-branches N] [--max-queued N]
```

- Checkpoint resolution reuses `laya::model_path::resolve`
  (CHECK-001/CHECK-002: explicit `--model` bypass; else variant key under
  `--models-root`/`LAYA_MODELS_ROOT`; else download into `~/.cache/laya-rs`)
  — identical precedence to `laya ask`/`laya answer`.
- The **model name** reported in `/health`, checked against
  `request.model`, and echoed in responses is the value the user supplied
  (`--model` path or the variant key) — "the model ID or local path used to
  start the server" (reference rule).
- Defaults: `--host 127.0.0.1`, `--port 8000`, `--max-model-len` =
  checkpoint `max_len`, `--max-request-branches 100`, `--max-queued 16`.
  `--max-model-len` greater than the checkpoint's native `max_len` is
  clamped (the effective cap is the smaller; the reference documents this).
- clap flags are native-only; the `serve` subcommand must not break the
  `wasm32` build (server deps are target-gated like `ureq`).

## Lifecycle (`start_server`/`ServerHandle` pattern)

```rust
pub struct ServerHandle {
    pub port: u16,                       // actual bound port (0 → ephemeral)
    pub model_name: String,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}
impl ServerHandle {
    pub async fn stop(self) { /* send oneshot, await task */ }
}
pub async fn start_server(config: ServerConfig, agent: RLAgent) -> anyhow::Result<ServerHandle>
```

- Load the agent **first** (stderr progress like the existing subcommands),
  then `TcpListener::bind`, then `axum::serve(listener, app)` with
  `.with_graceful_shutdown` on the oneshot.
- Ctrl-C / SIGTERM: the CLI's main task installs a `tokio::signal::ctrl_c`
  waiter; on signal → `handle.stop()`. In-flight forwards complete; the
  reference notes an in-flight forward "cannot be interrupted" — graceful
  wait is the correct behavior, not a kill.
- Bind failure (port in use) → `anyhow` error before any HTTP is served.
- Log line on startup: `laya serving '<model>' on http://<host>:<port>`
  plus a hint of the endpoints.

## Errors

- Checkpoint errors propagate as today (download/load failures, readable
  stderr, non-zero exit).
- A bad `--host`/`--port`/`--max-*` value is a clap-level error (validate
  positivity of the three `--max-*` flags with clap value parsers).

## Testing notes

- Weight-free: `ServerConfig` parsing/clamping, model-name precedence, the
  `--max-model-len` clamp, and `start_server` with an `RLAgent` behind the
  `Answerer` mock (binds an ephemeral port, health check, stop()).
- Model-gated (LAYA_TEST_MODEL): real load + one round-trip + clean
  shutdown (JEV-007 owns the full conformance suite; this card's tests only
  prove the CLI wiring).
