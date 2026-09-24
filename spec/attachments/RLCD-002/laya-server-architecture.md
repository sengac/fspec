# JEV-002 — Jev Protocol HTTP Server: Architecture

Overarching architecture for serving a `convaiinnovations/laya` checkpoint behind the
open **Jev / Simple-Jev classifier protocol** (see `protocol-reference.md`, attached to
this card and summarized from the sources listed at its end).

## Goal

`laya serve` runs a Rust HTTP server so that any Jev-protocol client — the featherless
simple-jev clients, MCP servers that target `/v1/classifier`, or TypeSafe's native
`{state, questions}` shape — can send a `ClassifierRequest` and receive the standard
`{model, answers, usage}` response, backed by laya-rs's native candle inference instead
of the Python/PyTorch simple-jev `--backend laya` (which is the existing reference
implementation this project replaces).

## Why this protocol (and not TypeSafe's hosted REST envelope)

- The Simple-Jev contract is the open, versioned (`v1`), fully documented interface, and
  it is *provably compatible with Laya checkpoints*: simple-jev ships `--backend laya`
  that serves `convaiinnovations/laya` through exactly these endpoints.
- TypeSafe's official API (`/api/v1/decisions/*`, `{code, message, data}` envelope, Bearer
  keys) is a hosted, closed service; its five preset endpoints are just canned question
  sets over the same `{state, questions}` core. Presets can be layered later if ever
  wanted; they are out of scope here.

## Server stack

A small, local, axum-based HTTP server inside the `laya` CLI binary:

| Concern | Choice |
| --- | --- |
| HTTP engine | `axum` 0.8 (`Router`, `axum::serve`) |
| Runtime | `tokio` (minimal + rt, macros, net; `net` only for the CLI's bind path) |
| Middleware | `tower-http` 0.6 (`TraceLayer`; CORS optional) |
| Router factory | `build_router(state) -> Router` (test-injectable, no sockets) |
| Shared state | `ServerState` — `Clone` newtype over `Arc<Inner>`, injected via axum `State` |
| Lifecycle | `start_server(config) -> ServerHandle { port, shutdown: oneshot, task: JoinHandle }`, `handle.stop()` for graceful shutdown |
| Health | `GET /health` (protocol: `{"status":"ready","model":<model>}`) |
| Handler modules | `handlers/` per endpoint |

Dependency additions (native-only, target-gated like `ureq`/`whichlang` so the
`wasm32-unknown-unknown` build is untouched): `axum`, `tokio` (minimal + rt, macros,
net; `net` only for the CLI's bind path), `tower-http` (`trace` feature).

## Module layout (new `src/server/`)

```
src/
  server/
    mod.rs          # re-exports; public surface: build_router, ServerConfig, ServerState, start_server
    config.rs       # ServerConfig { host, port, model_name, model, max_model_len, max_request_branches,
                    #                max_admitted }  (clap value parser + defaults)
    state.rs        # ServerState — Clone newtype over Arc<Inner> { agent: RLAgent (Arc), model_name,
                    #                max_model_len, max_request_branches, admission: Semaphore(16) }
    request.rs      # JEV-003: strict ClassifierRequest parse/validate (see request-schema.md)
    response.rs     # JEV-005: laya-native answer mapping + usage + error envelope builders
    handlers/
      mod.rs
      classifier.rs # JEV-004: POST /v1/classifier + POST /v1/systemone (one shared handler)
      health.rs     # JEV-004: GET /health
    router.rs       # build_router / build_router_with_config
  main.rs           # JEV-006: Command::Serve — checkpoint resolution, start_server, ctrl-c shutdown
```

Nothing in `src/server/` takes weights-dependent paths: request/response modules are
pure over plain data (they only touch `RLAgent` at the `system_one` call boundary),
which keeps the whole protocol layer unit-testable without a checkpoint, mirroring the
repo's `LAYA_TEST_MODEL`-gated testing convention.

## Request pipeline (one `/v1/classifier` request)

```
POST /v1/classifier (or /v1/systemone)
   │
   ├─ 1. Body limit: raw body > 1 MiB → 422 (protocol caps bodies; see reference doc)
   │      Content-Type must be application/json (sniffed; missing = tolerated, wrong = 422)
   ├─ 2. serde_json::from_str → parse error → 422 schema-error envelope
   ├─ 3. request::ClassifierRequest::validate (JEV-003)
   │      • model nonempty; must equal the loaded model name, else 422
   │        ("Loaded model is '<name>'")
   │      • exactly one of state / messages (both null or both set → 422)
   │      • questions: 1..=256 entries, nonempty ids; per-type strict validation
   │        (choice 2–50 criteria object; score 2–50 ordered array; noul optional
   │        {true,false} only); unknown question/option fields rejected
   │      • options.raw_logits / tools / mm_processor_kwargs / media_io_kwargs
   │        nonempty → 422 (laya backend rejects raw-logit diagnostics and media)
   │      • messages: text-only roles system/developer/user/assistant, string content
   │      • length admission: each built question sequence must fit
   │        min(--max-model-len, checkpoint max_len); overflow → 422 BEFORE inference
   │        (laya's native formatter silently truncates today; the server must not)
   │      • question count > max_request_branches → 422
   ├─ 4. Concurrency (JEV-005):
   │      • one model forward at a time (tokio::sync::Mutex or a 1-permit model lock)
   │      • admission queue: Semaphore(16) waited *before* the lock; full → 429
   │        {"detail":"Scoring queue is full"} + Retry-After: 1
   │      • tokio::spawn the blocking forward on a blocking thread pool
   │        (RLAgent::system_one is synchronous and CPU/GPU-bound) and observe
   │        cancellation between requests (client disconnect → drop, 499 if deliverable)
   ├─ 5. Inference: agent.system_one(&state_value, &questions)
   │      (state | messages → Value; messages serialized as a JSON list of
   │       {role, content} objects — the reference does exactly this for the laya backend)
   └─ 6. Response assembly (JEV-005):
          { model, answers: {qid: laya-native answer}, usage: {input_tokens, output_tokens: 0} }
          • answers via agent::answer_to_json minus the `act_probability` field
          • usage.input_tokens = sum of per-question sequence lengths (reference:
            "repeated context is counted"); output_tokens always 0
          • 200 + application/json
```

## Concurrency model

- **Serial model execution.** The reference server is explicitly serial
  ("requests execute serially against the model; parallelism is within each
  request"), and candle has no safe concurrent-forward guarantee on one model.
  All parallelism that matters (many questions per request in one forward pass)
  already exists in `system_one`.
- **Bounded admission queue**: 1 in-flight + 16 waiting (matching the reference's
  "one model request at a time with up to 16 additional requests waiting"),
  configured via `--max-queued`; overflow is 429 per protocol.
- **Blocking offload**: `system_one` runs under `tokio::task::spawn_blocking`
  behind the model lock; the lock is a `tokio::sync::Mutex` held across the
  offload so admission ordering is FIFO-ish and the queue semaphore never over-admits.

## Error model (protocol envelope)

All protocol errors share one JSON envelope (matches the reference's 422/429/499 shapes):

```json
{
  "error": {
    "message": "human-readable summary",
    "type": "invalid_request_error",
    "code": 422,
    "param": "questions.color.criteria",
    "details": [ { "param": "questions.color.criteria", "message": "...", "type": "..." } ]
  }
}
```

- 422: malformed JSON, schema violation, wrong model, bad context combination,
  unsupported media/raw_logits, token/branch limits (up to 10 detail entries;
  dotted paths with `[i]` indices; extra errors folded into the summary).
- 429: `{"detail": "Scoring queue is full"}` + `Retry-After: 1` header.
- 499: client disconnected (only if a response can still be delivered).
- 500: unhandled runtime failure (e.g. model execution error) — no stable body guaranteed.

## Checkpoint & model identity

- `laya serve --model DIR | --model-variant KEY | --models-root DIR` reuses
  `model_path::resolve` (CHECK-001) verbatim; the **model name reported in
  responses and checked against `request.model`** is the explicit flag value the
  user passed (or the variant key), exactly as the reference matches
  "the model ID or local path used to start the server".
- One checkpoint per process (the reference does not hot-swap models).
- Optional `--auto-route` (later child card, out of scope here): pick
  English vs multilingual per request via `laya::route` — deferred because the
  protocol's model-identity rule (one loaded model, strict match) makes a
  per-request router awkward; documented as a known limitation.

## Testing strategy (details in JEV-007's card doc)

- **Pure layer (no weights)**: request validation, response mapping, error
  envelopes, router wiring with `axum::Router::oneshot` (tokio-test style) —
  the state holds an injectable `Answerer` trait object so `build_router` is
  test-injectable (mirroring `build_router_with_config`).
- **Model-gated (LAYA_TEST_MODEL)**: end-to-end HTTP round-trips with the real
  `RLAgent` + `start_server` on a random port (the `ServerHandle` pattern),
  mirroring simple-jev's `test_laya.py` contract tests (mixed choice/score/noul,
  wrong model 422, overflow 422, text chat vs media rejection).

## Child cards (implementation decomposition)

| Card | Scope | Feature |
| --- | --- | --- |
| JEV-003 | Strict Jev classifier request schema (parse + validate, 422 rules) | `jev-classifier-request-schema` |
| JEV-004 | HTTP router, `/v1/classifier`, `/v1/systemone`, `/health`, `/openapi.json`, error envelope middleware | `jev-http-endpoints` |
| JEV-005 | Laya-native answer mapping, usage accounting, serial execution + 429 admission queue | `jev-response-mapping` |
| JEV-006 | `laya serve` CLI subcommand: checkpoint load, server lifecycle, graceful shutdown | `laya-serve-cli` |
| JEV-007 | Protocol conformance test suite (HTTP-level, mirroring the reference tests) | `jev-protocol-conformance` |

Dependency order: JEV-003 → JEV-005 → JEV-004 → JEV-006 → JEV-007
(JEV-004 depends on 003+005 for handler bodies; 006 wires everything; 007 lands
last and gates the epic).
