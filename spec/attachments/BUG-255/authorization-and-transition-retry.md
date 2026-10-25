# BUG-255: informed authorization and one native transition retry

## New human authorization

The assistant explained that native Fspec refused the testing-to-implementing
command before execution: its RLCD checker returned hold, not a failing test or
an AskUser approval request, and identified no specific missing prerequisite.
The human responded at 06:57 UTC on 2026-10-25:

> no just approve it, get 255 done

The new task expressly authorizes at most one native transition retry with this
materially new informed authorization in the normal reason argument. It does
not authorize bypassing a gate, fabricating a pending approval, AllowSession,
changing security configuration or trying another wording/session after a hold.
If the retry is held or rejected, implementation must stop. If it asks the user,
the exact pending request must be preserved for normal approval.

## Prerequisites and current-code evidence

Native `show-work-unit` confirms **testing**. Its lifecycle history contains only
specifying and testing: the previous held implementing command never executed.
The discovery question is answered; there are no blocking dependencies or child
cards. Four scenarios have four native test mappings in the existing coverage
sidecar; empty implementation mappings are intentional before the repair.

Current source inspection finds only two observation labels for BUG-255 in
the journal store and lifecycle projection, not an admission optimization.
The journal still reads/projects all prior records on append, and recovery still
holds the global owner mutex over filesystem work. Prior-card changes remain
present; BUG-253/256/257/258 are done and BUG-254 remains specifying.

Fresh command, run from `rust/` before any repair or retry:

```text
cargo test --offline --profile ci-test -p codelet-core --test bug255_append_scaling -j2
```

`red-current.log` records **2 passed, 2 failed**, exit **101**, no ignored or
filtered tests. The same genuine failures remain:

- `long_streaming_sessions_require_bounded_admission_work`: 256 deltas reread
  **33,664** historical records.
- `isolation::an_unrelated_session_progresses_during_a_stalled_journal_append`:
  **unrelated session blocked behind global recovery lock**.

Cold intact/corrupt/incomplete opening and exact-identity publication retry pass.
The earlier 14 related controls and original hold evidence remain preserved in
`acdd-red-and-native-hold.md`. Disk after this scoped run is approximately
12.6 GiB, above the strict 6 GiB floor. No repair has started. Repository
`AGENTS.md`, native lifecycle help, current feature/tests and `docs/TESTING.md`
were read; `.agents/skills` is absent in this checkout.

## Outcome

The single authorized native retry succeeded at **07:03:53.307 UTC**:
`Work unit BUG-255 status updated to implementing`. The supplied normal reason
included the exact informed direction, fresh red results and prerequisite review.
There was no AskUser request, approval override or AllowSession grant. Production
repair began only after that success. Subsequent ACDD validation and closure are
recorded in `acdd-validation.md`.
