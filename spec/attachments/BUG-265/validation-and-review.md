# BUG-265 validation and direct review

## ACDD and runtime evidence

The seven-scenario feature and real shared-loop tests preceded the behavior fix.
`causal-red.log` and, after the separately approved mechanical extraction,
`mechanical-causal-red.log` each show five causal failures and two controls.
`final-regression-green.log` shows all seven acceptance tests passing. This is
real admitted input, synthetic loopback inference, automatic compaction,
loop-abort recovery and attributed native Done, not a replacement state model.
Production has not changed since that green run and both successful lint runs.

Only genuine input resets per-task nudges. Recovery uses the same synchronization
without that reset. Registry goals are written only when GoalSpec changes, so
unchanged goals retain lifetime rejections. Actual replacement/clear, generation
gates, atomics and arming before output/provider construction remain intact.
Reviewed the dispatch classification after admission, locked synchronization,
goal generation handling, registry comparison, constructor/field preservation,
channel ownership, and acceptance fixtures. No new admission/security policy,
runtime, dependency, credential, external request, or model change is introduced.

## Extraction and traceability

`module-plan.md` records the exact native split approval. Mechanical token and
shape audits preserve the original methods/types/defaults and all 14 shape tests
with all 67 original assertion predicates. These are structural audits, not
behavioral proof. All extracted source/test modules are below 300 lines.
Private sibling helper reachability uses only pub(super), documented in the
mechanical logs. Existing interruption/agent-message/handoff modules stay intact.

Native coverage rebasing records all moved scenarios in
`mechanical-coverage-rebase.jsonl`; the two contradictory historical RPC-039 steps
were reconciled only under the recorded native approval. The RPC-043 coverage
maps its unchanged NAPI test, not the moved child tests, and needs no line rebase.
Six root compile-time function-pointer witnesses reference the actual moved
scenario functions, retaining the old source-reader names without dummy tests.

Final review strengthens the BUG-228 guard to require both genuine-input and
recovery synchronization and to reject removal of either. Its final test passes.
`final-test-source-review.log` passes nine nonbuilding shape tests after the
witness edits. `final-wiring-lint.log` passes formatting, the strengthened guard
and scoped Clippy for both changed sessions test targets. Dependency warnings
are retained, not suppressed. Prior `final-session-controls.log` and
`final-cont008.log` pass 33 controls; with seven acceptance tests, 40 distinct
runtime/control checks pass. Repeated checks are not counted as new tests.
`final-cargo-check.log` and both crate-specific final Clippy logs passed.

## Honest exclusions and baseline limitations

Five shape scenarios invoking nested Cargo builds/metadata/TypeScript generation
remain excluded under resource constraints. The RPC-043 NAPI scenario also
invokes a nested default-profile Cargo check and was not executed. Compile-time
witnesses do not claim that these subprocess tests ran.

The changed CMPCT-041 source reader correctly reaches the extracted metrics
accessors, then fails on the old NAPI CompactionStarted arm expectation:
`final-test-source-review.log` preserves the failure. NAPI delegates to the shared
engine rather than containing that arm; CONT-011 owns repairing this stale
reader while preserving its real accessor/writer assertions. This is not a green
test, and no complete workspace-green claim is made. The failed chained command
did not run its final Clippy; the separate final wiring/lint command did.

Native feature formatting/validation and seven-scenario coverage pass. Audits
also pass for RPC-039, RPC-041 and RPC-043 mappings. Global native tag validation
reports 522 unrelated feature violations; the exact output is retained in
`final-native-validation.log`, not repaired outside scope. Native file-scoped
tag validation passes for the BUG-265 feature. The original history logs remain
chronology, not current-state claims. No commits or publication are performed.
