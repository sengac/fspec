# JEV-007 — Protocol Conformance Test Suite

Scope: `tests/jev_conformance.rs` (integration) + `tests/jev_conformance/`
fixtures — HTTP-level tests that pin laya-rs to the Jev/Simple-Jev protocol
contract, mirroring simple-jev's own `test_laya.py` (contract) and
`test_api.py` (validation) suites. This card is the epic's quality gate:
nothing in the `jev-server` epic is `done` until this suite passes.

## Two tiers (repo testing convention)

1. **Weight-free (always run)**: `axum::Router::oneshot` against
   `build_router` with a mock `Answerer` — no checkpoint needed.
2. **Model-gated (`LAYA_TEST_MODEL`)**: `start_server` with a real
   `RLAgent` on an ephemeral port, `reqwest`/`ureq`-free — the tests use the
   `tokio` + `hyper` client that axum's dev-deps already provide (or
   `reqwest` with `rustls-tls` if a dev-dep is preferred — decided at
   implementation; keep it dev-only).

Fixtures: `tests/jev_conformance/*.json` — request bodies for each protocol
case (see scenario list). Keep them small; the mixed choice/score/noul
fixture reuses the README's refund example.

## Scenario list (mirror of the reference tests)

### Contract (from simple-jev `test_laya.py`)

- **Mixed batch round-trip**: one request, one choice + one score + one
  noul → 200; `answers` keyed by question id in request order; choice has
  `choice`/`confidence`/`probabilities` (no `act_probability`), score has
  `score`/`confidence`/`probabilities`/`legend`, noul has only `type`/`noul`
  (no confidence field); `usage.output_tokens == 0`; `usage.input_tokens > 0`
  and equals the sum of per-question sequence lengths.
- **Alias**: `POST /v1/systemone` returns an identical response shape for an
  identical body to `/v1/classifier`.
- **Health**: `GET /health` → `{"status":"ready","model":"<name>"}`.
- **Wrong model**: request `model` ≠ loaded name → 422 envelope with
  message `Loaded model is '<name>'`.
- **Context overflow**: state text long enough that a question sequence
  exceeds `--max-model-len` → 422 with the question id named; **no
  inference** (assert via mock / timing that the model was never called).
- **Branch limit**: more questions than `--max-request-branches` → 422.
- **Chat history**: `messages` instead of `state` (2–3 text turns) → 200,
  answered against the serialized message list.
- **Media/tool rejection**: message with `content` as a part array (image),
  `role: "tool"`, or a `name` field → 422 "text messages" error.
- **`options.raw_logits: true`** → 422.
- **Reserved fields**: nonempty `tools` / `mm_processor_kwargs` /
  `media_io_kwargs` → 422; empty ones → 200 (accepted, no effect).

### Validation matrix (from `test_api.py` + request_schema)

- Context XOR: both `state`+`messages`, neither, or `state: null` +
  `messages` → 422 `Provide exactly one of state or messages`.
- Unknown top-level fields (e.g. `stream: true`, `temperature`) → **200**
  (ignored), for an otherwise valid body.
- Unknown question fields → 422 with `param` naming the question id.
- `type` outside choice/score/noul → 422.
- choice criteria: 1 candidate → 422; 51 candidates → 422; 50 → 200.
- score criteria: 1 level → 422; 51 levels → 422; empty array → 422.
- noul criteria with a `maybe` key → 422; with only `true` → 200.
- Empty question id key `""` → 422; >256 questions → 422.
- `state` as bare number/bool → 422; `state: ""` / `state: {}` → 200.
- Malformed JSON body → 422 parse envelope (not 500).
- Body > 1 MiB → 422.
- Non-JSON `Content-Type` → 422.
- 429: fill the admission queue (`--max-queued 1` + a blocking mock
  forward) → `{"detail":"Scoring queue is full"}` + `Retry-After: 1`.
- 500: mock forward erroring → 500, and the server stays up (next request
  succeeds).

### Numerical sanity (model-gated only)

- Choice probabilities sum within 0.01 of 1; `confidence` equals the max
  probability; `score` lies in `[0, N-1]`; `noul` in `[0, 1]`.
- Determinism: identical request twice → identical answers (laya is
  non-autoregressive; no sampling).

## Pass criteria

- All weight-free scenarios pass on a plain `cargo test` (no GPU, no
  checkpoint).
- Model-gated scenarios pass under `LAYA_TEST_MODEL=<checkpoint dir>` and are
  skipped (not failed) otherwise — the repo's established gating convention.
- `cargo clippy --all-targets` clean (the configured quality check).
