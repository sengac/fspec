# BUG-245: Separate archival scrollback from model context

## Priority and evidence

**P2 — confirmed projection mismatch, not erased persisted history.**
Cold resume uses the compaction-aware model-history getter to generate display chunks.
After compaction it therefore displays a synthetic summary plus recent suffix instead
of the full original conversation. Historical references remain in the manifest.
The screenshot session has no committed compaction, so this is a separate issue.

## Root cause and code to inspect

- `rust/core/src/persistence/session_replay.rs:15`: uses `get_session_messages` for restoration envelopes.
- `rust/core/src/persistence/manifest.rs:830`: substitutes a summary and skips the compacted prefix.
- `rust/core/src/persistence/manifest.rs:874`: full getter retains original access but currently skips unavailable records.
- `rust/sessions/src/replay_content.rs:41`: one conversion creates both model messages and display chunks.
- `rust/sessions/src/session_restore.rs:100`: installs model messages, then emits the same projection for display.

The model must use the validated compacted generation. The user transcript must not be
limited to what currently fits in that model context window.

## Required repair

1. Define separate, explicitly named model-replay and archival-transcript projections.
   Share decoding/identity logic, not business logic that forces their content sets to match.
2. Hydrate the TUI from full original conversational/event records with original coordinates,
   timestamps and tool content. Do not expose provider checkpoints as blank assistant turns.
3. Preserve compaction boundaries/summary as clearly identified metadata or markers without replacing
   the original transcript or pretending synthetic content is an original user message.
4. Continue installing only summary plus compatible suffix into inference; maintain validated
   provider generation, reminders, token accounting, and next-request admission.
5. Use BUG-244's paginated/cursor delivery contract. Coordinate the legacy checkpoint distinction
   with BUG-243. Do not introduce a second independently ordered archive.
6. Make explicit visual trimming discoverable and provide historical access when older rows are not
   resident. Preserve ordinary collapsed-tool expansion; completeness does not require unbounded RAM.

## Regression scenarios

- Save a multi-turn session with tools, compact its prefix, cold resume, and verify every original
  transcript item is available while the actual next provider request contains only the committed projection.
- Perform two compactions and resume repeatedly; keep original ordering and boundary identities
  without duplicate summaries or duplicated suffix work.
- Verify full tool results/arguments remain accessible even when their normal views are collapsed.
- Missing referenced originals return a visible typed archive failure, not silently skipped rows.
- Test both transports, legacy uncompacted sessions, large archives, and marked visual trimming.
- Ensure opening history does not change durable manifests, source coverage, tokens, or inference state.

## Non-goals and constraints

- This is not a replacement for existing hierarchical DAG compaction or a switch to native/flat memory.
- Do not feed the full archive back into model context or raise context limits to make resume look complete.
- PROV-046 is related historical backlog, but its suggested second history location is not an approved
  requirement here. Reuse authoritative stores and avoid creating another source of truth.
- Feature scenarios must distinguish archival completeness from in-memory rendered row policy.

## Completion evidence

- [x] A real compacted cold-resume test asserts archival completeness and reduced physical model input.
- [x] Repeated compaction/resume retains identity, tool pairing, and exact-once suffix display.
- [x] Missing data and visual trimming are explicit; large-history access is bounded.
- [x] Features, red/green tests, shared decoding, and scoped quality checks are documented.

Final evidence: `../BUG-244/final-acdd-validation.md`.

Related: BUG-243, BUG-244, PROV-046. Investigation:
`docs/investigations/2026-10-23-compaction-and-resume.md`.
