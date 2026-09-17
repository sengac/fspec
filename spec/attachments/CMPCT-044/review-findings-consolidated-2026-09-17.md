# Compaction Cards Review: CMPCT-044 + CMPCT-045

**Date:** 2026-09-17
**Reviewer:** fspec review skill (spec/skills/review-skill.md), 2 parallel ACDD reviewer workers + supervisor verification
**Work Units Reviewed:** 2 (both completed 2026-09-17)

## Summary
- 🔴 Critical: 0 issues across 2 work units
- 🟡 Warnings: 8 issues (3 in CMPCT-044, 5 in CMPCT-045)
- 🟢 Observations: 13 (8 in CMPCT-044, 5 in CMPCT-045)

All scenario coverage is complete (23/23 + 21/21), all scoped test suites pass, clippy is clean on all touched crates, and `rust/napi` has zero changes (Rule [8] of CMPCT-044 holds). No critical issues.

## Work Unit Results

### CMPCT-044: Clean compaction sub-agent triggered on API context-overflow errors — ⚠️ WARN
Full report: `spec/attachments/CMPCT-044/review-findings-cmpct-044.md`

**🟡 Warnings (supervisor-verified):**
1. **Rule [8] names the wrong crate** — the example-map rule says "codelet-tools (handler registry + types)" but `CompactorSubAgentHandler` + `COMPACTOR_SUB_AGENT_HANDLERS` live in codelet-cli (`rust/cli/src/compactor_sub_agent.rs:57-101`). The work unit's own architecture note [0] agrees with the code, so the rule text is the stale artifact. Fix: correct the rule text + feature example-mapping comment block.
2. **Unhoned Gherkin step** — "And the failure is surfaced with a structured compaction-failed lifecycle event" (`clean-compaction-sub-agent...feature:145`) is never implemented: `run_compactor_sub_agent_round` logs `warn!` + pins fallback + emits `CompactionComplete`; `StreamEvent::CompactionFailed` is never emitted on this path. The test comment (`cmpct044_compactor_pin_test.rs:535-541`) rationalizes the absence instead of asserting the step. Fix: emit `CompactionFailed` on the fallback path, or amend the scenario.
3. **Partial-DAG recovery not wired on the 044 timeout path** — on timeout the spawner returns only a generic message (`generate_compaction_handler.rs:139-150`), so `pin_fallback_dag` (`compactor_sub_agent.rs:240-249`) always emits the generic node; `extract_partial_dag_nodes_from_text` is used only by the 045 handler path. The 044 "Partial dag-node blocks" test passes only because the fake handler returns partial text via `Ok`, not via the timeout `Err` branch. Fix: thread partial output into the timeout path or call the extractor on the failure text.

**🟢 Observations (8):** duplicated 7-line token_state reset block (DRY); fallback-DAG assembly in three shapes (DRY); `generate_compaction_handler.rs` 739 lines (should split 044 spawner vs 045 handler); `compactor_sub_agent.rs` 492 lines (test module could move to integration file); `terminal_overflow_recovery.rs:80` production `.expect()`; legacy substrings re-listed rather than delegated in `is_context_overflow_error`; FRESH prompt adaptation is fragile string-replace; no @CMPCT-045 cross-tag on the 3 shared features.

**Test results:** 4/4 scoped suites pass (8 + 5 + 6 + 7 tests), clippy clean on codelet-cli / codelet-agent-loop / codelet-tools.

### CMPCT-045: GenerateCompaction tool — ✅ PASS (with 5 should-fix warnings)
Full report: `spec/attachments/CMPCT-045/review-findings-cmpct-045.md`

**🟡 Warnings (supervisor-verified):**
1. **Custom-provider compactor sub-agent escapes the 7-tool read-only surface** — `generate_compaction_handler.rs:189-216` dispatches to `CustomProvider::create_rig_agent`, which attaches the FULL provider surface (`rust/providers/src/custom/custom_provider.rs:266-303`) including `GenerateCompactionTool` (recursion risk), `InjectSummaryTool`, `DeepSearchTool`, `AgentManagerTool`, etc. The `SUB_AGENT_TOOL_COUNT == 7` compile-time assert guards only the native-provider macro path. (Mitigation: the ephemeral id has no registered handler, so a stray GenerateCompaction call would error rather than recurse — but the contract is still broken.) DeepSearch's spawner has the same shape.
2. **Pin-mutation duplication vs CMPCT-044 (DRY)** — `generate_compaction_handler.rs:587-597` re-implements `pin_dag_to_session` (`rust/cli/src/compactor_sub_agent.rs:115-131`) verbatim, which the 045 design doc explicitly says to reuse.
3. **Fallback-DAG assembly triplicated** — 045 `build_fallback_dag` + agent_loop watchdog Level-3 + 044 `pin_fallback_dag`; a single shared helper in `codelet_cli::compaction_dag` (which already owns `extract_partial_dag_nodes_from_text`) would keep the labels in sync and close 044-W3 at the same time.
4. **File size** — `generate_compaction_handler.rs` 739 lines bundles two work units' concerns (split candidate: `compactor_spawner.rs`); `compaction_dag.rs` 450 lines (mostly instruction constants).
5. **Lifecycle step not asserted in tests** — "compaction progress update before the sub-agent was spawned" (scenario at feature line 219) is implemented (`generate_compaction_handler.rs:510-514`) but the behavioral test only asserts the state changes + CompactionComplete, not the CompactionProgress chunk.

**🟢 Observations (5):** 1531-line monolithic test file (organized, acceptable); tool-surface test duplicated (behavioral + inline); fallback note in tool result only, not in pinned message; `serde_json::to_value(...).unwrap_or_default()` in the pre-tool-hook path; "persisted manifest NOT truncated" verified by comment only.

**Test results:** 20/20 behavioral + 969 codelet-tools lib tests pass, clippy clean on codelet-agent-loop / codelet-tools / codelet-cli / codelet-providers.

## Cross-Card Findings (shared surface)
- **One shared file, two work units:** `rust/agent-loop/src/generate_compaction_handler.rs` contains both the 044 spawner and the 045 tool handler; both reviewers independently flag a split.
- **Three convergence paths, three fallback assemblies:** the CMPCT-020 Level-3 shape now exists in 044 (CLI), 045 (agent-loop), and the agent-loop watchdog. A shared `build_recovered_or_generic_dag()` in codelet-cli would unify them.
- **Spec-of-record hygiene is the dominant theme:** 3 of 8 warnings are spec text that drifted from the implementation (rule [8] crate name, CompactionFailed step, timeout partial-recovery), not logic bugs.

## Suggested Follow-up Ticket (if/when fixes are scheduled)
1. Correct CMPCT-044 Rule [8] text (codelet-cli owns the compactor registry).
2. Decide the `CompactionFailed` contract: emit it on fallback paths in `run_compactor_sub_agent_round` (design doc §Lifecycle permits this) OR amend the 044 feature scenario; update the test to assert it.
3. Wire partial-node recovery into the 044 timeout path (thread partial output into the spawner timeout Err, or run `extract_partial_dag_nodes_from_text` on the failure text in `pin_fallback_dag`).
4. Add a sub-agent tool-surface guard to the custom-provider path (strip/limit tools for the ephemeral compactor session, or assert post-build), and consider the same for DeepSearch.
5. Extract a shared pin primitive + fallback-DAG builder; have the 045 handler call `pin_dag_to_session`.
6. Split `generate_compaction_handler.rs` into `compactor_spawner.rs` + `generate_compaction.rs`.
7. Replace `terminal_overflow_recovery.rs:80` `.expect()` with warn + fallthrough.
