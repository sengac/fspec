# PROV-154 — Codex stream recovery and cancellation

## Goal and evidence

Give Codex predictable idle deadlines, bounded recovery and prompt cancellation without replaying
known tool side effects. Audit: fspec `38a95522`, upstream https://github.com/openai/codex `bb64b89`.
Current OAuth refresh is proactive only; generic application recovery starts a new Continue request.
In the CLI status-enabled branch, status ticks recreate the idle sleep. The SSE consumer breaks on
errors despite its generic event source having reconnect machinery.
Official SSE has transport-level idle deadlines; client.rs handles recoverable unauthorized setup.

## Codex-only boundary

Depends on PROV-147, PROV-148, PROV-149 and PROV-153. Implement deadlines and recovery inside
the Codex adapter/session driver, not a global patch to CLI retry/timeout behavior. Other providers'
timers and recovery policies are excluded. Any Codex opt-out from generic retries must be explicit.

## Acceptance criteria

1. A stalled Codex network stream times out based on actual transport progress, unaffected by UI ticks.
2. User cancellation interrupts network waits and retry backoff promptly, retaining useful partial output.
3. Recoverable unauthorized setup triggers bounded credential refresh and retry without logging secrets.
4. Retryable errors honor available retry advice and a bounded budget; permanent failures stop clearly.
5. Recovery distinguishes safe request retry from a new continuation and does not replay known executed calls.
6. Long-running authorized tools are not mistaken for network stalls, and non-Codex recovery remains unchanged.

## Test examples and implementation notes

Use controlled Tokio time and local endpoints for idle stream with UI ticks, Retry-After, 401
followed by refreshed success, repeated auth failure, cancellation during sleep and disconnect
after a tool result. Persist completed-call identity via PROV-149 where needed.
Do not claim seamless same-response resume where the protocol requires a new request. Do not
retry partial side-effecting work without an explicit safe policy. Never silently replay credentials
to a different host during fallback or redirect.

## Validation and exclusions

Specify behavior in Gherkin and prove failure before implementation. Use shared fixtures and
scoped Cargo tests; no real OAuth refresh required. WebSocket interrupt/drain belongs to PROV-156.
The analogous timeout issue for other providers remains a separate card, not incidental scope.
