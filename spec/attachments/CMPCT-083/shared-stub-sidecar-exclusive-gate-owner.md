# Shared stub sidecar ownership — 2026-10-18

At 11:46 UTC coordinator `59a366f8` explicitly assigns exclusive write ownership
of `spec/features/stub-provider-physical-serializer-parity.feature.coverage` to
gate owner `414c4991`. Provider/Rig owners must not regenerate or overwrite this
shared CMPCT-071/083 sidecar during final validation. Provider source and tests
remain outside the gate owner's write scope.

The gate owner restores exact complete attributed test spans `12-59` and
`61-109`, including both Tokio and serial-test attributes. Current mapped-line
bookkeeping is 264: test spans contribute 48 + 49 lines and the retained
implementation mappings contribute 75 + 92 lines. The initially requested 268
is stale arithmetic: both the gate auditor and provider owner's independent
auditor reject it with an actual computed total of 264. No artificial mapping
lines are added to reach the requested number. Production mappings and all
assertions are retained.
The existing strict auditor is not weakened. This repairs the failed mapping
validation recorded in `/tmp/cmpct083-final-board-confirmed-traceability.log`;
it does not represent production behavioral RED or measured execution coverage.

The final integrated runtime gate waits for the coordinator's production freeze
because CLI is repairing genuine structured-overflow behavioral RED. Earlier
21/21 runtime results are retained but are not substituted for the required
post-repair, stable-source gate execution. The coordinator message reporting
this arithmetic correction was rejected busy and was not queued; this note
preserves the correction and exclusive ownership handoff.
