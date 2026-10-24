# Cross-layer checkpoint audit — 2026-10-18

## Status and provenance

Two additional P1 acceptance blockers, reported by independent read-only reviewer
`55e0d7e7-4d2a-4802-9d7c-607bd3cc9e19`, were checked against the current on-disk
call paths by the CMPCT-083 gate owner. This is source-review evidence only:
no new test, build, behavioral RED/GREEN or passing acceptance result is claimed.
The earlier selected-provider identity blocker can mask both findings. Repairing
that identity comparison alone does not establish checkpoint correctness.

No board, production, acceptance criteria, dependency, staging or commit changes
are made for this research update. Existing gate evidence remains intact.

## F1 — P1: checkpoint-only records prevent valid Codex compaction

Affected cards: CMPCT-059, CMPCT-072, CMPCT-083.
Repair owners: CLI runtime, coordinating with providers and the gate test owner.

Actual call path and incompatible representations:

- `rust/agent-loop/src/agent_loop.rs:544` binds the production session's provider
  persistence writer before streaming.
- `rust/cli/src/session/provider.rs:29` writes provider checkpoint metadata into
  an immutable assistant record whose `message` is
  `{"role":"assistant","content":[]}`. This is checkpoint state, not an
  ordinary conversational assistant message.
- `rust/providers/src/codex/streaming.rs:193` commits history through the bound
  writer before yielding tool calls at line 194, including `inject_summary`.
- `rust/cli/src/session/preservation.rs:15` deserializes source-prefix `message`
  values as Anthropic messages while collecting retained reminders.
- `rust/cli/src/session/compaction.rs:124` likewise deserializes messages in the
  generated summary-plus-suffix replay used for application/reconciliation.
- `rust/patches/rig-core/src/one_or_many.rs:340` requires at least one content
  element and rejects the normal checkpoint record's empty array.

Consequently, a valid fitting candidate can fail during reminder preservation
when the checkpoint is in the captured prefix, or during application replay when
it is in the retained suffix. These records arise from normal production Codex
streaming, not malformed user messages. Silently dropping their metadata or
turning checkpoints into conversational text would not satisfy lossless replay.

Required behavioral regressions, before production repair:

1. Bind the actual production persistence writer, stream a real selected Codex
   response, submit an actual `inject_summary` tool call and apply the queued
   candidate through the runtime owner. Do not manufacture a checkpoint-free
   provider substitute or invoke only a persistence helper.
2. Exercise checkpoint records in both captured-prefix and post-capture suffix
   positions. Assert successful installed generation and complete ordered tool
   suffix, with checkpoint metadata retained but absent from conversational
   request content.
3. Assert the next real selected physical request succeeds with the committed
   DAG; verify immutable originals remain retrievable.

## F2 — P1: suffix checkpoint overwrites authoritative cold-restored generation

Affected cards: CMPCT-061, CMPCT-078, CMPCT-080, CMPCT-083.
Repair owners: providers and sessions, coordinating with the gate test owner.

Actual restore order:

- `rust/sessions/src/session_restore.rs:52` calls
  `restore_compaction_boundary`, which installs the manifest's committed provider
  generation via `rust/cli/src/session/restore_compaction.rs:23`.
- `rust/sessions/src/session_restore.rs:58` then restores provider state from all
  replay envelopes, after binding persistence.
- `rust/core/src/persistence/session_replay.rs:47` embeds the committed boundary's
  G+1 checkpoint in the leading synthetic summary envelope. Retained suffix
  records follow in order and can legitimately contain checkpoints written at G
  after source capture and before promotion.
- `rust/providers/src/codex/session.rs:24` scans every envelope and selects the
  last `codexContinuation`, without committed-boundary precedence.
- `rust/providers/src/codex/history.rs:74` validates checkpoint version/protocol
  but replaces the installed state without checking its committed generation.

Thus a legitimate retained G suffix checkpoint can overwrite the installed G+1
boundary checkpoint. Subsequent request assembly may reject stale DAG ancestry
in `rust/providers/src/codex/history_lineage.rs:24`; the committed generation is
already no longer authoritative in provider memory. F1 can prevent ordinary
creation of such a commit, but does not make this restore-order hazard safe.

Required behavioral regressions, before production repair:

1. Persist an authoritative G+1 boundary plus a genuine G checkpoint-bearing
   retained suffix, preserving exact original order and payloads.
2. Restore through the real `SessionManager` in a fresh process, not just direct
   `CodexHistory` or manifest helpers. Assert the boundary remains authoritative
   and the complete suffix survives without rollback or silent deletion.
3. Send the next physical Codex request and assert G+1 lineage only. Include
   incompatible checkpoint data and require typed rejection before mutation,
   no alternate request and no successful completion notice.

## Gate blind spot and completion condition

`rust/sessions/tests/cmpct083/runtime.rs:62` constructs an agent and calls
`run_agent_stream` without `bind_provider_persistence`, unlike the production
agent-loop path. Consequently, streaming through the real adapter alone does
not currently prove the checkpoint-writing path used in production. The gate
must bind that writer and retain the resulting genuine checkpoint records; it
must not remove them to make prefix/suffix assertions pass.

Both findings remain open source-verified blockers pending behavioral RED,
owner implementation, scoped `ci-test -j2` reruns and fresh-process evidence.
They add no passed tests to the retained seven-provider/eight-negative matrix
and do not change its existing scenario or coverage requirements.
