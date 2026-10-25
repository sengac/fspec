# Review: BUG-258 — outer terminal journaling

Reviewer: the primary codex/gpt-6.1-sol session, direct review only.
Scope: BUG-258; no child sessions, unrelated card edits, commit or push.

## Result

BUG-258 acceptance and code-quality review passes. Broader repository validation
has warnings: five failures reproduced on the executed isolated pre-BUG-258
baseline and final current code, plus existing global tag/format failures.
These remain disclosed in `acdd-validation.md`, not claimed as passing.

## Findings resolved

- The 1,536-line async loop is a 233-line orchestrator plus bounded phase modules.
  All new/edited Rust files remain below 300 lines; no source-fragment includes.
- Receiver priority/closure, epochs, interruption/notify, inner-lock scope,
  fresh-user counter reset, registration/drop ordering and Idle-before-Done
  remain intact. Five unchanged MCP structural contracts also pass.
- Outer termination uses the same request-owned output and journal. Owned
  overflow recovery no longer allocates a second output/journal identity.
- An existing CompactionFailed terminal remains immutable while one actionable
  outer diagnostic may publish; ProviderError duplicates remain suppressed.
- Seven scenarios map 1:1 to seven real-path tests with exact step comments.
  Safe typed outcomes survive repeated fresh-process replay without transcript
  metadata or terminal prose entering model originals/history.
- The extraction baseline preserved four reds/two greens before repair. The
  related explicit-retry compatibility failure received its own native
  specification, red test, bounded fix and green rerun.

## Verification

- Seven acceptance cases pass, including seven explicitly invoked cold helpers.
- Eighty distinct observed scoped test passes; five baseline-proved preexisting
  failures remain reported, so the broader repository suite is not all green.
- Scoped library check, library Clippy, BUG-258 test Clippy and gated stub library
  check pass. Test warnings are unused members of existing shared fixtures.
- Native feature/tag validation, coverage audit (15 file mappings), exact step
  and line-bound checks, targeted rustfmt and `git diff --check` pass.
- No new global publication hook, terminal rewrite, synthetic identity, silent
  journal failure, external provider request or disabled lint is introduced.

## Completed baseline classification

The isolated copy retained earlier-card changes and restored only the four
BUG-258 parent source files from HEAD `2699693b74124cf12b323997fd68dd491adf7e8f`;
the original working tree was untouched. The manifest and execution metadata are
attached as `baseline-manifest.json` and `bug258-baseline-run.json`.
`bug258-baseline-tests.log` records the full budget suite (4 passes/4 failures)
and dependency suite (2 passes/1 failure), expected exit 101.

`baseline-classification-current.log` records the full final current-code budget
suite (4 passes/4 identical token-assertion failures), explicit-retry safety
(2 passes) and dependency suite (2 passes/1 identical unknown-invoking-context
failure), expected exit 101. All five failure names and substantive diagnostics
match; per-run context UUIDs differ. The original repeated-builder-failure test
passes on baseline and final current code, confirming the separate introduced
live-diagnostic regression was repaired rather than mislabeled preexisting.
See `baseline-comparison.md` for the five-name mapping and exact diagnostics.

This evidence-only finalization is performed by the one human-authorized
replacement codex/gpt-6.1-sol session. It preserves the original direct review
and old session/history, with no compaction retry, new implementation or
validation rerun. Existing seven-case acceptance, coverage and quality evidence
is retained; only BUG-258 evidence and native lifecycle bookkeeping change.

## Production inventory reviewed

- `agent_loop.rs`; phase modules `state`, `input`, `lifecycle`, `prompt`, `routing`,
  `thinking`, `handlers`, `services`, `bridge`, `providers`, `provider_openai`,
  `provider_custom`, `reconciliation`, `finalization`, `idle_guard`.
- `background_recovery.rs`, `stream_journal.rs`, `terminal_overflow_recovery.rs`;
  existing output/admission/replay/compaction call sites for ownership tracing.
- `tests/bug258_outer_terminal.rs`, `tests/support/bug258.rs` and reused harnesses;
  seven-scenario feature, native coverage sidecar, plan and evidence attachments.

The native completion record must retain these warnings and the uncommitted
state; it must not imply a clean whole-workspace validation result.

Native finalization accepted **validating → done** after attaching the completed
comparison evidence. The completion reason retains the five baseline-proved
failures, global-spec warnings and uncommitted state. The feature already carries
`@done`; no acceptance, source, test or unrelated-card change was required.
