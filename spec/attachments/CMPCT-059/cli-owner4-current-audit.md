# CLI current runtime audit

Owner `d3fea5df`, coordinator `970b4a72`, 2026-10-18. CLI-only source ownership;
no staging, commits, board edits, acceptance weakening or cross-owner edits.

## Initial gaps and readiness

Read latest CMPCT-059/CMPCT-079 owner attachments and
CMPCT-083/traceability-current-owner-completion.md. Confirmed two real test gaps:

- Rejected summary submission checked only its first invocation. Added the
  actual second `run_agent_stream` call through `assert_reentry_blocked`, which
  asserts typed blocked result and unchanged physical request count.
- Automatic publication retained a conjunctive scenario without one matching
  test. Added `cmpct059_automatic_publication`, exercising independent real
  durable builder success and observer rejection executions within that exact
  scenario. Checks authoritative generation/context before completion, pending
  exact identity, no completion on failure, reentry without duplicate promotion,
  and explicit exact-identity reconciliation.

Composite identity already has six actual tests. Their fresh execution and
normal coverage relinking remain necessary; no new production defect is assumed.

Scoped command running with output in `/tmp/cmpct-cli-owner4-gap-tests.log`:
`cargo test --profile ci-test -p codelet-cli --test cmpct052_inference_handoff
--test cmpct059_provider_identity --test cmpct059_automatic_publication -j2`.
No new production code has been written. Pending execution is not GREEN or RED.
All features precede their new assertions. Full runtime readiness remains
unproven until these tests, normal coverage validation and quality checks finish.

Parent's initial direct message was rejected busy and not queued. This
attachment is the durable mailbox; it does not change board state.

Provider currently exposes separate `validate_restore` and
`validate_committed_restore` APIs, but the CLI proposed
`preflight_compaction_restore` is not implemented yet. Sessions/parent must
coordinate the existing genuine restore-atomicity regression before that repair.
The first gap-test invocation failed to compile the new fixture (missing
PartialEq on wire snapshot/manifest and incorrect boundary DAG field). These
are fixture errors, not behavioral RED; corrected against existing types.
