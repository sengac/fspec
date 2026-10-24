# BUG-223 validation — 2026-10-21

## Outcome

Gemini helper normal completion returns text, display state, usage, tool activity
and the actual stop reason to the main stream's existing settlement owner.
Accepted Done uses shared immediate teardown before another stream poll.
Interruption, errors and compaction remain separate from clean completion.
No commit or branch was created for this work; unrelated worktree changes remain.

## ACDD evidence

1. The completion-settlement feature and nine scenario tests preceded production
   changes. Example Mapping rules and examples are recorded on BUG-223.
2. The corrected offline Gemini fixture demonstrated RED before implementing:
   seven failures, two passing controls (`/tmp/bug223-red.log`). Failures exposed
   missing nudges/refunds/warnings/escalation/accepted teardown and lost stop reasons.
3. Implementation followed the RED run and the implementing status transition.
4. Final real streamed tests pass all nine scenarios, including ordinary/nested
   helpers, goal/non-goal acceptance, measured output/reasoning and physical input/cache.
5. Accepted completion was moved, without changing its criteria, into a separate
   feature to satisfy the one-feature/one-test-file gate while keeping files below
   300 lines. It is compiled as a module of the same Cargo integration target.
6. Coverage is 8/8 for `gemini-helper-completion-settlement` and 1/1 for
   `gemini-helper-accepted-completion`. Both coverage audits and step validation pass.

The compaction fixture now uses an approximately 45 KB tool result: above its
context threshold but below Rig's 64 KiB output truncation cap. It verifies the
specific recovery handoff error, not just an arbitrary failure. Usage assertions
distinguish display output from billed output including reasoning. Accepted-tool
request billing remains unchanged, matching ordinary immediate-exit behavior.

## Checks performed

From `rust/`:

```sh
cargo check -p codelet-cli
cargo clippy -p codelet-cli --lib --test bug223_gemini_helper_settlement -- -D warnings
cargo test -p codelet-cli --lib --test auto_continue_engine_test --test bug223_gemini_helper_settlement --test cmpct041_seed_cache_double_count_test --test cmpct042_gemini_continuation_token_state_test --test compaction_cancel_preservation_test --test compaction_real_integration_coverage_test --test compaction_recovery_unification_test --test compaction_retry_prompt_policy_test --test compaction_trigger_reliability_test --test cont005_done_immediate_termination --test cont006_goal_immediate_termination --test goal_enforcement_test --test token_001_cumulative_billed_output_test --test token_002_per_call_site_delta_test --no-fail-fast
```

- Compilation and scoped Clippy pass.
- Regression run: 245 passing tests, five failures across four targets, listed below.
- BUG-223: nine passing tests. TOKEN-001: three passing; TOKEN-002: four passing.
- Scoped rustfmt and git whitespace checks pass for this change's files.
- Both BUG-223 features pass formatting, Gherkin validation and tag validation.
- Repository Gherkin validation passes all 2,035 features.
- Repository-wide tag validation/check fails on 529 unrelated feature files.
- Package-wide formatting reports an unrelated `compaction_durable_source.rs` diff.

Logs: `/tmp/bug223-final-regressions.log`, `/tmp/bug223-final-check.log`,
`/tmp/bug223-final-clippy.log`, `/tmp/bug223-scoped-format.log`,
`/tmp/bug223-format.log`, `/tmp/bug223-validate-all.log`, `/tmp/bug223-tags.log`,
`/tmp/bug223-fspec-check.log`.

## Broader failures outside BUG-223

Source comparison against HEAD `f0194bd9` establishes the following existing
conflicts. This is source-level baseline verification, not a separate clean-HEAD
test run. No unrelated production fixes or weakened assertions were applied.

- Two `compaction_threshold` unit tests still expect a 68,000-token fallback for
  zero output limits, while `calculate_usable_context` returns zero. The source
  and tests are unchanged by BUG-223.
- `auto_continue_engine_test::done_is_registered_only_while_auto_continue_is_armed`
  reads Codex `mod.rs`; registration lives in `codex/agent.rs`. Both are outside
  this fix and the failing test is unchanged.
- `cmpct041_seed_cache_double_count_test::stream_loop_seed_sites_route_through_audited_constructor`
  rejects the main loop's physical-usage `StreamingTokenDisplay::new` constructor.
  That constructor is already present in HEAD. The unchanged failing assertion
  runs before the Gemini guards adapted for this fix's module split.
- `cont006_goal_immediate_termination::stream_loop_wiring_pins_goal_mode_early_exit_and_single_teardown_invariants`
  searches `session/mod.rs` for `clear_goal`, which lives in `session/reminders.rs`.
  The failing test and those session files are unchanged by BUG-223.

Read-only correctness review found no blocking BUG-223 issue. Nonblocking future
coverage opportunities: EOF without FinalResponse, rejected helper Done, and
successful compaction through eventual completion (the current scenario verifies
handoff only).
