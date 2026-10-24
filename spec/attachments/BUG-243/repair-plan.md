# BUG-243: Separate provider state from conversation originals

## Priority and evidence

**P1 — directly reproduced against a production-written session.**
`SessionSearch originals` for session `7a093866-f589-4d59-817e-3a86a189e9fd`,
`start_turn=1`, `end_turn=1`, `limit=1`, `max_bytes=61440` returns
`missing_envelope` for message `1a9a0833-ce7a-4a0b-abd9-33c02a1adfd2`.
This is not a missing indexed record: its metadata exists but is not an original envelope.

There are 217 provider-checkpoint records in this session. Of these, 215 metadata objects
exceed the 61,440-byte originals budget; the largest is 1,913,685 bytes. Indexed checkpoint
records occupy 221,300,553 bytes versus 1,253,283 bytes for ordinary records. Adding UUIDs
alone changes the first failure but does not solve traversal or amplification.

## Root cause and code to inspect

- `rust/cli/src/session/provider.rs:89`: appends empty assistant records containing provider state without envelope UUIDs.
- `rust/providers/src/codex/session.rs:146`: publishes serialized checkpoint metadata through that writer.
- `rust/core/src/persistence/originals.rs:48`: traverses all manifest positions as original envelopes.
- `rust/core/src/persistence/originals.rs:56`: requires a structured message and string UUID.
- `rust/core/src/persistence/originals.rs:69`: enforces bounded whole-envelope responses.
- `rust/agent-loop/src/compaction_evidence.rs:94`: the compactor uses the same reader.

## Required repair

1. Define versioned record kinds and ownership for conversation originals versus provider continuation state.
   Prefer a separately referenced state store/checkpoint identity over repeated full-history pseudo-messages;
   approve the architecture before implementing a major storage refactor.
2. Preserve provider generation/model/protocol identity and crash-safe message/checkpoint ordering.
   Removing pseudo-messages must not make cold Codex continuation lose native items or authentic outcomes.
3. Define backward-compatible reads and an idempotent migration for existing mixed manifests.
   Existing original coordinates, DAG ranges, compaction source prefixes, and continuation cursors must
   retain meaning. Do not silently renumber historical turns or skip malformed genuine originals.
4. Make state records explicitly non-conversational. Specify whether legacy positions are represented
   by typed non-conversation records or a versioned coordinate mapping; do not disguise a checkpoint as dialogue.
5. Keep bounded lossless originals retrieval. Truly oversized conversational envelopes must remain typed
   failures unless a separately specified lossless blob/paging contract handles them.
6. Bound storage amplification through state reuse/deltas/checkpoint retention consistent with recovery
   guarantees. Verify references before cleanup; do not delete state still needed by a live or archived generation.

## Regression scenarios

- Create a session through the real provider persistence binder; produce several tool turns; traverse
  originals from beginning to end through the actual tool without `MissingEnvelope` on internal state.
- Read a legacy mixed manifest with missing checkpoint UUIDs and megabyte state payloads without modifying
  the source or fabricating original content; validate coordinate/cursor behavior explicitly.
- Cold resume before and after migration; next physical Codex request preserves reasoning and tool identity.
- Crash/retry migration at each promotion boundary; repeat migration; retain identical originals and state identity.
- Genuine corrupt/missing originals and oversized dialogue still fail with the correct typed error.
- Repeated turns demonstrate the agreed storage-growth bound and retain required recoverability.

Reuse CMPCT-063 original-retrieval fixtures, but add the real production writer rather than supplied
valid-envelope-only tests. Property-test versioned serialization and coordinate round trips.

## Non-goals and prohibited shortcuts

- Do not expand the response limit to megabytes, truncate checkpoints, or add UUIDs as the entire solution.
- Do not silently drop originals, reset valid Codex lineage, or discard native reasoning to reduce size.
- This card does not repair BUG-242's ephemeral timeout settlement or prove a parent disk-loss event.

## Completion evidence

- [ ] Actual provider-written sessions can be traversed through bounded originals retrieval.
- [ ] Legacy identity/coordinates, DAG references, and cold continuation are preserved.
- [ ] Migration, cleanup ownership, error behavior, and amplification are tested and documented.
- [ ] Feature-first red/green evidence, scoped quality checks, and coverage mappings are attached.

Related: CMPCT-063, BUG-242, BUG-245, BUG-247. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
