# PROV-150 — Codex reasoning stream presentation

## Goal and evidence

Expose readable Codex reasoning consistently across incremental and completed-section delivery.
Audit: fspec `38a95522`; upstream https://github.com/openai/codex `bb64b89`.
Upstream SSE handles reasoning_text.delta, reasoning_summary_text.delta/done, and summary parts.
Its turn loop optionally displays complete summaries with sequential-cutoff delivery.
Our Responses parser lacks raw reasoning deltas; summary done reuses a structure expecting delta
instead of text. The application consumes ReasoningDelta but ignores completed Reasoning items.

## Codex-only boundary

Depends on PROV-147 and PROV-149. Normalize Codex reasoning in its adapter and use a Codex-selected
presentation bridge. Do not change Claude/Gemini thinking rendering or generic history rules.
Treat reasoning state retention (PROV-149) separately from what the UI displays.

## Acceptance criteria

1. Supported readable reasoning and summary deltas reach the Codex frontend incrementally.
2. Summary done events read text correctly and deliver complete sections when incremental text is absent.
3. Receiving both deltas and done for one section does not display the same text twice.
4. Multiple item IDs and summary indexes preserve section identity and order, including interleaving.
5. Feature-gated completed-section delivery works without suppressing ordinary answer text streaming.
6. Encrypted reasoning never appears in the UI or logs, and other providers render exactly as before.

## Test examples and implementation notes

Exercise delta-only, done-only, delta-plus-done, empty summary, multiple sections and malformed
known events with deterministic local fixtures. Follow events through the Codex output bridge,
not merely JSON deserialization. Do not synthesize private reasoning if the backend emits none.
Request-side sequential-cutoff capability negotiation belongs to PROV-152.

Upstream references: `codex-rs/codex-api/src/sse/responses.rs` and
`codex-rs/core/src/session/turn.rs` (ReasoningSummaryDelta/Done branches).
Current integration references: rig Responses `streaming.rs`, CLI interactive `stream_loop.rs`.

## Validation and exclusions

Specify scenarios, establish failing tests, then implement. Use shared helpers, scoped Cargo
targets, UI event snapshots, and a non-Codex thinking regression control. No global UI redesign.
