# PROV-156 — Codex WebSocket transport and incremental continuation

## Goal and evidence

Add optional Codex Responses WebSockets after SSE correctness and state preservation are in place.
Audit: fspec `38a95522`, upstream https://github.com/openai/codex `bb64b89`.
Upstream client.rs manages reusable connections, prewarming, continuation-prefix checks and
fallback; codex-api/src/endpoint/responses_websocket.rs supports interrupt/drain semantics.
Our Codex path is HTTP SSE. SSE is supported upstream and is not intrinsically incompatible.

## Codex-only boundary

Depends on PROV-147, PROV-148, PROV-149, PROV-152, PROV-154 and PROV-155.
Put transport selection and connection state in Codex modules. No global WebSocket migration,
provider-name heuristic, shared HTTP behavior change or other-provider fallback policy changes.

## Acceptance criteria

1. Eligible explicitly enabled Codex sessions can use a reusable Responses WebSocket; SSE remains available.
2. Incremental requests use previous_response_id only when request properties and prior input/output prefixes match.
3. Changed history, reset, account change or invalid baseline sends a full safe request instead of stale continuation.
4. Defined unsupported-transport cases fall back to SSE without duplicate requests or tool effects.
5. Cancellation performs supported interrupt/drain with a deadline and retires unusable sockets.
6. WebSocket and SSE transcripts produce equivalent observable Codex events; non-Codex traffic is unchanged.

## Test examples and implementation notes

Use a local WebSocket/SSE pair to test success, HTTP 426, handshake auth recovery, disconnect,
stale response ID, changed tool definitions, shortened history and cancellation acknowledgement.
Mirror upstream's distinctions: not every error means fallback, and auth failures must not be hidden.
Use bounded reconnects and guard against replaying dispatched calls (PROV-154).
Capability-gated preconnect/prewarm may be included only with tests proving it cannot execute tools
or produce visible duplicate output. Compression is a separate optimization unless required by protocol.

## Validation and exclusions

Gherkin, then failing local transport integration tests, then implementation. Reuse shared helpers
and scoped Cargo commands. Live timing comparisons are opt-in, not unit-test prerequisites.
Do not promise a performance gain without measurements; do not remove the working SSE path.
