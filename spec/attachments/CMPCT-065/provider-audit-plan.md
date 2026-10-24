# Provider audit plan (2026-10-18)

Owner: a09f4feb. No board changes, commits, branches, or compiler runs yet.
The initial parent message was rejected as busy; this file is the shared handoff.

## Gaps and files

- Refresh stale scenario/test/implementation mappings for 065/066/069/074/075/
  078/080/081 and provider 071/082. The 080 invalid-checkpoint test moved into
  `providers/src/codex/history_state_tests.rs`; existing coverage incorrectly
  points at an unrelated integration-test range.
- Audit physical adapter paths, rather than accepting converter-only evidence:
  Anthropic terminal collector, custom adapters, Codex continuation/admission,
  generation checkpoint, full-current-DAG transport, and real RigAgent settlement.
- Strengthen 069 ordinary-provider runtime success/persistence-failure evidence.
  Existing 069 runtime coverage exercises Codex internal continuations only;
  existing persistence-failure coverage manually calls the finish hook.
- Check incomplete/inconsistent usage, absent measurements, and malformed replay
  for fail-open behavior. Add feature scenario and failing test before production.

## Ownership and coordination

All edits stay within providers, patched rig-core, and assigned feature/evidence
files. Core owns normalization consumers and durable settlement; runtime owns
CLI/background binding/Done projection. Cross-domain requests go to the parent.
No major extraction or oversized-file modification without approval.

## Historical evidence inspected

- `/tmp/cmpct065-green-066-incomplete-red.log`: existing provider normalization
  green followed by behavioral Anthropic cache/zero/missing-start failures.
- `/tmp/cmpct074-behavior-red.log`: aggregate continuation occupancy failure.
- `/tmp/cmpct078-080-081-red.log` and `/tmp/cmpct080-state-red.log`: prior
  lineage/replay negative runs; distinguish assertion failures from build errors.
- Existing attachments describe subsequent green runs, but coverage and final
  runtime claims remain incomplete and must be verified against current code.

Await parent cached-profile/common baseline results before cargo. Proposed
validation is package+target scoped, existing profile, `-j 2`, never workspace.

## Recovery update

Repaired unfinished 069 test setup after context recovery (no production edits).
The ordinary-provider success scenario already exists; persistence failure now
uses a real Anthropic adapter/RigAgent stream rather than manually invoking the
finish hook. Both need current validation. Exact prior behavioral red assertions
for 066/074/078/080/081 were inspected; `/tmp/cmpct080-state-red.log` is a compile
failure only, not behavioral red. Current process inspection shows no cargo job.
Request parent compiler slot for `cargo test --profile ci-test -j 2
-p codelet-providers --test cmpct069_request_settlement
--test cmpct069_runtime_settlement`, followed by remaining owned targets.

## Current verification

Custom extraction imports repaired in `rig_completion.rs`/`rig_streaming.rs`
against the existing compiler-red baseline. The scoped 069 command is queued
behind the sessions owner's active ci-test process, logging to
`/tmp/cmpct-provider-069-audit.log`; no independent target/profile was created.
Strengthening existing 078 compatible-continuation evidence with an actual DAG,
required-reminder identity and recorded tool outcome, and 080 replay rejection
with typed MissingOutcome assertions. No behavioral production changes made.
074/075 registered feature-group tags repaired (`@context-management`).
