# Epic Review: completion-contract follow-up cards (CONT-005..CONT-009)

**Date:** 2026-07-10
**Reviewer:** Claude Code (fspec review skill, 5 parallel review agents)
**Work Units Reviewed:** 5 (CONT-009, CONT-005, CONT-006, CONT-007, CONT-008 — review order = dependency order)
**Full reports:** reviewer sessions d6eaa190 (009), 99ccb30a (005), 53985f4e (006), cb16398c (007), 3b5433aa (008)

## Summary
- 🔴 Critical: 1 issue in 1 work unit (CONT-009 feature-file corruption)
- 🟡 Warnings: 13 across 5 work units (dominant theme: coverage impl line-range drift caused by later cards editing the same files without re-linking earlier cards' coverage)
- 🟢 Observations: 14

## Work Unit Results

### CONT-009: NAPI arming gap — **FAIL**
- 🔴 1. `spec/features/completion-contract-dispatch-arming.feature` corrupted with 4× duplication of its 7 scenarios (28 scenario blocks; first two copies mis-indented; 21 duplicates linked to the work unit but uncovered). Must deduplicate to the canonical 7 (last block had correct indentation), regenerate coverage, repair work-unit scenario links.
- 🟡 2. Stale impl coverage ranges: scenarios 1–6 map background_session.rs:1223–1273 but the helper now lives at :1304–1339 (shifted by CONT-008). Twin-parity mappings still accurate.
- 🟡 3. Feature architecture doc string is the pre-implementation research note (port plan), not the as-built design (shared helper in codelet-sessions).
- 🟡 4. Work-unit assumption #1 ("research attachment is EMPTY 0 bytes") is stale — file restored to 4.6KB.
- 🟢 A third inline arming site exists in cli agent_runner.rs:49–50/:110–111 (repl surface) outside the shared helper — in-scope-compliant, candidate follow-up.

### CONT-005: done() immediate termination — **WARN**
- 🟡 1. Stale coverage impl line ranges (drift from CONT-006/007/008): FinalResponse-fallback scenario maps stream_loop.rs:1557–1570 (now argument lists; real call site :1586–1588); early-exit scenarios map :1012–1057 (includes unrelated lines, excludes teardown call :1058–1063, emit :1065–1067, break :1068); assistant-text scenario cuts off mid-flush; all done_early_exit.rs mappings end at :102 but apply_finish_with_summary extends to :120.
- 🟡 2. Work-unit example map diverges from the feature file: feature carries [SUPERSEDED/UPDATED BY CONT-006] markers but the work unit's rule [3], example [1], assumption 3, arch note [3] still state the pre-CONT-006 goal-deferral with no markers — contradicts shipped code.
- 🟡 3. Feature doc string "Implementation shape" paragraph still describes decide_tool_result_early_exit(goal_active, …) — a signature that no longer exists (dropped by CONT-006); no supersession marker on the paragraph itself.
- 🟢 Semantic checks all passed: pairing invariant, no-nudge-after-exit, interrupt precedence, single teardown helper, stop_reason wiring.

### CONT-006: /goal immediate termination — **WARN**
- 🟡 1. Stale impl coverage ranges in done_early_exit.rs (mapped 64–102 / 84–102; apply_finish_with_summary now 92–120 — mapped ranges omit the entire goal-branch teardown body :105–116).
- 🟡 2. Stale impl coverage range for stream_loop.rs (mapped 1012–1057; real block 1024–1069; scenario 6 asserts the FinalResponse fallback :1576–1589 which is absent from its mapping).
- 🟡 3. Rule [6] (paired tool_use/tool_result + no reminder after early exit): reminder half tested; pairing half asserted nowhere in cont006 tests (inherited implicitly from CONT-005).
- 🟡 4. Rule [3] "verify timeout" rejection: timeout ⇒ no-early-exit combination unpinned (only Tier-1 and Tier-2 exit-1 exercised; test hook set_verify_timeout_for_tests exists at done.rs:138).
- 🟢 CONT-008's later addition to the shared helper is coherent with the single-teardown invariant; minor DRY nit (session.goal.is_some() evaluated twice).

### CONT-007: Live status-bar counter — **PASS**
- 🟡 1. Coverage ranges for the pinned-sources scenario are extremely coarse (stream_loop.rs 296–1640; output.rs 125–578) — dilutes traceability; tighten to the ~5 actual sites.
- 🟡 2. Gherkin style: two action steps expressed as And-after-Then (feature lines 61, 67) — should be explicit When/Then pairs.
- 🟡 3. Rule 7's /goal half (dispatch drops live cache) tested only via un-stepped extra assertions (test :438–458) with no Gherkin step.
- 🟢 Twin mapping bodies duplicated verbatim (pinned by shape test — convention-compliant); footer double-width fix verified safe for other consumers; CONT-008 drift in assumption 1 documented, not incoherent.

### CONT-008: Goal state back-sync — **PASS**
- 🟡 1. Pre-existing red test bug146_napi_attribute_scoping (asserts a time-bound uncommitted-git-diff shape; known since BUG-147 era) deepened by rpc-types edits; no follow-up card referenced. Requires a tracked follow-up work unit.
- 🟡 2. Escalation scenario's When step is a comment-only no-op; engine rule enforced only by source-shape scan. A behavioral test driving raise_goal_escalation with a stub pause handler would pin it stronger.
- 🟢 Concurrency review of the generation-stamp design: sound (mutex-protected check-then-clear, snapshot-before-read, self-healing idempotent re-sync); theoretical delayed-write-back race unreachable today — worth a code comment; resurrection repro verified genuinely end-to-end; no_napi_dependency contract intact.

## Fix Results (Phase 4/5, completed 2026-07-10)

### CONT-009 — FAIL → ✅ fixed, done
- 🔴 Feature file 4× duplication → ✅ rewritten to the 7 canonical scenarios; work-unit linkage self-repaired (28→7); coverage 7/7 100%, audit clean.
- 🟡 Stale impl ranges → ✅ re-linked to sync_completion_contract_for_user_turn (:1304-1339; later :1317-1352 after CONT-008's race comment shifted lines — re-linked again).
- 🟡 Doc string → ✅ rewritten as-built (shared helper, twin call sites, parity shape test).
- 🟡 Stale assumption → ✅ amended via successor assumption (time-scoped note).

### CONT-005 — WARN → ✅ fixed, done
- 🟡 Stale coverage ranges → ✅ all six scenarios re-linked to verified current extents (stream_loop.rs 1024-1069 / 1576-1589 / 1041-1054; done_early_exit.rs 40/48/71-120).
- 🟡 Example-map divergence → ✅ stale rule[3]/example[1]/arch-note[3] soft-deleted; successor rule[8]/example[7]/arch-note[4]/assumption 5 carry [SUPERSEDED BY CONT-006] markers.
- 🟡 Doc string contradiction → ✅ implementation-shape paragraph rewritten to the shipped 1-arg signature with inline supersession note.

### CONT-006 — WARN → ✅ fixed, done
- 🟡 Stale coverage ranges → ✅ re-linked (done_early_exit.rs 92-120 incl. goal-branch body; stream_loop.rs 1024-1069 + FinalResponse fallback 1576-1589 for the wiring scenario).
- 🟡 Rule 6 pairing → ✅ new Gherkin step + real paired tool_use/tool_result assertion through clear_goal's reminder filter.
- 🟡 Rule 3 timeout → ✅ new scenario/test: sleep-5 verify + 200ms test timeout hook → rejection recorded, no acceptance, no early exit. Feature now 7 scenarios, 7/7 covered.

### CONT-007 — PASS → ✅ warnings fixed, done
- 🟡 Coarse coverage → ✅ tightened to the actual sites (stream_loop.rs 296-306/1537-1543/1590-1602/1617-1639; output.rs 125-170/224-226/552-566) as multi-range mappings.
- 🟡 And-after-Then action steps → ✅ restructured into explicit When/Then pairs; @step comments updated to exact text.
- 🟡 Un-stepped /goal cache-drop → ✅ Gherkin steps added; existing assertions now sit under matching @step comments.

### CONT-008 — PASS → ✅ warnings + observation fixed, done
- 🟡 bug146 follow-up → ✅ BUG-150 created (backlog), relatesTo CONT-008.
- 🟡 Escalation no-op When → ✅ behavioral pin: stub PauseHandler registered via tool_pause, real raise_goal_escalation driven; asserts handler invoked, no goal-cleared/ContinueState emission, goal survives (session + registry). Source-shape pin retained.
- 🟢 Race comment → ✅ timing contract documented on clear_goal_state_if_unchanged_since_sync; coverage ripple re-linked in both affected features.

### Additional follow-up cards raised during this cycle
- **BUG-150** — bug146_napi_attribute_scoping pins a time-bound uncommitted git-diff shape (backlog).
- **BUG-151** — add-attachment truncates the source file to 0 bytes on self-copy into spec/attachments/<ID>/ (backlog; bit this workflow twice).

## Final Verification (independent supervisor re-run, 2026-07-10)
- cont005 7/7 ✅, cont006 7/7 ✅, cont009 7/7 ✅, cont007 6/6 ✅, cont008 9/9 ✅
- Regressions: auto_continue_engine 11 ✅, goal_enforcement 18 ✅, goal_command_surface 11 ✅, cont002 TUI 13 ✅ (89 tests green total)
- Feature files valid (per-file), coverage 100% on all five features, audit-coverage clean
- clippy clean on all touched crates; no --workspace runs at any point
