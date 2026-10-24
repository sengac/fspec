# Replacement core/tools audit — 2026-10-18

Owner: `620d65fd-f508-40b7-8a8e-afdfdb6dc73c`, replacing unavailable
`368aa411`. No index, board, commit or other owner's production changes.

## Initial source audit

- Core captures `SessionManifest.provider` verbatim, including legitimate
  persisted composite identities such as `openai/o3`. Promotion and cold replay
  require exact equality. CLI must reconcile selected adapter identity with
  that canonical persisted identity, not weaken core freshness comparisons.
- Checkpoint F1 parsing failures belong to CLI preservation/application. Core
  replay retains checkpoint metadata and empty-content checkpoint records rather
  than converting them into conversation text. F2 checkpoint precedence belongs
  to providers/sessions: the synthetic envelope leads, authentic suffix follows.
  Do not discard or reorder immutable checkpoint suffix records to evade F2.
- Inherited evidence reports 36 core integration tests, 4 tools dispatch tests,
  and 7 inject-summary tests passing. These are historical results, not a fresh
  run by this owner. Core integration Clippy was blocked by sessions production
  `expect_used`; that dependency requires a fresh check after owner repair.
- Owned specifications currently map multiple integration targets in 055, 064,
  and 070. Strict board gates need target-preserving feature splits and actual
  step documentation. Existing tests/mappings must not be dropped or fabricated.

Parent notification returned `session_busy` and was not queued. This attachment
is the initial audit mailbox while work continues.

## New behavioral gap and current repair

The shared message cache could authorize capture, promotion and replay after
immutable backing storage disappeared. Specification and a new warm-cache
regression preceded production changes. Actual RED is
`/tmp/cmpct-core-warm-evidence-red.log`: the test executes and fails because
`capture_compaction_source` accepts a cached original after deletion of the
message JSONL. This is not a compilation failure.

Reference validation now opens an independent message-store view and replay
checks all committed references, not only covered references from a compaction.
GREEN is queued behind shared Cargo builds in
`/tmp/cmpct-core-warm-evidence-green.log`; no pass is claimed yet.

Inherited core regression has freshly passed 36 selected integration tests in
`/tmp/cmpct-core-replacement-regression.log`, before this repair and the
traceability-preserving target splits. Final rerun must include the new warm
regression and standalone lost-ack target. Own feature/sidecar splits remain in
progress. No other owner's features or code are being edited.

## Durable reasoning repair — 10:25 AEST

New independent-review gap reproduced behaviorally in
`/tmp/cmpct-core-reasoning-red.log`: the real terminal hook settles two requests,
duplicates terminal delivery, clears latest context, compacts and reconciles.
A fresh child process restores reasoning **11 instead of 48**, while a separate
overflow regression wrongly accepts another measured token at `u64::MAX`.
The unknown-detail control passes. New owned specification:
`durable-reasoning-settlement.feature`; shared provider features are untouched.

Minimal production repair adds checked cumulative reasoning in the same durable
physical-identity transaction, after duplicate/conflicting usage checks. No
reasoning is invented for unknown detail. Sessions/provider owners should rerun
their actual resume paths against this core change. GREEN and strict sidecar
verification follow; this is not an end-to-end acceptance claim.

Warm-cache GREEN is now confirmed: one selected regression passes in
`/tmp/cmpct-core-warm-evidence-green.log`. The previous selected final core tests
also finish green in `/tmp/cmpct-core-replacement-final-tests.log` before the
new reasoning repair. Core check, library Clippy and tools Clippy finish
successfully. Test Clippy identifies two inherited expects in whole-DAG tests;
owned test error propagation is being repaired before final quality rerun.
Parent message was busy/not queued; this attachment is the durable mailbox.
