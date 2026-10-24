# BUG-217 verification

Both frontend forwarders now use shared cursor delivery from a temporary disk
journal. Bounded broadcasts are wakeups, not the output authority. The relay
queue is bounded at 256 records with backpressure; caught-up cursors permit
reclamation, and dropping the last subscription releases its journal.

Strict ACDD included specification and four scenario tests before production.
Behavioral red failures observed lost initial output (`first-344` arrived first)
and a 600-record unbounded queue. The same four scenarios pass after the fix.
They cover controlled lag bursts, terminal ordering, independent subscriptions,
root routing, relay replacement, source closure, and cancellation.

Commands from `rust/` use `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`:

- Red/green: `cargo test -p codelet-agent-loop --test bug217_subordinate_forwarding -j 2`
- Final: `cargo test -p codelet-agent-loop --test bug217_subordinate_forwarding --test subordinate_manifest_single_writer -j 2` (7 passed)
- Check and library Clippy: scoped to sessions, tools, agent-loop, and NAPI.
- Test-target Clippy and `cargo check -p codelet-tools --tests -j 2` pass.

Logs: `/tmp/BUG-217-red.log`, `/tmp/BUG-217-green.log`,
`/tmp/BUG-217-final-tests.log`, `/tmp/BUG-217-check.log`,
`/tmp/BUG-217-clippy.log`, `/tmp/BUG-217-test-clippy.log`,
`/tmp/BUG-217-tools-tests-check.log`, `/tmp/BUG-217-fmt.log`, and
`/tmp/BUG-217-spec-quality.log`. Coverage audit and scoped tags/syntax pass;
all four scenarios map one-to-one to tests with implementation mappings.

The journal is not crash-recovery storage. A lagging cursor retains unread
records on disk; journal I/O is synchronous. I/O failure is explicit and stops
forwarding rather than fabricating successful delivery. No live WebSocket load
test was performed. Existing NAPI warnings are not suppressed.
