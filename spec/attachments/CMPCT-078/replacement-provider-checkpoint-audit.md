# Replacement provider owner — checkpoint coordination

Owner `042b9b3c` replaces provider owner382. Scope remains `rust/providers`
and non-provider Rig modules; builtin owner `d97a39a4` retains Rig provider
subtree, builtin serializer targets/features and Anthropic terminal feature.
Messages to parent and builtin owner were rejected as busy (not queued).

F2 is confirmed in current source: CodexSession::restore selects the last
checkpoint even after committed generation installation. I am adding a
provider-specific feature and integration target for committed-boundary
precedence and incompatible suffix rejection, obtaining behavioral RED before
repair. Gate owner should independently exercise the real fresh-process
SessionManager restore with a genuine retained old-generation checkpoint.
Sessions owner need not reorder restoration to hide the provider defect.

F1 remains CLI-owned: checkpoint metadata must survive while empty
checkpoint-only assistant content stays out of conversational replay. Gate
owner should bind the actual provider persistence writer. No shared gate tests
will be edited by this provider owner.

No board changes, staging, or commits. Final evidence and strict provider
traceability audit pending.

Behavioral RED now recorded in
`/tmp/cmpct-provider-replacement-f2-red.log`: three tests executed, one passed;
retained pre-promotion checkpoint restores generation 0 instead of committed 1,
and future-generation suffix metadata is accepted. This is executable assertion
evidence, not a compiler failure. Repair validates the whole replay before
mutation, honors the explicit synthetic committed boundary, rejects incompatible
suffix metadata and preserves same-generation genuine continuation.

GREEN: `/tmp/cmpct-provider-replacement-f2-green.log` runs all three new
precedence regressions, three existing lineage tests and four existing replay
integration tests successfully (10 total), with `ci-test`, `-j 2` and
`codelet-providers --features test-support`. Fresh-process gate evidence remains
the gate owner's responsibility, not claimed by this provider-only run.

Strict traceability source audit: shared 065 maps core plus two provider targets;
shared 069 maps loop plus two provider targets; shared 071 maps CLI plus two
provider targets; shared 082 maps CLI plus two provider targets. Parent and
traceability messages were busy-not-queued. Provider ownership of those scenarios
is stable; approve transfer/splitting before touching shared specifications.
Own 080 invalid-state unit scenario is split losslessly to
`codex-invalid-checkpoint-state.feature`; public replay feature retains its four
real integration scenarios. New precedence feature maps only its own real target.

## Shared traceability handoff

Parent's current plan approves lossless splits by actual test target. The eight
provider siblings now preserve exact scenario wording/examples from shared
065/069/071/082. Provider owner retains their sidecars and actual test docs.
Shared-feature/sidecar owners must remove the transferred rows (not the tests):

- `inclusive-provider-usage`: last two custom scenarios transfer to
  `custom-provider-usage-normalization` and `custom-provider-runtime-usage`.
- `complete-token-event-forwarding`: four ledger scenarios transfer to
  `physical-request-settlement`; two runtime scenarios to
  `runtime-request-settlement`. Keep the two loop JSON scenarios.
- `whole-request-admission-before-dag-commit`: four serialized scenarios transfer
  to `serialized-request-admission`; two preparation scenarios to
  `selected-provider-request-preparation`. Keep the four CLI candidate scenarios.
- `explicit-request-admission-without-fallback`: two physical/lock scenarios
  transfer to `provider-physical-admission`; cancellation outline to
  `provider-hook-cancellation`. Keep four CLI limits/classification scenarios.

Messages to parent and trace owner were again rejected busy-not-queued. This
attachment is the durable handoff; no shared source feature/sidecar was changed.
The temporary duplicate source scenarios must be removed by that owner before
declaring strict traceability complete. New siblings are documentation partition,
not new acceptance behavior or additional test results.

## Repeated-restore gap and disk guard

The additional independent review gap is confirmed in `history_generation.rs`:
installing the original empty G+1 boundary rejects an already extended G+1
continuation because it compares the entire checkpoint. Provider owner is adding
two scenarios to the owned precedence feature and its integration target before
repair: real physical completion then repeated boundary-install/restore/next
physical request, and incompatible same-generation model refusal.
Sessions/gate owners should exercise their real SessionManager restoration.

At this checkpoint `df -k .` reports 25,477,668 KiB available (24.30 GiB), below
the required 25 GiB floor. No new Cargo command is started until sufficient
headroom exists. Parent notification was busy-not-queued; please coordinate safe
shared-artifact cleanup. Existing source edits, tests and index remain intact.
