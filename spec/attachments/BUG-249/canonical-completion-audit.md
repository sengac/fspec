# BUG-249 canonical completion audit

## Scope and preservation

This audit owns BUG-249/250 transcript, archive, lifecycle and display-only replay
validation. Existing staged changes are preserved. No production edits, direct
compiler invocations, commits, dependency changes or alternate target directories
are used by this completion audit. Existing historical direct-runner evidence is
not counted as a substitute for Cargo execution.

## Verified canonical results

`/tmp/bug246-250-final-integrated-cargo.log` completed at 18:06 on 2026-10-23.
The BUG-249 targets all pass:

| Package | Target | Passed |
| --- | --- | --- |
| codelet-core | bug249_journal_admission | 5 |
| codelet-core | bug249_journal_integrity | 2 |
| codelet-agent-loop | bug249_admission_ownership | 1 |
| codelet-agent-loop | bug249_executor_gate | 1 |
| codelet-agent-loop | bug249_interrupted_tool_state | 1 |
| codelet-agent-loop | bug249_stream_crash | 4 |

The crash target's one ignored test is a subprocess entry point explicitly run
by its four passing parent tests, not omitted acceptance coverage. These results
verify 14 of this card's 18 scenarios. The remaining four core transcript-journal
tests are invoked in `/tmp/bug249-250-completion-cargo.log` with the existing
`release-slim` profile and `-j2`; pending compilation is not GREEN.

## Traceability

`/tmp/bug249-250-completion-coverage.log` records successful `fspec audit-coverage`,
`fspec show-coverage` and scoped `fspec validate` for all 16 BUG-249/250 features.
BUG-249 has 18/18 scenarios across six features, with exactly one test mapping
per scenario. A separate source-span audit finds every test range contains a
test function and every test/implementation range is within its current file.
This is acceptance traceability, not a claim of measured branch coverage.

Scoped `rustfmt --check --edition 2021` passes all owned core transcript/archive/
lifecycle, stream journal/background output, replay decoder and BUG-249/250 test
files: `/tmp/bug249-250-completion-format.log`, `FORMAT_EXIT=0`.
Global `fspec validate-tags` still reports 526 unrelated feature violations and
1582 passing files in `/tmp/bug249-250-completion-tags.log`; no BUG-249/250 feature
is reported there. Independent registered-tag and feature-level ID checks pass
all 16 owned features. Global tag validation is not represented as GREEN.

## Outstanding gates at initial recording

The completion Cargo chain also invokes scoped core/agent-loop/sessions
`cargo check` and strict library `cargo clippy --no-deps -- -D warnings`.
The supplied `/tmp/bug242-252-final-clippy.log` actually fails on a TUI
`collapsible_match` lint; it is not blanket passing quality evidence.
No card is advanced to done while its required gates remain pending.

## Missing core target executed

`/tmp/bug249-250-completion-cargo.log` now records canonical Cargo GREEN for
`bug249_transcript_journal`: **4 passed, 0 failed**, after the existing target
rebuilt in 17m 42s. This closes the runtime execution gap: all **18/18** BUG-249
scenarios have passing canonical Cargo results. No new repair was needed.
Scoped library check/Clippy are still queued; status remains validating.

## Final quality GREEN and completion decision

The entire `/tmp/bug249-250-completion-cargo.log` chain has now completed with
**`CARGO_COMPLETION_EXIT=0`**. Its final two commands actually executed:

```sh
cargo check --profile release-slim -j2 -p codelet-core -p codelet-agent-loop -p codelet-sessions --lib
cargo clippy --profile release-slim -j2 -p codelet-core -p codelet-agent-loop -p codelet-sessions --lib --no-deps -- -D warnings
```

Both pass. Earlier pending/broken broad logs are not used as passing evidence.
The complete canonical test/quality outputs are now preserved in the repository:

- `spec/attachments/BUG-249/bug246-250-final-integrated-cargo.log`: the 14 passing
  BUG-249 scenarios listed above, plus 19 BUG-250 scenarios and related BUG-246
  regressions.
- `spec/attachments/BUG-250/bug249-250-completion-cargo.log`: the remaining four
  BUG-249 scenarios, six remaining BUG-250 scenarios, and both quality commands.

Together they establish **18/18 BUG-249** and **25/25 BUG-250** distinct passing
acceptance scenarios, with no ignored acceptance scenario. The two ignored
subprocess entry points are explicitly invoked by their passing parent tests.
The refreshed scoped fspec audit confirms **43 unique in-bounds mappings across
16 features**, all syntax-valid. Scoped formatting passes.

No BUG-249 production repair is needed in this final audit. The only Rust edit
is the BUG-250 cold fixture correction documented in that card's audit, after
actual canonical runtime RED. Existing staged content is preserved; no commits,
direct compiler fallback, lint suppression or manual work-unit edits are made.
There is no remaining owned behavioral, scenario-mapping or library-quality gap.
The done transition is performed through fspec only after these results.
Broader supervisor-owned failures and test-only warnings remain explicitly listed
in BUG-250's final scope limitations; this is not whole-workspace GREEN.

## Board completion

`fspec update-work-unit-status BUG-249 done` succeeds after the final gates.
`fspec show-work-unit BUG-249` independently confirms **Status: done** in
`/tmp/bug249-completion-status.log`. No temporal/workflow bypass is used.

## Coordinator standards correction: one feature per test file

The coordinator correctly identified that the prior 16-feature audit verified
one test mapping per scenario, but did not enforce one test file per feature.
`transcript-journal-integrity.feature` still combined the two ordering/integrity
tests with five tests from `bug249_journal_admission.rs`. The prior done state
was reopened through normal fspec states to correct that specification boundary.

Using the coordinator's fspec-only factoring helper, the five unchanged admission
scenarios and their complete mappings move to
`spec/features/transcript-semantic-admission.feature`. The two ordering/integrity
scenarios remain in `transcript-journal-integrity.feature`. The only additional
Rust edit is the first-line feature documentation header in
`rust/core/tests/bug249_journal_admission.rs`; test bodies and production are
unchanged. Original executable behavioral RED and canonical Cargo GREEN continue
to refer to the same tests, not newly invented acceptance criteria.

`/tmp/bug249-specification-factor.log` records the fspec commands. The independent
verification in `/tmp/bug249-factored-coverage-verification.log` proves:

- All seven original scenario names and every Given/When/Then step are unchanged.
- All seven complete test/implementation mapping objects are identical before and
  after factoring; only their owning feature changes for the selected five.
- All **17** BUG-249/250 features now map to exactly **one test file each**, with
  matching test feature headers and exactly one mapping per scenario.
- Counts remain **18 BUG-249** and **25 BUG-250** scenarios. Both factored features
  pass fspec syntax/path audits, and registered-tag checks and rustfmt pass.

The ordinary state sequence is **done -> specifying -> testing -> implementing
-> validating -> done**, with no skip flags. The normal validating gate succeeds
after the split. This section supersedes the earlier 16-feature count and the
earlier incomplete feature/file-boundary audit.

No fresh Cargo execution is claimed for this documentation-only factoring. The
registered actual seven integrity/admission test results, all 43 scoped scenario
results and the final three-package Cargo check/strict Clippy exit 0 remain the
execution evidence: neither their test bodies nor production changed. The latest
quality log is inspected and still ends `CARGO_COMPLETION_EXIT=0`. This avoids a
redundant relink after the reported mid-build target disappearance; it is not a
direct-compiler fallback or a claim that a pending/failed command passed. This
worker neither claims nor performs target cleanup.
