# BUG-266 validation and direct review

## Approved contract and ACDD

Native question `bug266_settlement_and_split` received **Approve exact contract**
from the supervising caller acting under the user's explicit in-scope grant,
not fabricated new human speech. Stop alone is nonfinal; preserve trailing
usage and typed transport errors before authoritative [DONE] or clean EOF after
stop. [DONE] settles even while the connection remains open. No timeout, error
masking, endpoint/security change, or new protocol policy was approved or added.
The same response approved extracting only the public chunk/usage types into
stream_types.rs, retaining paths/types/defaults before behavior changes.

Rules, examples and all seven scenarios preceded tests and production. The
native one-feature/one-test gate rejected the initial two-target feature;
the original native-Done scenario was moved unchanged to its own feature, with
the same test and criteria, before the one permitted transition retry succeeded.
Both features retain one-to-one scenario/test coverage, seven tests total.

`causal-red.log` shows four causal provider failures and two controls: trailing
usage lost, and an executable tool escaped before post-stop transport failure.
Its initial CLI test did not compile because a reused cross-crate helper needed
dependencies absent from CLI; this is not causal red. Subsequent CLI attempts
had no HTTP requests (manager setup / missing script tool catalog), also not
causal red. The corrected script preserves the real attributed tool catalog;
`causal-native-done-red-admitted.log` reaches accepted native Done but records
zero billed input instead of twelve. All invalid attempts remain preserved.

`baseline-controls.log` passes twelve BUG-227 controls. The pure extraction
then passes those same twelve and repeats the same four causal failures/two
controls in `mechanical-controls-red.log`. `mechanical-types-audit.log` proves
the moved type block byte-identical to immutable b2475dcd and the processor
source identical except its re-export; this is structural, not behavior proof.

## Minimal repair and runtime evidence

Content stop flushes the existing low-level stop/tools without terminating the
processor. Further decoded maps permit only usage/token updates; text/reasoning,
additional calls and repeated stops cannot extend content. Parser errors remain
errors. The HTTP bridge's authoritative [DONE], EOF and typed error policy is
unchanged. There is no speculative success for a stop-only pending connection.

The actual custom rig adapter buffers executable calls until successful stream
settlement, emits exact FinalResponse first, then releases calls in order.
Errors drop buffered calls; attributed native Done cannot short-circuit trailing
usage or physical settlement. Incremental text/reasoning/argument deltas remain
compatible and are not executable calls. Public types remain re-exported through
the original paths; all touched source/test files are below 300 lines.

`green-settlement-matrix.log`: six new provider tests, twelve BUG-227 controls,
six reasoning tests, eleven SSE-bridge tests and one new native-Done test pass:
36 distinct passes. The additional selected real rig streaming keystone test
passes in `final-quality.log`, for 37 distinct checks. No full workspace-green
claim is made. Every actual request uses synthetic numeric-loopback fixtures;
stop/DONE and cancellation tests use real still-open HTTP connections.
The native-Done test checks ordinary admission, accepted completion, no invalid
attribution, exact persisted physical billing 12 input / 3 output, one request
and no nudge. Its final-source rerun passes in `final-native-done-quality.log`.

Default affected-crate library check and scoped provider/CLI Clippy pass.
Review fixed the new test's lock-across-await warning; final lint retains only
six reused-fixture dead-code warnings. No new suppression or dependency was
added. Scoped Rust formatting and whitespace checks pass. Reviewed processor
state/array order, late usage aggregation, error short circuit, delayed executable
calls, ownership/cancellation and the real shared-loop settlement path.

## Native traceability and gated reconciliation

The initial hand-written feature needed a native coverage sidecar. To avoid
global generator mutations, native generate-coverage ran against disposable
projects containing only the exact feature; its generated sidecar was copied
unchanged before normal native links. No coverage JSON was hand-edited.
The moved reasoning/SSE source mappings were updated through native link-coverage.
Both new features and both affected legacy coverage audits pass, as do syntax
and native file-scoped tag validation. Global tag validation's unrelated failures
are documented by BUG-265; they were not changed or claimed fixed.

Native test-line relinking appended a duplicate mapping; implementation links
updated only the first. The validating gate correctly denied incomplete coverage.
The first attempted repair also failed because native unlink removes only the
first matching test mapping. No status write executed on either denial.
Inspection established the remaining duplicate and source semantics. Native
question `bug266_duplicate_mapping_reconciliation` received **Approve exact
repair**: the supervising caller explicitly authorized clearing only the exact
native-Done scenario using native --all, restoring one real test plus the three
implementations, auditing, and one gated retry. The before/after evidence and
successful normal validating transition are preserved in
`approved-mapping-reconciliation.log`. No gate was relaxed or bypassed.

No sessions/subagents, model changes, compaction actions, external providers,
credentials, cache cleanup, commits or publication are performed.
