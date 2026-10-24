# CMPCT-083 gate audit and file plan

## Starting evidence (2026-10-18)

- Existing positive outline is Anthropic-only. Its request helper copies restored
  messages into a separate adapter rather than invoking the selected session's
  inference path. `durable::commit` directly calls the persistence API, not the
  runtime candidate submission/application path. Two commits use the same text.
- The cancellation test aborts an unrelated pending task holding a candidate. This
  does not exercise production working-view preparation cancellation.
- Notification loss directly calls the persistence publisher and reconciles before
  resume, so it does not prove the resumed inference barrier before resolution.
- No negative matrix test exists. Coverage currently maps only one of four
  scenarios (25%); this is inventory, not evidence that its seven provider rows pass.
- `/tmp/cmpct083-cold-red.log` ends in rig-core Gemini compilation errors and is
  not behavioral red evidence. `/tmp/cmpct083-cold-current.log` contains a later
  actual behavioral red: all three existing tests fail with
  `Preparation("RequestError: selected provider has no physical request serializer")`;
  result: 0 passed, 3 failed, 0 ignored (0.27s). This supersedes the earlier
  compilation-only description in `gate-coordination.md`.

## Owned file plan

1. Keep the four acceptance scenarios and explicitly require real runtime candidate
   application, selected physical adapters, exact suffix order, failure barriers,
   and fresh-process pending-publication reconciliation.
2. Replace `rust/sessions/tests/cmpct083_cold_process.rs` orchestration and split
   helpers only under `rust/sessions/tests/cmpct083/`, below 300 lines each.
3. Add the negative scenario as `rust/sessions/tests/cmpct083_failures.rs`, using
   real stores and transport capture, typed errors, deadlines, byte comparisons,
   no completion, and runtime inference barriers.
4. Record exact per-provider/per-failure results and scenario-to-test mappings in
   this attachment directory and the owned feature coverage file. Never count a
   synthetic task cancellation or generic fixture error as a production failure.

## Coordination

Cargo initially deferred to parent shared baseline. Requested baseline command:
`cargo test --profile ci-test -p codelet-sessions --test cmpct083_cold_process -j2 -- --nocapture`.
Production API gaps go through parent. No production, board, branch, or commit
changes are owned here. Existing staged work remains preserved.
