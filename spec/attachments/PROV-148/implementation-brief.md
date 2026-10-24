# PROV-148 — Codex stream terminal states and continuation

## Goal and evidence

Give Codex an explicit stream lifecycle instead of treating socket EOF as successful completion.
Audit baseline: fspec `38a95522`, upstream https://github.com/openai/codex `bb64b89`.
Our shared Responses `streaming.rs` processes only completed response variants, waits for EOF,
and can finalize without observing completion. Its final response exposes usage, not `end_turn`.
Upstream `codex-rs/codex-api/src/sse/responses.rs` handles failed/incomplete responses and errors
on premature EOF. `codex-rs/core/src/session/turn.rs` respects explicit `end_turn: false`.

## Codex-only boundary

Depends on PROV-147. Put lifecycle policy in the Codex adapter. Do not globally change rig's
OpenAI parser or generic continuation heuristics. Shared event plumbing, if essential, must
be explicit opt-in for Codex, with unchanged non-Codex defaults and regression tests.

## Acceptance criteria

1. A completed response finalizes exactly once without waiting for HTTP connection closure.
2. EOF before completion produces an interrupted-stream error, never a successful empty final result.
3. Failed responses and top-level error events retain actionable error codes and messages; no success follows them.
4. Incomplete responses distinguish interrupted, content-filtered and budget-limited outcomes and preserve available usage.
5. Explicit end_turn false triggers bounded continuation; true ends the turn; absent uses documented fallback behavior.
6. Unknown optional events may be ignored, but malformed known terminal events cannot silently become success.

## Test examples and implementation notes

- Local SSE server sends completion and keeps its socket open: client finishes immediately.
- Server drops after one text delta: retain visible partial text but report interruption.
- Failure arrives after tool argument deltas: incomplete calls never execute.
- Interrupted incomplete response requests follow-up without duplicating completed tool effects.
- A terminal event is repeated: no duplicate usage, completion or tool dispatch.

Define a Codex state machine with explicit terminal outcomes and cancellation ownership.
Do not infer that a transport retry is safe after arbitrary side effects; coordinate PROV-154.
Detailed tool dispatch policy belongs to PROV-151, not a blind flush after every failure.

## Validation and exclusions

Write Gherkin and failing local-server integration tests before code. Use shared test helpers,
one test per scenario, scoped Cargo targets and non-Codex control fixtures (PROV-147 contract).
No live credentials required. WebSocket transport and broad provider error refactors are excluded.
