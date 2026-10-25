# BUG-263: honest Chat terminal evidence and settlement

## ACDD evidence

Discovery rules/examples and five scenarios preceded tests. `red-terminal.log`
is a fixture compilation failure, not a causal red. After fixing String coercion,
`red-terminal-causal.log` is a genuine 0 passed / 5 failed reproduction:
usage-bearing EOF without finish reason succeeded; malformed frames succeeded;
actual attributed Done concealed missing terminal usage; accepted Done had zero
billed input instead of 20; explicit DONE waited for EOF and hit its deadline.
Normal disposable admission registration and mandatory work_unit_id are retained.

The repair preserves progressive text/reasoning/tool deltas, but queues executable
calls until finish reason and consistent usage are closed by DONE or EOF.
Malformed/schema/error envelopes fail closed and preserve actionable cause/code.
DONE closes the SSE parser immediately, without requiring TCP closure. Validated
physical Final is emitted before executable tools, allowing the existing request
settlement hook to finish before native Done can drop consumption. No trailing
executable tools are drained. JSON adaptation validates nonempty finish reason
and consistent usage, and publishes usage/physical Final before queued tools.
Ordinary text-only response ordering is unchanged.

`green-terminal.log` passes all five new tests plus three BUG-262 ordering tests.
`green-final-matrix.log` passes new 5, ordering 3, accepted Codex batch 1 and 10
auto-continue controls, with one retained stale source-layout assertion failure.
`green-provider-matrix.log` passes all 7 provider streaming/JSON/cancellation
controls. Total affected matrix: 26 passed / 1 stale source assertion failed.
The unchanged failing auto-continue test reads Codex mod.rs for registration
that lives in agent.rs, exactly as recorded by the pre-batch coordinator audit.
CONT-011 owns replacing both stale layout assertions; this is not a new runtime
failure and is not silently repaired on this card.

After review, the fixture's capture await was moved before acquiring the output
mutex. `green-final-source.log` and `clippy-final-source.log` cover that final
fixture source. `check.log` passes scoped default CLI compilation. Reused helper
dead-code warnings remain, with no lint suppression. The excluded patched-rig
direct unit-runner limitation from BUG-262 still applies; no unit pass is claimed.
Its usage-only fixture now includes the required stop finish before usage.

## Direct review

Reviewed the SSE decoder, nonstreaming adapter and retained unit fixture, both
new test files, normal auxiliary admission and actual shared stream-loop hook/
Done early-exit path. Valid SSE and JSON Done cases assert exactly 20 input and
30 output billed once in the real persisted manifest, current context input 20,
accepted native Done result/status, paired call identity, one physical request,
no trailing filesystem effect and no ordinary nudge. Invalid cases cover missing
and inconsistent usage, malformed/error frames after a tool finish, and JSON
invalid usage, without hiding terminal failure behind Done. A gated loopback
response with one withheld byte proves explicit DONE does not depend on EOF.

All newly touched files remain below 300 lines. No credential, endpoint,
attribution, Done validation or shared acceptance guard was relaxed. No external
provider calls, downloads, commits or agent sessions were used. Scoped format,
whitespace, native spec/tag validation and five-scenario coverage audit are
checked before native Done. Final status is the native work-unit record.
