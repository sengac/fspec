# BUG-236: Token-accounting lifecycle and compaction estimates

Research date: 2026-10-22. HITL explicitly selected an accounting investigation,
not just log-level cleanup. No provider reporting defect has been proven.
Source references describe the triage snapshot.

## Context and impact

Thirty-two warnings say that tracker input is zero because the provider reported no
usage, then use a content estimate. That causal wording is too strong: registration
calculates the compaction basis before ordinary inference, when zero can be normal.
Longer-history zeros still need investigation because stale/restored/reset accounting
may affect compaction measurement or admission even when startup behavior is harmless.

## Incident evidence

- Logs: `~/.fspec/logs/fspec-combined.log.2026-10-21` and
  `~/.fspec/logs/fspec-combined.log.2026-10-22`, pre-review records before
  `2026-10-22T03:32:00Z`.
- Startup example: Oct 21 `:52` through `:68` creates Codex session
  `e284c6cd-1886-492e-b7ec-0430e3eeb893`, starts its first run, then reports an
  estimated 2,785 tokens over two messages at 22:30:18.955764 UTC.
- Longer histories: Oct 22 `:271` estimates 66,751 tokens over 51 messages;
  Oct 22 `:410` estimates 21,641 tokens over 62 messages.
- Message: `tracker input_tokens is 0 (provider reported no usage)`.
- These are fallback computations, not evidence of 32 attempted or failed compactions.

## Code and existing contracts

- `rust/agent-loop/src/generate_compaction_handler.rs:116` captures the turn's
  compaction state when registering handlers.
- `rust/agent-loop/src/turn_compaction.rs:46` computes the original-token basis.
- `rust/cli/src/interactive_helpers.rs:240` prefers nonzero tracker input, otherwise
  estimates message content and warns when that estimate is positive.
- `rust/core/src/token_usage.rs:76` updates tracker input from provider usage.
- `rust/core/src/compaction_hook.rs:266` maintains related context accounting.
- `spec/features/generate-compaction-tool.feature:1` describes the existing
  compaction measurement behavior.

Related completed work: CMPCT-046 and CMPCT-038. Preserve honest before/after
measurement and existing cache-token accounting; do not count cache usage twice.

## Investigation questions

1. Which warnings occur before a first provider response versus after successful usage?
2. Are longer histories restored sessions, provider streams without usage, reset
   trackers, or another initialization/accounting path?
3. Are provider-reported totals current for the captured history, and how is freshness
   represented across restore, model switching, retries, and compaction?
4. Does any incorrect basis influence request admission or only displayed reduction ratios?

## Expected outcomes

- Distinguish uninitialized accounting, provider-reported usage, unavailable usage,
  restored/stale accounting, and content estimation wherever those states matter.
- Correct any reproduced accounting defect at its owner; do not assume the fallback
  itself is wrong merely because the tracker is zero.
- Preserve content estimation when usage is unavailable and messages contain countable
  content; empty history may legitimately have a zero basis.
- Never present an estimate as provider-reported usage or assert a missing-provider
  cause without sufficient evidence.
- Warn on meaningful accounting anomalies without producing routine startup warnings
  on every handler registration; retain useful provenance in diagnostics.
- Preserve consistent pre/post compaction measurement and cache-token semantics.

## Regression evidence to produce

Cover fresh session registration before inference, real reported usage, a provider
without usage, restored nonempty histories, reset/model-switch paths, empty history,
and post-compaction recalculation. Assert both numeric results and truthful diagnostics.

Review `rust/agent-loop/tests/cmpct041_pre_compaction_basis_test.rs:1`,
`rust/cli/tests/cmpct041_seed_cache_double_count_test.rs:1`, and
`rust/core/tests/rig_011_api_token_usage.rs:1` before extending coverage.

## Scope limits and completion

Do not invent provider usage, silence all anomalies, change thresholds to mask bad
accounting, or replace estimates with arbitrary nonzero constants. Completion requires
an explanation of the startup and longer-history cases, scenario-linked tests for any
fix, and scoped checks showing that measurement and admission remain truthful.

## Implementation investigation (2026-10-22)

- Startup is reproduced without a provider request: repeated basis capture on a
  new tracker and two messages estimates four tokens and formerly emitted WARN
  with the unsupported provider-omission claim. This establishes the startup
  diagnostic defect, not a provider reporting defect.
- Longer histories can legitimately coexist with zero accounting: durable restore
  accepts a saved zero current-context total, model/provider state resets invalidate
  input evidence, and a completed response may lack usable input accounting.
  Behavioral fixtures reproduce these distinct paths. Historical records do not
  include enough provenance to determine which path caused the logged 51-message
  and 62-message examples. No specific historical cause is claimed.
- Added input provenance independently of numeric display state: uninitialized,
  unknown legacy metadata, reported, unavailable in an observed response, restored,
  reset, and content estimated. Unknown is the serde default for old records;
  newly constructed trackers are uninitialized. Durable manifests do not preserve
  this provenance, so restoration is explicitly labeled restored rather than
  claiming a fresh provider observation.
- Stream Usage and FinalResponse boundaries record observations in both ordinary
  streaming and Gemini continuations. A zero-input delta does not erase a positive
  input observation from the same response. Finalization clears the per-response
  observation; request restarts/errors cannot carry it into another attempt.
  Display-only settlement cannot resurrect a stale total after an unavailable
  response. Pre-compaction display flushes preserve provenance and existing billing.
- Provider/model reset invalidates old input while retaining session spend; full
  history clear/provider switch preserve their existing full-reset spend behavior.
  `reset_after_compaction` retains the established numeric-input contract while
  labeling it reset. Content recalculation labels estimates and clears stale caches.
- Cache-inclusive totals, content token estimation, reduction-ratio math, selected
  request admission, and compaction thresholds are not replaced or redefined.
  A 30,000 fresh plus 150,000 cache-read response remains 180,000 input tokens;
  billing still counts raw input only.

## ACDD and regression evidence

- Feature written/formatted/validated before tests; work unit progressed through
  specifying, testing, implementing, and validating without temporal overrides.
- Observed runtime RED: `/tmp/bug236-red.log` reproduces the misleading startup
  WARN. `/tmp/bug236-stream-observation-red.log` reproduces a zero delta erasing
  a 180,000-token observation before the per-response collector was corrected.
  Initial new-API compile failures are recorded separately in
  `/tmp/bug236-provenance-red.log`; they are not claimed as runtime evidence.
- Ten scenarios map 1:1 to ten runtime tests in
  `rust/cli/tests/bug236_input_provenance.rs` (294 lines). Tests exercise numeric
  results, captured tracing events, real durable restore with shared temp helpers,
  serialization compatibility, reset, display flush, and post-compaction estimates.
  None asserts source strings. `fspec show-coverage token-accounting-lifecycle`
  reports 100% (10/10); `fspec audit-coverage` finds all 20 file references.
- Core scoped regressions: 44 tests pass across token tracker updates, cache
  extraction, API usage, reasoning propagation, and reasoning accumulation
  (`/tmp/bug236-core-green-final.log`).
- Existing CMPCT-041 and TOKEN-001 session fixtures required replacing live
  credential selection with the existing hermetic `ProviderManager::for_testing`
  constructor. Their runtime regressions now run without keys or network access.
- The supplementary CMPCT-041 source guard already fails on HEAD: it forbids all
  `StreamingTokenDisplay::new` calls, but HEAD already contains one for authoritative
  FinalResponse reconstruction, not seeding. Runtime regressions run with only
  `--skip stream_loop_seed_sites_route_through_audited_constructor`. No source-guard
  rewrite or unrelated production change is included for that baseline failure.
- Intermediate cross-package checks encountered temporarily incomplete edits in
  other workers' compaction operation/builder/tracing files. Those failures are
  not attributed to BUG-236 and are not reported as successful validation.

## Final validation

- `/tmp/bug236-cli-validated.log`: 22 passing tests across the lifecycle (10),
  cache-integrity runtime regressions (4), Gemini continuation (5), and cumulative
  output billing (3). Only the baseline source guard described above is skipped.
- `/tmp/bug236-core-green-final.log`: 44 passing core regression tests.
- `/tmp/bug236-basis-behavioral.log`: the existing overflow-recovery seed-window
  snapshot runtime test passes with `--features test-support` and a single test
  filter. In the full target, two obsolete source guards expect a
  `StreamEvent::CompactionStarted` arm that HEAD's agent-loop output no longer has.
  The full concurrent run also exposes the old tests' process-global data-dir race;
  the isolated runtime scenario passes. No source guards are counted as new coverage.
- Total scoped passing tests: 67, including legacy supplementary structural guards.
  All ten new acceptance tests are runtime behavioral tests. The full legacy basis-unification target
  is not reported green; its limitations are retained in `/tmp/bug236-basis-final.log`.
- `cargo check --profile ci-test` and `cargo clippy --profile ci-test`, scoped to
  `codelet-core`, `codelet-cli`, `codelet-sessions`, `codelet-agent-loop`, and
  `codelet-napi` with `--lib`, both pass. Logs:
  `/tmp/bug236-check-final.log` and `/tmp/bug236-clippy-final.log`. Both retain seven
  existing NAPI warnings (unused legacy persistence helpers and ignored clear result).
  The new lifecycle test target also passes clippy (`/tmp/bug236-test-clippy.log`).
- Targeted rustfmt checks and `git diff --check` pass. All new Rust and feature
  files are under 300 lines; the scenario test file is 294 lines and its coverage
  file is 274 lines.
- Feature syntax, registered tags, coverage audit and 10/10 scenario coverage pass.
  Repository-wide `fspec check` is not green: existing unrelated features have
  unregistered tags and missing required feature-group tags
  (`/tmp/bug236-fspec-check.log`). Those unrelated specifications are not changed.

## Reopened request-settlement completion (2026-10-22)

- Six additional acceptance scenarios cover display settlement, stale post-compaction
  displays, aborted attempts, internal tool followups, retries and Gemini continuations.
  `/tmp/bug236-completion-red.log` reproduces five runtime failures before the fixes.
- Display settlement accepts legacy numeric input only while provenance is unknown
  or uninitialized. Reported, restored, unavailable, estimated and reset input/cache
  state stays authoritative; output and reasoning still settle normally.
- Each physical rig request emits its existing attempt-start boundary, including
  internal tool followups. Ordinary abort handling and Gemini start/abort handling
  discard the provisional observation collector. Persistent request settlement
  remains the billing authority; preserving streamed context does not invent spend.
- Review found a separate absent-usage case: a present physical request context
  with no usage fell back to aggregate usage from earlier requests. The expanded
  followup test reproduces this runtime failure in `/tmp/bug236-absent-usage-red.log`.
  Ordinary and Gemini finalization now distinguish absent physical usage from absent
  context, retaining aggregate fallback only for the legacy no-context case.
  Followup coverage includes both explicit zero and absent usage on OpenAI and Gemini.
- `/tmp/bug236-completion-final-tests.log`: 28 tests pass, including all 16 acceptance
  scenarios and 12 adjacent cache, continuation and output-billing regressions.
  The previously documented CMPCT-041 source guard remains the only filtered test.
  `/tmp/bug236-completion-core.log`: 44 core regressions pass. Total: 72 scoped passes.
- Five affected libraries pass scoped `cargo check` and `cargo clippy` under
  `ci-test`; both acceptance targets also pass clippy. Logs are
  `/tmp/bug236-completion-check.log`, `/tmp/bug236-completion-clippy.log` and
  `/tmp/bug236-completion-test-clippy.log`. Seven existing NAPI warnings remain.
- Both feature files validate and report full scenario-to-test-to-implementation
  coverage (10/10 and 6/6); both coverage audits pass. Targeted rustfmt checks and
  `git diff --check` pass. Repository tag validation still fails on unrelated
  specifications (`/tmp/bug236-completion-tags.log`); neither accounting feature
  appears among the violations.
- Additional exploration of `bug223_gemini_helper_settlement` yields six passes and
  three failures (`/tmp/bug236-completion-cli.log`). Read-only comparison with HEAD
  identifies existing fixture/admission gaps: scripted tool arguments omit required
  `work_unit_id`, and the fixture never registers an invoking admission context.
  Tools therefore return admission errors without executing, explaining zero call
  counts, no accepted Done and no large-result compaction handoff. These unrelated
  fixtures are unchanged and this target is not reported green.
