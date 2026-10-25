# BUG-255 ACDD evidence and native hold

Historical pre-authorization record, preserved below. The human's new informed
authorization and single successful native retry are recorded separately in
`authorization-and-transition-retry.md`; subsequent work is in `acdd-validation.md`.

Actual lifecycle status: **testing**. No production repair implemented.

Discovery, rules, answered question, user story and four scenarios were recorded
through native Fspec. Feature syntax and registered tags validate successfully.
The native one-feature/one-test-file requirement was satisfied by consolidating
the tests into `rust/core/tests/bug255_append_scaling.rs`.

Command run from `rust/`:

```text
cargo test --offline --profile ci-test -p codelet-core --test bug255_append_scaling -j2
```

Result: **2 passed, 2 failed**, with successful compilation. Failures are behavior:

- 256 deltas reread **33,664** historical records, exceeding the linear bound.
- An unrelated session times out behind the process-global recovery mutex while
  the first transcript writer is held by a real filesystem lock.

Cold intact/corrupt/incomplete opening and failed-directory-acknowledgement retry
controls pass. Two observation labels in the existing durability observer count
the current implementation's actual historical reads and lifecycle work; they
do not change admission behavior. No optimization has been implemented.

Native `update-work-unit-status` to implementing was rejected with:

```text
RLCD gate: P(hold)=0.38770774 (argmax hold; hold threshold 0.65) — the engine judged this workflow command mistimed or risky
```

Per explicit user instructions, this hold was not retried, bypassed, or followed
by gate configuration changes. Status remains testing, not blocked or done.
Proceeding to independent BUG-256 requires no relaxation of this hold.

Subsequent integrity controls ran offline with the same ci-test profile and -j2:
`cargo test --offline --profile ci-test -p codelet-core --test bug249_journal_admission --test bug249_journal_integrity --test bug249_transcript_journal --test bug250_transcript_lifecycle -j2`.
All **14** existing admission, corruption, ordering, tool lifecycle, storage,
promotion suppression and deletion controls pass. This does not resolve the two
BUG-255 behavioral reds or authorize the held implementing transition.

Disk free after scoped builds: 9.2 GiB, above the 6 GiB floor. BUG-253 and BUG-254
changes remain preserved and uncommitted. No release workflow was triggered.
