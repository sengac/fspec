# Supervised CLI API state

Current CLI owner started from retained code, without staging or board edits.

- `Session::preflight_compaction_restore(&self, &SessionManifest, &[String])`
  already exists and calls the provider's nonmutating prospective restore
  validation before any CLI mutation. Sessions now calls it before installation.
  Sessions owns `cmpct061_restore_atomicity` acceptance verification.
- Current final-response path still ignores aggregate output/reasoning when live
  Usage already supplied authoritative segment values. Genuine retained RED is
  `/tmp/cmpct-cli-owner4-fill-red.log`: physical occupancy 8000 remains correct,
  but spend is 1700/300 instead of 3400/600. CLI is repairing this boundary.
- Current terminal recovery still consumes its in-loop budget before handing
  the original error to the outer owner. Genuine retained RED is
  `/tmp/cmpct-cli-owner4-terminal-red.log` (2 passed, 1 failed).

## Loop owner integration request

CLI now exposes
`compactor_sub_agent::set_terminal_overflow_owner(session_id: Uuid, present: bool)`.
Loop owner should call it with `true` in `CompactorRegistration::new` and `false`
in `Drop`. The stream loop will yield the original structured overflow before
any in-loop recovery mutation only while this explicit marker is present.
Standalone CLI compaction (even with a builder handler) keeps its current route.
Ordinary turn handler cleanup remains entirely loop-owned and unchanged.

Direct parent and loop messages were rejected busy, not queued. This mailbox
records the exact seam; it does not claim terminal acceptance is complete.

Fresh actual behavioral RED before CLI changes:

- `cargo test --profile ci-test -p codelet-fspec-tui --test cmpct067_runtime_fill -j 2`
  failed 0/1 in `/tmp/cmpct-cli-supervised-fill-red.log`, missing 3400/600 spend.
- `cargo test --profile ci-test -p codelet-agent-loop --test cmpct054_terminal_dependencies -j 2`
  failed 2/3 in `/tmp/cmpct-cli-supervised-terminal-red.log`, in-loop retries
  exhaust before the required outer boundary.

Spend GREEN after the minimal CLI final-response repair:
`cargo test --profile ci-test -p codelet-fspec-tui --test cmpct067_runtime_fill -j 2`,
1 passed, 0 failed, exit 0 in `/tmp/cmpct-cli-supervised-fill-green.log`.
The same unmodified test still asserts two physical segments, occupancy 8000,
separate aggregate 3400/600, and matching backend/TUI current request fill.
Sessions/UI retains ownership of that test, feature and coverage sidecar.

The original aggregate FinalResponse supplies generated spend; its separate
latest physical request snapshot supplies occupancy/cache split. CLI does not
sum physical occupancy or add already-displayed segment totals a second time.

## Outer-owner continuation

Loop registration now sets and clears the explicit marker. The unchanged
terminal dependency test reaches the captured outer builder with exactly two
physical requests and proves ordinary handlers are gone. Its new genuine RED
is `/tmp/cmpct-cli-supervised-history-red2.log`: the first rejected user prompt
was still appended to live history. CLI now removes only that unaccepted prompt
when no assistant text, reasoning, tool work or further history was produced.
Completed tool turns are not rolled back. The original structured error and
blocked inference survive the handoff.

`has_terminal_overflow_owner` is now public for loop lifecycle acceptance.
The first post-repair run `/tmp/cmpct-cli-supervised-history-green.log` did not
execute tests: the concurrently edited loop target calls this query at line 221
without importing it. Loop owns fixing that import and rerunning the unchanged
terminal assertions; this compile failure is not GREEN evidence.
