# PROV-151 — Codex tool input streaming and safe dispatch

## Goal and evidence

Make incremental tool input observable and dispatch valid complete Codex calls without waiting
for transport closure, while preventing duplicate or partial execution.
Audit: fspec `38a95522`; upstream https://github.com/openai/codex `bb64b89`.
Our rig Responses path buffers complete calls until EOF. The shared multi-turn agent sends
argument deltas only to a hook, not the ordinary output stream. Official turn handling processes
completed items and supports custom-tool input deltas for incremental diff presentation.

## Codex-only boundary

Depends on PROV-147, PROV-148 and PROV-149. Tool input assembly, event emission and dispatch
timing changes must live in a Codex-only path. Do not alter tool timing for other providers.
Retain existing permission checks, isolation, cancellation and tool concurrency limits.

## Acceptance criteria

1. Codex tool name and argument updates reach a Codex presentation consumer before call completion.
2. Partial arguments never execute; complete calls dispatch according to a documented safe item lifecycle.
3. Calls can be handled after their completed item without waiting for the HTTP body to close.
4. Interleaved call IDs keep independent buffers and matching results; duplicate events do not reexecute a call.
5. Failure or cancellation prevents dispatch of incomplete calls and preserves already completed tool outcomes.
6. Ordinary function-call behavior and other providers remain unchanged unless Codex explicitly selects this policy.

## Test examples and implementation notes

Use two interleaved calls, fragmented JSON strings, an early completed item on an open stream,
duplicate completion, malformed arguments and cancellation. Assert invocation counts and output
ordering using safe local tools. Validate actual upstream payload shapes before requiring optional
fields such as content_index; the audit identified strict parsing, not a proven failure for every wire shape.

Define retry/idempotency boundaries with PROV-154. Do not promise exactly-once external side effects
across crashes; retain completed IDs and avoid automatic replay of known executed calls.
Custom/freeform dialect registration is PROV-157; this card establishes the streaming bridge.

## Validation and exclusions

Gherkin first, failing integration tests second, implementation last. Use shared fixture helpers,
scoped Cargo commands and non-Codex timing controls. No global tool scheduler rewrite.
