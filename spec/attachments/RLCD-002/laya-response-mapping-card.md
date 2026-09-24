# JEV-005 — Laya-Native Answer Mapping, Usage, and Execution

Scope: `src/server/response.rs` + the execution/admission layer in
`src/server/state.rs` — turn `RLAgent::system_one` output into the protocol
response shape, and run the model serially with a bounded admission queue.
Contract: `protocol-reference.md` (JEV-002 attachment), "Lay-a backend
admission rules" + "Response body" sections.

## Answer mapping (laya-native semantics)

Input: `Vec<(qid, laya::Answer)>` from `RLAgent::system_one` + the original
`Vec<(String, Question)>` + per-question sequence lengths.

| Jev answer | laya-rs source | Transform |
| --- | --- | --- |
| choice: `choice` | `Answer::Choice.choice` | pass through (argmax already computed) |
| choice: `confidence` | `Answer::Choice.confidence` | pass through (= max candidate prob) |
| choice: `probabilities` | `Answer::Choice.probabilities` | `{candidate_id: p}` — insertion order of the request's criteria, sum ≈ 1 |
| score: `score` | `Answer::Score.score` | pass through (expected zero-based index `sum(p[i]*i)`) |
| score: `confidence` | `Answer::Score.confidence` | pass through (largest level prob) |
| score: `probabilities` | `Answer::Score.probabilities` | keys `"0".."N-1"` |
| score: `legend` | `Answer::Score.legend` | keys `"0".."N-1"` → original criteria text |
| noul: `noul` | `Answer::Noul.noul` | pass through — the **native P(true)**, clamped to [0,1] (the reference's laya backend explicitly uses the SDK's native binary probability, NOT the v1 nine-bin 0.01–0.99 mapping) |

**Stripped:** `act_probability` (the laya SDK's action field — the reference
backend omits it; laya-rs's `answer_to_json` emits it, so the server mapping
must not). Implement as a dedicated `answer_to_jev_json(answer, question)`
rather than post-filtering `answer_to_json` (which also re-sorts maps via
BTreeMap — the protocol wants request order for choice probabilities).

## Usage accounting

```
usage.input_tokens  = Σ len(built.sequence ids) over all questions
usage.output_tokens = 0
```

The laya backend re-tokenizes context per question (no shared-prefix cache),
so repeated context is counted per question — matching the reference's
documented behavior for `--backend laya`. Sequence lengths come from
`schema::build_sequence` (the same calls `system_one` makes); `system_one`
should be extended to also return per-question lengths (a pure refactor of
existing data it already computes) or the server recomputes them from the
validated questions (idempotent — same tokenizer + same inputs).

## Pre-inference length admission (422 before inference)

`build_sequence` today *truncates* silently; the protocol layer forbids that.
The handler, after request validation and before inference:

1. For each question, build the sequence with `schema::build_sequence` using
   the checkpoint's `max_len`/`head_max_len` and the configured
   `min(--max-model-len, cfg.max_len)` cap.
2. If any question's option markers are lost to truncation
   (`built.markers.len() != n_opts`) or the total exceeds the cap →
   422 `question '<id>' exceeds <cap> input tokens` (no inference runs;
   `system_one`'s existing marker-loss bail becomes unreachable via the
   server path).
3. Question count > `--max-request-branches` (default 100) → 422.

The same pre-built sequences are then handed to inference, so the server does
not tokenize twice (the refactor exposes `system_one`'s internal
tokenization step to its caller).

## Execution + admission queue

```
ServerState {
    agent: Arc<RLAgent>,            // loaded once at startup
    model_name: String,             // the --model/--model-variant value
    config: ServerConfig,           // max_model_len, max_request_branches, max_queued
    model_lock: tokio::sync::Mutex<()>,   // one forward at a time
    admission: tokio::sync::Semaphore,   // max_queued permits (default 16)
}
```

Flow:
1. `admission.acquire_owned().await` → permit full and request still
   incoming → 429 path (the handler treats a full semaphore as "queue is
   full" per the reference's 1-in-flight + 16-waiting model; in practice the
   semaphore covers the waiting slots and the mutex covers the in-flight one).
2. `model_lock.lock_owned().await` (FIFO-ish ordering among admitted
   requests).
3. `tokio::task::spawn_blocking(move || agent.system_one(...))` — the candle
   forward is synchronous; never run it on an async runtime thread.
4. Map results (above) → response; release both guards.
5. Client disconnect: `axum` cancels the future when the client goes away;
   the in-flight forward cannot be interrupted (reference behavior) —
   if a response can still be delivered, send 499
   `{"detail":"Client disconnected"}`; otherwise drop.

## Why serial

The reference server documents "requests execute serially against the
model; parallelism is within each request" — all parallelism that matters
(already many questions in one forward pass) exists inside
`system_one`. Adding concurrent forwards on one candle model risks device
contention and nondeterministic GPU memory layout; serial keeps latency
predictable and matches the protocol's reference semantics exactly.

## Testing notes

- Mapping + usage: pure unit tests over hand-built `Answer` values
  (weight-free), including the act_probability-stripping and ordering cases.
- Length admission: unit-test with a small `max_len` cap and a long state
  (the sequence builder needs only the tokenizer — the existing
  `tests/schema.rs` in-memory-tokenizer pattern).
- Queue/429: weight-free test with a mock `Answerer` that sleeps — fill the
  semaphore, assert 429 + `Retry-After: 1`; assert exactly one concurrent
  forward via an atomic counter in the mock.
