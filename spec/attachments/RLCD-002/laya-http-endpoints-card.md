# JEV-004 — HTTP Endpoints and Router

Scope: `src/server/router.rs` + `src/server/handlers/` — the axum router and
endpoint handlers that expose the Jev protocol. Contract:
`protocol-reference.md` (attached to JEV-002). Depends on JEV-003 (request
validation) and JEV-005 (answer mapping + execution); those seams are traits
so this card's router compiles and tests against mocks.

## Router (standard axum router-factory pattern)

```rust
pub fn build_router(state: ServerState) -> Router {
    Router::new()
        .route("/v1/classifier", post(handlers::classifier::classify))
        .route("/v1/systemone", post(handlers::classifier::classify))  // exact alias
        .route("/health", get(handlers::health::health))
        .route("/openapi.json", get(handlers::openapi::openapi))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
```

- `ServerState` (JEV-005) is the `Clone`-over-`Arc` state newtype, injected via
  axum's `State` extractor.
- The reference also serves `/docs` (Swagger) and `/redoc`; the laya-rs server
  serves `/openapi.json` only (the machine-readable contract) — interactive
  docs are out of scope and noted in the feature file.

## GET /health

`{"status":"ready","model":"<loaded model>"}` — 200. Served after the model
is loaded; `start_server` only binds after load success (the CLI waits for
load before listening, so there is no "loading" state to report).

## POST /v1/classifier (and /v1/systemone)

Shared single handler; pipeline (details in JEV-005/003 cards):

1. Reject bodies > 1 MiB (axum `DefaultBodyLimit::max(1_048_576)` at the
   router) and non-JSON `Content-Type` → 422.
2. Parse + validate via JEV-003's `ClassifierRequest` → 422 envelope on error.
3. Model-identity check: `request.model == state.model_name` else 422
   `Loaded model is '<name>'`.
4. Admission: model lock + queue (JEV-005) → 429 when the queue is full
   (`{"detail":"Scoring queue is full"}`, `Retry-After: 1`).
5. Infer + map (JEV-005) → 200 response.

Handler never panics on malformed input (the reference's 422 contract
requires readable errors for every validation path).

## GET /openapi.json

A static OpenAPI 3.1 document (hand-written JSON in the crate, no codegen
dependency) describing the three classifier endpoints + the request/response
schemas. Served to keep `/openapi.json` parity; its content is the
`protocol-reference.md` schema, not a codegen artifact.

## Error envelope (one place, shared with JEV-005)

`handlers/error.rs` builds the protocol error JSON from a
`ClassifierError` (schema/admission from JEV-003, execution from JEV-005):

```rust
pub struct ErrorBody {
    pub error: ErrorDetail { message, r#type: "invalid_request_error", code, param, details },
}
```

Plus the short forms: 429 `{"detail":"Scoring queue is full"}` (+
`Retry-After: 1`), 499 `{"detail":"Client disconnected"}`, 500
`{"detail":"internal error"}` for unexpected failures (e.g. a model forward
error, which is a 500 per the reference's "unhandled runtime failure").

## Testing notes

- Weight-free tests: router wiring via `axum::Router::oneshot` with a mock
  `Answerer` (the JEV-005 trait) — verify alias behavior, health body,
  content-type/body-limit 422s, openapi shape.
- Model-gated (LAYA_TEST_MODEL): real round-trips, per JEV-007.
