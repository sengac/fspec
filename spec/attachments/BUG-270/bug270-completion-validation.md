# BUG-270 completion validation

## Repair and traceability

Healthy transcript submissions commit directly to the journal under the existing
session recovery lease only when both memory ownership lists and the on-disk
recovery lane are empty. Journal failures preserve the original envelope under
that lease before falling back to the existing bounded recovery path. Journal
durability still gates stream publication and tool dispatch.

The feature-first repair and its genuine failing permission-preservation test
are documented in `bug270-streaming-speed-research.md` and the retained RED logs.
Finalization did not require another production change. A duplicate empty test
mapping was removed from the diagnostic feature's coverage file; each of the
twelve scenarios now has one test mapping and an implementation mapping.

## Final test runs

All commands run from `rust/`, scoped to `codelet-agent-loop`, using the
`ci-test` profile and at most two build jobs.

- `bug270_admission_safety`: 6 passed.
- `bug270_failure_ownership`: 3 passed, including the macOS ACL fixture for
  simultaneous journal and recovery-staging failure.
- `bug270_streaming_speed`: 3 passed.
- Sixteen surrounding BUG-246, BUG-249 and BUG-250 test targets: 35 parent
  tests passed, with two intentionally ignored subprocess entry points.
  The retirement suite also successfully invoked eight child-process runs.

The surrounding targets cover corrupt recovery, bounded capacity, exact
ownership, failed journal promotion, executor admission, interrupted tools,
process termination, deletion retirement and causal request ordering. Full
commands and results are retained in `bug270-final-focused.log` and
`bug270-final-regressions.log`. No workspace-wide test pass is claimed.

The latest diagnostic run measured 76.1 and 28.5 durable deltas/s for its two
payload labels. This variability reinforces that the earlier 98–102 deltas/s
measurement is not a universal speedup promise. Both streams recovered all 256
identities and original contents, with zero warm history reads. Synchronous
journal I/O and executor blocking remain explicitly outside this focused repair.

## Rust quality checks

- `cargo fmt -p codelet-agent-loop -- --check`: passes.
- `cargo check --profile ci-test -p codelet-agent-loop -j 2`: passes.
- `cargo clippy --profile ci-test -p codelet-agent-loop --lib
  --test bug270_admission_safety --test bug270_failure_ownership
  --test bug270_streaming_speed -j 2`: passes without diagnostics.
- `git diff --check`: passes.

## Specification validation

- All three card features were formatted and individually validated.
- All twelve scenario mappings pass `fspec audit-coverage`.
- Repository-wide `fspec validate`: all 2,149 features pass.
- Repository-wide `fspec validate-tags`: 522 existing feature files fail;
  none of the three BUG-270 features is reported as an offender.

## Existing unrelated regression failures

The earlier broader test run found two failures in
`cmpct052_durable_submission`: a missing OpenAI usage response and an assertion
that compaction inference should remain blocked. The recorded control run
against the unmodified production source has the same two failures and
signatures (`bug270-cmpct-head.txt`). These tests use local scripted providers;
they do not require a live OpenAI endpoint. Their fixture/protocol behavior is
outside this card's persistence fast-path scope and was not changed.

## Review

A separate read-only review found no concrete production blocker in durability,
bounded ownership or deletion exclusion. This is not a claim of exhaustive
concurrency or cross-platform fault coverage. Memory-only retention during total
storage failure remains process-local and shutdown-unsafe until repaired.
