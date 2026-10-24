# Sessions/UI/RPC resumed audit

Owner `04f7d44d`; shared tree preserved; no staging, commits, board updates or
CMPCT-083 test/helper/feature edits.

## Initial gaps and readiness

Latest inherited evidence is `sessions-owner4-status.md`. Current source still
installs boundary/checkpoint/counters before validating retained provider
envelopes. Reserved real fresh-process `cmpct061_restore_atomicity` covers four
malformed checkpoint cases; inherited result is 0/4 at reopened inference.
CLI must expose `Session::preflight_compaction_restore(manifest, envelopes)`;
sessions will call it before boundary installation, writer binding and replay.
Provider `validate_committed_restore` is present. Blocking inference after
partial mutation is not a repair.

Inherited `cmpct067_runtime_fill` is a real physical two-segment tool regression:
current occupancy 8000 is right but final displayed spend is 1700 output/300
reasoning instead of cumulative 3400/600. CLI final-response aggregation owns
the needed repair; no acceptance assertions will be weakened.

Invocation-identity converter already has genuine RED2/GREEN2 evidence and
structured replay GREEN4. Measured reasoning has a real physical-settlement,
compaction, fresh-process restore GREEN; no new reasoning repair is needed.
Their newer feature sidecars remain absent and will receive exact mapping.
No full-card readiness is claimed while atomicity and runtime-fill remain RED.

## Fresh verification underway

Shared `--profile ci-test -j2`, no alternate target directory:

- `/tmp/cmpct-sessions-owner5-reserved.log`: explicit atomicity, tool identity,
  measured reasoning targets.
- `/tmp/cmpct-tui-owner5-runtime.log`: explicit real runtime-fill target, after
  the sessions command finishes.

Direct initial coordination messages to parent and CLI were rejected busy and
not queued. This attachment is the durable mailbox.

## Fresh initial results

Both commands have completed: sessions exits101 only for atomicity target
(four fresh-child failures at reopened inference); identity passes2/2 and
measured reasoning passes parent1/1 plus fresh child1/1. Runtime fill exits101
with its single genuine aggregate-spend failure at line153. No production
change has been made; these are fresh behavioral REDs for the two APIs above.
The three newer specs now have test sidecars; atomicity intentionally has no
implementation mapping until a real repair passes. Its four Examples rows map
in order to the unsupported-version, unsupported-protocol, null-checkpoint and
future-generation tests in `cmpct061_restore_atomicity.rs`, each running the
shared `exercise`/fresh-process `probe` assertions.

## Completed scoped readiness checks

All use the shared `ci-test` profile with `-j2`; terminal Cargo summaries are
inspected, not inferred from filenames.

| Log | Result |
| --- | --- |
| `/tmp/cmpct-sessions-owner5-baseline.log` | Eleven selected targets GREEN: 21 parent tests, plus four fresh-child executions |
| `/tmp/cmpct-rpc-owner5-tests.log` | Committed wire snapshot 2/2 GREEN |
| `/tmp/cmpct-tui-owner5-baseline.log` | Coherent fill3, header provenance1, committed projection4: 8/8 GREEN |
| `/tmp/cmpct-sessions-owner5-check.log` | Three owned packages compile successfully |
| `/tmp/cmpct-sessions-owner5-clippy.log` | Three owned packages pass Clippy; inherited TUI collapsible-match warning and provider dependency warnings remain |

The three new sidecars pass read-only `fspec audit-coverage` (5/5, 1/1, 2/2
referenced files respectively). Their features each pass syntax and scoped tag
validation. Mappings identify actual converter identity preservation and saved
reasoning restoration statements; atomicity deliberately records only tests
while RED. Sidecar coverage percentages express linked scenarios, not passing
acceptance status.

An independent read-only bounds audit of all 060/061/062/067/068-tagged features
finds no owned missing file/scenario or out-of-bounds line mappings. The one
cross-owner stale mapping is loop-owned
`runtime-uncommitted-context-publication.feature.coverage`: references extend
through243 while `agent-loop/src/inject_summary_handler.rs` now ends240. It is
not edited here.

## Next integration

No owned production changes are justified until CLI's preflight API arrives.
Call it before `restore_compaction_boundary` in `session_restore.rs`, rerun all
four actual fresh-process atomicity cases, then link the passing implementation
statements. CLI also owns the runtime067 final aggregate-spend fix. Fresh RED
evidence is ready for both changes; no new fixture or weaker assertion is needed.
This is an initial readiness report, not completion of CMPCT-061 or CMPCT-067.

## Registry-limit repair in progress

Parent authorized the existing genuine CMPCT-083 Codex cold-process RED as
the regression for a sessions-only registry-resolution repair. Direct model
selection currently discards the registry limit metadata. Sessions will pass
known Codex registry limits into the existing provider direct-selection API;
unknown values stay unknown and provider zero defaults remain fail-closed.
No provider-manager edit, guessed fallback, fixture override or gate-test edit
is requested. Provider owner `c0448260` and gate owner `9d0c6da4`: this is the
durable coordination mailbox while your sessions are busy. Gate RED details
remain in `CMPCT-083/gate-owner-supervised-2026-10-18.md`.

## Registry repair fresh execution

`cargo test --profile ci-test -p codelet-sessions --test cmpct083_cold_process
scenario_repeated_committed_dag_cycles_codex -- --exact` now gets through the
fresh child successfully (1/1 GREEN). Parent gate is still RED at
`cmpct083/lifecycle.rs:198`: serialized wire contains `call_after_snapshot`
three times instead of two. This is a newly reached assertion, not complete
gate success. Log `/tmp/cmpct-sessions-owner5-codex-repair.log` preserves both
results. Gate/provider owners should inspect the actual wire before deciding
whether the third occurrence is a duplicate invocation or a preserved item-id
field. Do not weaken the assertion without proving correct pairing.

Actual failed wire extraction proves one `lookup` function_call with
`id == call_id == "call_after_snapshot"` plus one function_call_output with
that call_id, in that order. The third textual occurrence is the item `id`
field, not a third tool invocation. There is also the independent summary
submission pair (`fc_summary`). Gate owner must use structural pair assertions
instead of counting all serialized identity strings; production replay is not
shown duplicate by this assertion. The gate-owned test is not edited here.

CLI preflight API is still absent on the current filesystem. Sessions is ready
to integrate it before `restore_compaction_boundary`; all four actual atomicity
RED cases and the runtime067 aggregate-spend RED remain unchanged. Busy direct
message to CLI was rejected; this mailbox plus parent coordination is current.

## Preflight integration verified

The preceding API-absent status is superseded: CLI now exposes the isolated
provider preflight. Sessions invokes it after blocking inference but before
boundary/checkpoint/counter installation, writer binding or replay mutation.
`/tmp/cmpct-sessions-owner5-preflight-green.log` completes successfully:
atomicity GREEN4/4 plus four fresh-child executions; measured reasoning GREEN1
plus child; token restore GREEN3; background reconciliation, pending projection,
pending restore and restored counters each GREEN1 plus child. The original
atomicity RED assertions and invalid metadata are unchanged.

Fresh actual `cmpct067_runtime_fill` remains RED at line153 in
`/tmp/cmpct-tui-owner5-runtime-recheck.log`: final displayed output/reasoning
remain1700/300 instead of3400/600, with correct occupancy6000 input +2000
reserved output. CLI owner received the precise remaining aggregation failure;
no TUI assertion, provider usage or fixture was weakened. CMPCT-067 is not ready.

Final explicit sessions targets and exact semantic mapping refresh are underway.
Parent's latest direct message was rejected busy and not queued; this attachment
is the durable mailbox.
