# PROV-149 — Lossless Codex continuation state

## Goal and evidence

Preserve protocol state across Codex sampling steps, application turns and supported session reloads.
Audit: fspec `38a95522`; upstream https://github.com/openai/codex `bb64b89`.
Our Responses `Output::Reasoning` drops encrypted content, and input conversion writes None.
The shared agent retains reasoning only with a Claude-style signature and reconstructs tool-turn
history without streamed commentary. Message phase, IDs and item boundaries are also reduced.
Upstream `codex-rs/protocol/src/models.rs` preserves typed ResponseItems, including encrypted
reasoning and message phase; `codex-rs/core/src/session/turn.rs` continues from typed history.

## Codex-only boundary

Depends on PROV-147 and PROV-148. Implement Codex-owned typed continuation state, not a global
change to rig messages, Claude signatures or preserve-thinking preferences. Shared persistence
may gain a narrowly scoped optional Codex payload; old and non-Codex records must remain valid.
Any major persistence refactor needs approval first.

## Acceptance criteria

1. Encrypted reasoning returned by Codex is preserved byte-for-byte and replayed when the protocol requires it.
2. Assistant IDs, phase, item boundaries, ordering, commentary and function call IDs survive continuation.
3. Displaying or hiding readable thinking does not silently delete required opaque Codex protocol state.
4. Save and reload preserve supported Codex continuation state; legacy records without it use a documented safe fallback.
5. Tool results match their call IDs without reordering or duplicating prior tool calls on continuation.
6. Non-Codex history serialization and preserve-thinking behavior remain unchanged.

## Test examples and implementation notes

Use a transcript containing reasoning, two differently phased messages, parallel function calls,
and results. Capture the next request and compare the ordered replayable items to the originals.
Test both immediate tool continuation and a new user turn after session reload.
Add property tests for opaque payload preservation and snapshots for typed protocol envelopes.
Do not claim byte-identical JSON formatting for fields whose semantic normalization is intentional.

Opaque reasoning is protocol data, not readable thinking: never render it, log it, or include real
payloads in fixtures. Respect existing session deletion/privacy controls. Decide retention across
compaction explicitly with PROV-158; do not append stale items after a conversation reset.

## Validation and exclusions

Prepare Gherkin and failing tests first, using shared filesystem helpers and package-scoped targets.
Recheck upstream serialization rules before treating every received field as replayable.
No general history migration, global thinking-policy change or automatic WebSocket chaining here.
