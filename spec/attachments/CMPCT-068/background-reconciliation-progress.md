# Active sessions/RPC/TUI owner checkpoint

Owner `37a35e4c` retains production ownership. Existing060/061/062/067/068/073
feature partitions and tests remain handed off to traceability `36c99684`.
Only `cmpct067_runtime_fill.rs` remains mutable: the inherited distinct-response
IDs did not repair the fixture; its rerun still sent one request and failed the
aggregate spend assertion. A real tool/internal continuation is required.

New mutable target/specification reserved to this owner:
`cmpct068_background_reconciliation.rs` and
`background-compaction-publication-reconciliation.feature`.
The existing cold gate reconciles only the inner CLI Session, leaving the actual
BackgroundSession projection empty. Reproducing through that real old entrypoint
before adding a background-owned exact-identity reconciliation front door.
The gate owner must adopt that background front door after it is available;
this owner does not edit CMPCT083 tests.

All other inherited session/RPC/TUI regression targets remain stable for step
documentation and actual-range mappings. Busy messages are not queued; this
attachment is the coordination mailbox.

## Current runtime067 / background068 evidence

Background reconciliation is GREEN: `/tmp/cmpct068-background-reconciliation-green.log`
passes parent 1/1 and fresh child 1/1. Gate owner may use
`BackgroundSession::reconcile_durable_compaction(&identity).await` instead of
reconciling the inner CLI Session; this republishes saved context once without
synthesizing completion. Production API and test are stable for gate adoption.
Traceability may take the new background feature/test after scoped quality.

Runtime067 remains reserved. `/tmp/cmpct067-runtime-tool-consistent.log` now
executes the real Inspect tool continuation without checkpoint lineage error:
the completed first response contains the actual function call rather than
unrelated text. The test still fails 0/1 at cumulative spend: all five token
updates report 1700 visible and 300 reasoning, never 3400/600. Latest physical
occupancy is correctly 8000. CLI/core owners should inspect per-segment usage
aggregation; do not weaken the cumulative spend acceptance.

Sessions owner is preparing the new SessionManager restore-atomicity regression
requested at 10:55, before any production ordering repair.

### Restore-atomicity behavioral RED and required coordinated API

New specification `session-checkpoint-restore-atomicity.feature` validates and
`sessions/tests/cmpct061_restore_atomicity.rs` executes the real SessionManager
restore with an actual CodexSession extension and committed boundary. The
initial `/tmp/cmpct061-restore-atomicity-red.log` was fixture setup failure (not
behavior evidence). `/tmp/cmpct061-restore-atomicity-red2.log` is genuine RED:
0 passed / 4 failed, each at `failed restore reopened session inference` for
unsupported version, unsupported protocol, explicit null and future generation.
Assertions also preserve the entire prior checkpoint/messages/token tracking/
replay installation/background output and exercise a subsequent valid retry.

Provider and CLI messages were rejected busy, not queued. Provider owner needs
to expose non-mutating complete-envelope restore validation (or a staged
transaction) against prior checkpoint and committed boundary; cloning
ProviderSession is not isolation because extensions share Arcs. Sessions can
then validate before fallible binding/boundary installation. CLI owner must
coordinate boundary preflight and avoid resetting an already continued
same-generation checkpoint. A simple error reblock after mutation does not
satisfy these tests. No production atomicity repair has been attempted yet.
