# Review: CMPCT-044 — Clean compaction sub-agent triggered on API context-overflow errors

**Reviewer:** ACDD Compliance Reviewer
**Date:** 2026-09-17
**Work unit:** CMPCT-044 (story, status: done, completed 2026-09-17)
**Commits reviewed:** `cd7d898b` (feat: clean compaction sub-agent on API context-overflow + GenerateCompaction tool), `f9227f6c` (spec split / clippy / rpc084 window refresh)

## Status: WARN

The work unit is fundamentally sound: all 23 scenarios have matching tests with exact `@step` text, all 9 example-map rules are reflected in code, all 4 scoped test suites pass, clippy is clean, the cascade ordering (PromptCancelled → prompt-too-long → context-overflow → network-retry) is verified in `stream_loop.rs`, the spawner uses a drop guard, and `rust/napi` has zero changes in the last 3 days. The findings below are spec-documentation drift and one unmet design-matrix failure case, none of which break the convergence guarantee.

## 🔴 Critical Issues (Must Fix)
None

## 🟡 Warnings (Should Fix)

1. **Example-map Rule [8] contradicts the implementation and its own architecture note.**
   - Rule [8] (work unit + feature example-mapping comment block, `spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature:25`) says "codelet-tools (handler registry + types)" for the compactor.
   - In reality the `CompactorSubAgentHandler` type + per-session registry live in codelet-cli at `rust/cli/src/compactor_sub_agent.rs:57-62` (type alias) and `rust/cli/src/compactor_sub_agent.rs:65-101` (`COMPACTOR_SUB_AGENT_HANDLERS`, `set_`, `has_`, `clear_all_`).
   - This matches the work unit's own architecture note [0] and the design doc `spec/attachments/CMPCT-044/CMPCT-044-research-2-compaction-subagent-design.md:45-68` (Option A) — an intentional decoupling to avoid changing `run_agent_stream_with_images`'s canonical 9-arg signature (`rust/cli/src/interactive/stream_loop.rs:189-199`, pinned by rpc084_streaming tests).
   - `rust/tools` holds only the CMPCT-045 `GenerateCompactionHandler`; a `Grep` for "compactor" across `rust/tools/src` returns zero matches.
   - **Fix:** correct the rule text (and the feature's example-mapping comment block) so the spec of record names codelet-cli as the registry home.

2. **Scenario "Missing compactor handler still guarantees a reduced session" — third Then step is not honored.**
   - The Gherkin step "And the failure is surfaced with a structured compaction-failed lifecycle event" at `spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature:145` is never implemented.
   - `run_compactor_sub_agent_round` on a missing handler only logs `tracing::warn!` (`rust/cli/src/compactor_sub_agent.rs:181-188`), and on a failing handler only `tracing::warn!` + `pin_fallback_dag` (`rust/cli/src/compactor_sub_agent.rs:202-208`); it then emits `CompactionComplete` (`rust/cli/src/compactor_sub_agent.rs:210-220`).
   - The existing structured event `StreamEvent::CompactionFailed` / `StreamOutput::emit_compaction_failed` (`rust/cli/src/interactive/output.rs:221`, `rust/cli/src/interactive/output.rs:383-385`) is never invoked on this path (confirmed by grep — only used at `stream_loop.rs:348` and `recovery_compaction.rs:478`).
   - The test `rust/cli/tests/cmpct044_compactor_pin_test.rs:535-546` asserts only a `CompactionComplete` and its comment explicitly rationalizes the absence, contradicting the scenario.
   - **Fix:** emit `CompactionFailed` on the missing-handler/fallback path (the design doc `…research-2…md:192-200` §Lifecycle explicitly allows emitting it when the fallback is used), or amend the scenario to describe the actual contract.

3. **Rule [4] partial-DAG recovery is not wired on the CLI/044 timeout path.**
   - Rule [4] specifies "extract_partial_dag_nodes then force_inject_fallback_dag" so the session is ALWAYS reduced.
   - On timeout the spawner returns only a generic message with no partial output — `rust/agent-loop/src/generate_compaction_handler.rs:139-150` builds `build_deep_search_timeout_message(...)` and returns `Err(timeout_msg)`.
   - So `pin_fallback_dag` (`rust/cli/src/compactor_sub_agent.rs:240-249`) always constructs the generic "Auto-recovered: context overflow" node; `extract_partial_dag_nodes_from_text` (`rust/cli/src/compaction_dag.rs:231-236`) is used only by the CMPCT-045 handler-side path at `rust/agent-loop/src/generate_compaction_handler.rs:650`.
   - The design doc `…research-2…md:206` §Failure-matrix states "Sub-agent times out → Fallback DAG (partial nodes if any, else generic node)".
   - The 044 test scenario "Partial dag-node blocks from a failed sub-agent are recovered" (`rust/cli/tests/cmpct044_compactor_pin_test.rs:318-369`) passes only because the fake handler returns the partial text as `Ok`, not via the actual `Err(reason)` timeout branch (which does no recovery).
   - **Fix:** thread the partial output into the timeout `Err` (or have the round call `extract_partial_dag_nodes_from_text` on the reason) so the 044 path matches the design matrix.

## 🟢 Observations (Nice to Have)

1. **DRY: identical 7-line shared-`token_state` reset block duplicated** — `rust/cli/src/interactive/stream_loop.rs:692-698` (macro in-view branch) vs `rust/cli/src/compactor_sub_agent.rs:226-232` (sub-agent branch); could be a shared helper.
2. **DRY: Level-3 fallback assembly exists in three shapes** — `build_fallback_dag` (`rust/agent-loop/src/generate_compaction_handler.rs:649-664`, partial-node-first), `pin_fallback_dag` (`rust/cli/src/compactor_sub_agent.rs:240-249`, generic-only), and the CMPCT-020 watchdog inline version (`rust/agent-loop/src/agent_loop.rs:1469-1498`). A single shared "assemble fallback DAG from output or generic" helper would keep the three convergence paths consistent and would close finding W3 at the same time.
3. **File sizes (300-line guideline):**
   - `rust/agent-loop/src/generate_compaction_handler.rs` (739 lines) is the standout — bundles the CMPCT-044 spawner (lines 1-379) and the CMPCT-045 handler + pin (380-664) plus 4 tests; a `compactor_spawner.rs` split would restore the guideline.
   - `rust/cli/src/compactor_sub_agent.rs` (492 lines) is borderline — production code is ~270 lines, the rest is a large `#[cfg(test)]` module (lines 269-492) that could move to the dedicated integration test file.
   - `rust/cli/src/interactive/error_classifiers.rs` (627 lines) and `rust/cli/src/compaction_dag.rs` (450 lines) are justified: the former is a dense classifier module with a 280-line pre-existing test suite, the latter is mostly const instruction text.
4. **`terminal_overflow_recovery.rs:80` uses `.expect("the compactor round always converges — the session must be reduced")`** on a `Result` in production code. Safe today because `run_compactor_sub_agent_round` cannot actually fail, but it converts any future failure mode into a panic inside a background session turn; a `warn!` + terminal-error fallthrough would be more robust (this is the only non-test-site `expect` in new production code).
5. **`is_context_overflow_error` re-lists the legacy substrings by construction** — the legacy `is_prompt_too_long_error` substrings are copied into `is_context_overflow_wording` (`rust/cli/src/interactive/error_classifiers.rs:68-85`) rather than delegating to `is_prompt_too_long_error` (`rust/cli/src/interactive/error_classifiers.rs:15-31`); any future edit to the legacy list must be mirrored manually. The 044 test `overflow_classifier_matches_every_prompt_too_long_error` (`rust/cli/tests/cmpct044_overflow_classifier_test.rs:22-49`) guards this, but only via its own hardcoded 6-string copy.
6. **FRESH prompt adaptation is string-replace-based** (`build_generate_compaction_prompt`, `rust/cli/src/compaction_dag.rs:301-356`) — replacing hardcoded English substrings ("Your conversation history has been preserved on disk", "Call inject_summary(content) with…"). Fragile if the base instruction constants (`COMPACTION_INSTRUCTION_FRESH` / `_INCREMENTAL`, `rust/cli/src/compaction_dag.rs:34-116`) are reworded; the two inline tests catch the `SessionSearch`-qualification replacements but not the "history" line replacement (no assertion that it was actually swapped).
7. **Spawner redundantly re-stores `compaction_in_progress=true`** (`rust/agent-loop/src/generate_compaction_handler.rs:98`) after the trigger site already set it (`rust/cli/src/compactor_sub_agent.rs:167`) — harmless, but the "who owns the flag" contract is split across two crates.
8. **No @CMPCT-045 cross-tag on the 3 shared-with-045 features** — only `spec/features/generate-compaction-tool.feature:5` carries @CMPCT-045. The FRESH/INCREMENTAL prompt scenarios also have near-duplicate inline tests in `rust/cli/src/compaction_dag.rs:358-450` with @CMPCT-045-flavored step comments, so the prompt contract is tested twice under two work units. Acceptable, but worth noting for future maintenance.

## Coverage Verification
- Feature file(s): `spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature` (8 scenarios, architecture docstring present and accurate, @CMPCT-044 + @done, correct Given/When/Then ordering, no placeholders) — OK; `compactor-sub-agent-escalation-in-stream-loop-error-cascade.feature` (4/4) — OK; `context-overflow-error-classifier.feature` (6/6) — OK; `terminal-overflow-recovery-for-background-sessions.feature` (5/5) — OK. All 4 coverage JSON files exist and are 100%.
- Test file(s): `rust/cli/tests/cmpct044_compactor_pin_test.rs` — OK (8 scenarios, exact @step text, real `Session` + pin primitive, fake handlers); `cmpct044_cascade_ordering_test.rs` — OK (5 tests, source-shape ordering asserts matching the established rpc084 pattern; includes a 5th test for the gate scenario that is not a feature scenario); `cmpct044_overflow_classifier_test.rs` — OK (6 scenarios, tests the real public function); `rust/agent-loop/tests/cmpct044_terminal_overflow_test.rs` — OK (5 scenarios + 2 extra unlinked tests at lines 218-269 and 351-379; the overlapping coverage ranges the supervisor flagged — 347-381 / 383-420 / impl 301-356 — were verified: FRESH test is lines 381-415 (coverage "347-381" is slightly off but points at the right function) and INCREMENTAL is 417-454 (coverage "383-420" likewise approximate but same function); impl range compaction_dag.rs 301-356 IS exactly `build_generate_compaction_prompt`, and the INCREMENTAL-specific sub-range 307-333 correctly covers the FRESH/INCREMENTAL selection + embedded-DAG logic).
- Impl file(s): `rust/cli/src/compactor_sub_agent.rs` — OK (registry + pin primitive + round; coverage ranges 115-131 / 153-249 verified against `pin_dag_to_session` and `run_compactor_sub_agent_round`); `error_classifiers.rs` — OK (classifier at 50-85, all legacy substrings mirrored plus provider variants, budget_tokens early-return preserved); `stream_loop.rs` — OK (classifier call at 1895, overflow arm 1897-1932 before the network arm at 2057, after `classify_compaction_branch` at 1708; escalation at 668 inside `in_loop_compaction_restart!` after the budget check at 646 — matches coverage ranges 1879-1932, 1700-1720, 640-675); `terminal_overflow_recovery.rs` — OK (30-89 verified); `agent_loop.rs` — OK (registration at 750 beside DeepSearch at 740, cleanup at 1536 beside DeepSearch at 1534, terminal arm at 1556-1591); `generate_compaction_handler.rs` — OK (ephemeral `Uuid::new_v4` at 82, drop guard 49-55 + 106, provider inheritance 219, SessionSearch-only survey, 7-tool build 248-256, wall-clock timeout 116-151, depth 62); `compaction_dag.rs` — OK (prompt builder 301-356, FRESH/INCREMENTAL selection, target-id qualification, no inject_summary).
- Scenario coverage: 23/23 scenarios covered (8 + 4 + 6 + 5), 100% per all four .coverage files; plus 4 extra unlinked tests.

## Test Results
All 5 scoped commands pass:
1. `cargo test -p codelet-cli --test cmpct044_compactor_pin_test` — **8 passed, 0 failed**
2. `cargo test -p codelet-cli --test cmpct044_cascade_ordering_test` — **5 passed, 0 failed**
3. `cargo test -p codelet-cli --test cmpct044_overflow_classifier_test` — **6 passed, 0 failed**
4. `cargo test -p codelet-agent-loop --test cmpct044_terminal_overflow_test` — **7 passed, 0 failed**
5. `cargo clippy -p codelet-cli -p codelet-agent-loop -p codelet-tools` — **clean, 0 warnings**

Cross-cutting checks: (F1) yes — `CompactorSessionSearchCleanup` drop guard at `generate_compaction_handler.rs:49-55`/`106` guarantees SessionSearch handler cleanup on panic; (F2) confirmed — `git log --since="3 days ago" -- rust/napi` is empty (napi remains the frozen legacy twin; its own `register_deep_search_handler` call sites at `rust/napi/src/session_bindings.rs:2078,2268` and `rust/napi/src/agent_loop.rs:750` are pre-existing, untouched); (F3) no unbounded loops — sub-agent bounded by 600s wall-clock timeout (`DEEP_SEARCH_WALL_CLOCK_TIMEOUT_SECS`, `recovery_stall.rs:26`) + depth 20 (`COMPACTION_SUB_AGENT_MAX_DEPTH`, `generate_compaction_handler.rs:62`), escalation bounded by shared MAX_COMPACTION_RETRIES; (F4) confirmed — `compaction_dag.rs` 301-356 is exactly the `build_generate_compaction_prompt` function.

## Files Reviewed
- `spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature` (+ `.coverage`)
- `spec/features/compactor-sub-agent-escalation-in-stream-loop-error-cascade.feature` (+ `.coverage`)
- `spec/features/context-overflow-error-classifier.feature` (+ `.coverage`)
- `spec/features/terminal-overflow-recovery-for-background-sessions.feature` (+ `.coverage`)
- `spec/TAGS.md` (header + work-unit tag grep)
- `spec/attachments/CMPCT-044/CMPCT-044-research-2-compaction-subagent-design.md`
- `rust/cli/tests/cmpct044_compactor_pin_test.rs`
- `rust/cli/tests/cmpct044_cascade_ordering_test.rs`
- `rust/cli/tests/cmpct044_overflow_classifier_test.rs`
- `rust/agent-loop/tests/cmpct044_terminal_overflow_test.rs`
- `rust/cli/src/compactor_sub_agent.rs` (full)
- `rust/cli/src/interactive/error_classifiers.rs` (full)
- `rust/cli/src/interactive/stream_loop.rs` (lines 84-117, 189-230, 630-740, 1696-1730, 1820-1950; grep over full file)
- `rust/cli/src/compaction_dag.rs` (full)
- `rust/agent-loop/src/terminal_overflow_recovery.rs` (full)
- `rust/agent-loop/src/generate_compaction_handler.rs` (full)
- `rust/agent-loop/src/agent_loop.rs` (lines 720-779, 1060-1140, 1406-1545; grep for `inner_session` / `compactor`)
- `rust/cli/src/interactive/output.rs` (grep: `CompactionFailed`/`Complete`/`Progress` event definitions)
- `rust/cli/src/interactive/recovery_stall.rs` (grep: timeout constant)
- git history: commits `cd7d898b`, `f9227f6c` (file lists), `rust/napi` change check

---

**Summary**: PASS-with-warnings. Ship-quality implementation with clean cascade ordering, guaranteed convergence (sub-agent DAG or free fallback — the session is never left oversized), and an honest post-pin `CompactionComplete` basis. The 3 warnings are all spec-of-record hygiene (Rule [8] names the wrong crate; the missing-handler scenario's `CompactionFailed` step is unimplemented; partial-dag-node recovery is not wired on the 044 timeout path). Recommend one small follow-up ticket: fix the rule text, emit/adjust the failure event, and share one fallback-assembly helper that adds partial-node recovery to the 044 timeout branch.
