# BUG-242 through BUG-252 completion audit

## Scope and acceptance traceability

The final coordinator preserves the existing staged work and independently
checks the eleven work units, their feature files, mapped tests, implementation
anchors, and executable evidence. No commits, workflow bypasses, alternate
Cargo profiles, direct compiler substitutes, or artifact deletion are used.

An independent read-only audit finds 47 feature files tagged with these work
units, containing 130 scenarios and 463 literal step annotations. Every scenario
has exactly one test mapping and at least one implementation mapping. All
mapped paths exist, all mapped ranges and implementation anchors are in bounds,
and every Gherkin step appears literally inside its mapped test range.

Scenario counts by work unit:

| Work unit | Scenarios |
| --- | ---: |
| BUG-242 | 4 |
| BUG-243 | 10 |
| BUG-244 | 18 |
| BUG-245 | 8 |
| BUG-246 | 17 |
| BUG-247 | 9 |
| BUG-248 | 9 |
| BUG-249 | 18 |
| BUG-250 | 25 |
| BUG-251 | 6 |
| BUG-252 | 6 |

The serialized `fspec audit-coverage` commands pass for all 47 features in
`/tmp/bug242-252-goal-final-coverage-paths.log`. This is requirement-to-code
traceability, not instrumented line or branch coverage.

## Verified repository checks

`fspec validate` passes for all 2,113 feature files in
`/tmp/bug242-252-goal-final-validate.log`.

Global `fspec validate-tags` reports 526 unrelated feature files with existing
tag violations; 1,587 files pass. The full result is recorded in
`/tmp/bug242-252-goal-final-tags.log`. All 47 scoped features pass. This failure is not presented as a global
pass or silently repaired outside the requested scope.

All newly added Rust files are below 300 lines. The staged Rust additions
introduce no `unwrap()`, `panic!`, `todo!`, `dbg!`, unsafe blocks, or production
print macros. Existing unchanged violations in older files are not claimed to
have been removed. Mandatory test step annotations are retained.

An edition-aware formatting audit compares each of the 170 changed Rust files
with its `rustfmt` output and checks formatting differences against the actual
HEAD-to-working-tree changed spans. It finds zero formatting defects in changed
lines. The patched rig crate uses edition 2024; workspace crates use edition 2021.
There are 33 formatting differences in unchanged legacy spans, which are not
rewritten as unrelated cleanup. Newly added modules pass whole-file formatting.
The coordinator fixes only new declaration ordering and the newly added
user-admission error expression. Whitespace checks pass for staged and working
changes.

## Executable validation discipline

Earlier artifact-lock waits, missing-artifact compiler failures, interrupted
commands, and direct compiler runs are not counted as new successful Cargo
executions. Existing feature-first RED and canonical GREEN logs remain available
in each card's evidence. The final canonical repeat uses the existing
`release-slim` profile and two Cargo jobs, preserving exit status.

The final coordinator's repeated canonical tests are recorded in
`/tmp/bug242-252-follow-goal-canonical.log`. All 18 BUG-247/248 tests, checkpoint
preservation, and all nine CLI provider-state/archive tests pass: 28 distinct
tests, with an additional checkpoint subprocess invocation. The subsequent
compaction repeat fails to compile because its LTO output directory disappears.
The entire pipeline exits 101 and is not presented as GREEN. Provider and
multimodal/discovery repeats after that failure do not execute. Existing canonical
BUG-242, BUG-251, and BUG-252 executions remain the recorded acceptance evidence;
their owned implementations and assertions are not changed by this completion
audit. No direct-compiler replacement, alternate profile, or target is used.

## Independent source review and final regression

Read-only reviews of native compaction settlement and the persistence/archive/
media boundaries identify no further concrete criterion violations. They check
authentic versus unknown tool outcomes, unchanged lineage and absolute deadline,
final physical request admission, explicit source failures, pinned original and
journal snapshots, causal request anchors, and serialized deletion ownership.
These reviews supplement tests; they are not presented as runtime passes.

The TUI review identifies an additional failure-notice regression under the
existing failed-resume scenario. Its specification and test are extended before
the production repair. The canonical executable RED is
`/tmp/bug244-245-completion-tui.log`: polling after a failed resume incorrectly
requests transcript pages. The other five targets in that invocation pass
(5 rendered, 3 replay-state, 9 delivery, 2 inspector, 12 restoration tests), while
the slash-resume target has 4 passing tests and this one failing test. The whole
invocation exits 101 and is not called GREEN.

The repair moves suspension release into the successful resume branch, retaining
the incomplete-transcript notice on failures. Its canonical GREEN repeat is
`/tmp/bug244-245-completion-tui-green.log`, with all six targets passing and
`CARGO_EXIT=0` (36 tests). The extra acceptance step does not add another scenario.

The coordinator also corrects the first-active-resume mapping to include its
actual test declaration through fspec, retaining its implementation mappings.
Subsequent header-only factoring refreshes its final range.
`/tmp/bug244-first-active-resume-range-audit.log` records the initial successful
relink; `/tmp/bug242-252-final-traceability-audit.log` verifies the final ranges.

## Mandatory feature boundary and final quality

Five mixed-test-file feature groups are factored without changing any acceptance
scenario or executable assertion. BUG-247/248 and BUG-249 revisit the normal
workflow and rerun mandatory gates. BUG-244's factored features also pass its
validating transition. Every one of the 47 features now references exactly one
test file. Every one of the 130 scenarios maps to a distinct test function, with
all 463 literal steps and implementation anchors in bounds. Derived test-file
and implementation-file statistics match the actual mappings. The independent
read-only audit reports zero issues.

Current canonical library check and strict Clippy pass in
`/tmp/bug249-250-completion-cargo.log` (`CARGO_COMPLETION_EXIT=0`) and
`/tmp/bug246-completion-cargo-quality.log` (`CHECK_EXIT=0`, `CLIPPY_EXIT=0`).
The existing TUI `collapsible_match` lint is corrected only by the identical
match guard, without suppressions or changed behavior. TUI/sessions/RPC-types
check and strict Clippy then pass in
`/tmp/bug244-245-completion-quality-green.log` (`CARGO_EXIT=0`).

The post-factoring delivery/replay-state repeat passes all 12 tests in
`/tmp/bug244-completion-split-green.log` (`CARGO_EXIT=0`). The combined archive,
empty-failure, cold-terminal and transcript replay repeat passes in
`/tmp/bug245-completion-sessions-green.log` (`CARGO_EXIT=0`). The broader TUI
repeat in `/tmp/bug244-245-completion-final-tui-green.log` completes with exit
101, despite its filename: all six BUG-244/245 targets pass (36 tests), as do
all four committed-projection tests. The additional legacy token-state target
has two passing and seven failing tests. These expect local context-fill
recomputation from cumulative display counters, which the authoritative
`coherent-request-context-fill` specification disallows. HEAD already lacks
that recomputation; the equivalent match-guard rewrite does not change it.
The failed run is preserved, not relabelled GREEN, and no legacy behavior
fallback or unrelated test rewrite is introduced to satisfy it.

The TUI owner removes obsolete comments describing that absent recomputation
and runs the current request-context/provenance regressions alongside all six
BUG-244/245 targets and committed-projection tests. The final results of that
run and its refreshed strict quality checks are required before closing the
remaining two cards. Completion tags for the nine already-finished cards are
synchronized through serialized fspec commands; the seven remaining feature
tags await their two cards' actual completion.
