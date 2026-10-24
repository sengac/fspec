# PROV-153 — Codex metadata and safety-buffering visibility

## Goal and evidence

Preserve server metadata through the Codex transport and expose useful status without leaking secrets.
Audit: fspec `38a95522`; upstream https://github.com/openai/codex `bb64b89`.
Our generic SSE handle_response reduces the response to its body and loses headers.
Official `codex-rs/codex-api/src/sse/responses.rs` extracts request/model IDs, rate limits,
model-cache etag, reasoning accounting, turn-state and safety-buffering signals. Stream metadata
can also carry model verification and moderation information.

## Codex-only boundary

Depends on PROV-147 and PROV-148. Capture metadata in a Codex transport envelope before generic
body conversion. Add only Codex-selected frontend events, with neutral behavior for other providers.
Do not redefine generic usage fields or rewrite the shared RPC schema without approval.

## Acceptance criteria

1. Upstream request ID and actual serving model remain available for Codex diagnostics and session status.
2. Rate limits, model etag, reasoning-accounting and opaque turn-state metadata survive header/event parsing.
3. Safety-buffering notifications accurately start, update and clear status without delaying text delivery.
4. Verification and moderation notifications are surfaced safely; absent optional metadata does not fail a normal stream.
5. Metadata events do not duplicate usage or output, and unknown optional metadata is handled compatibly.
6. Credentials, opaque reasoning and sensitive routing payloads are not logged or rendered; other providers are unchanged.

## Test examples and implementation notes

Use local responses with mixed-case headers, header-only metadata, event updates, conflicting
sources, missing metadata and explicit buffering false. Define precedence matching the pinned
upstream parser. A suggested faster model is advisory only: never switch automatically.
Preserve retry advice for PROV-154 and turn-state for PROV-155 without implementing those policies here.
Existing cached-input and reasoning token reporting must continue to work.

## Validation and exclusions

Gherkin then failing local-server tests; snapshots of sanitized UI events and redaction tests.
Use shared helpers and scoped Cargo targets. This is not a billing, moderation-policy or global
telemetry redesign. Do not assume every model emits every signal.
