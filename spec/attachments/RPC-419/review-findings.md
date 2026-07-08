# Review Findings: RPC-419 — Context fill badge oscillates during streaming

**Date:** 2026-07-08
**Reviewer:** Claude Code (fspec review skill, dedicated reviewer agent 68b7fa0b)
**Work Units Reviewed:** 1 (RPC-419, bug, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 3
- 🟢 Observations: 6

## Work Unit Results

### RPC-419: Context fill badge oscillates during streaming — PASS (with warnings)

All tests pass (Rust 9/9, TS 16/16 + 21/21), clippy clean with `-D warnings`, `npm run build` clean, coverage 100% on both amended features, formula convergence verified against the backend authority (`emit_context_fill_from_usage` + `ApiTokenUsage::total_context` + `output.rs` wire semantics).

#### 🔴 Critical Issues (Must Fix)
None.

#### 🟡 Warnings (Should Fix)
1. **Unverified final @step in the TS override test** — `src/tui/__tests__/context-window-fill-percentage.test.tsx:797-800`: the step `// @step And the cached threshold MUST remain at 100000 tokens for subsequent TokenUpdates` has no assertion behind it (comment claims indirect verification via the Rust test). Fix: send one more `TokenUpdate{inputTokens: 70000}` and assert the badge renders `[70%]`.
2. **Sibling living documentation contradicts the corrected understanding** — `spec/features/context-window-fill-percentage-indicator.feature` (TUI-033) lines 17-19, 37, 47 still document `Effective tokens = input_tokens - (cache_read_tokens * 0.9)` as the fill-percentage basis, and its scenario "Percentage calculation uses effective tokens with cache discount" presents the discount as current behavior. The backend emits `effective_tokens = total_context()` with no discount. The associated test only asserts display of a backend-supplied value (cannot cause live oscillation), so this is a documentation amendment, not a behavior change.
3. **Root-cause §6 plan vs execution discrepancy** — `spec/attachments/RPC-419/root-cause-analysis.md` §6 lists `src/tui/utils/__tests__/tokenStateUtils.test.ts` as needing expectation updates; it was (correctly) left untouched because its RPC-101 section tests threshold surfacing, not the recompute formula. The attachment should record why the planned change was unnecessary.

#### 🟢 Observations (Nice to Have)
1. Restore-path `calculateContextFillPercentage` still uses `Math.round` and input-only — confirmed sound as out-of-scope (§7): runs once on persisted-session restore before any live chunk; first live update overwrites; worst case one-off ±1%.
2. Rust test `token_state_realtime_recompute_rpc101.rs:78` carries `output_tokens=1000` beyond the literal step text (harmless, strengthens the invariant).
3. Edge-semantics divergence (Rust u16::MAX clamp vs backend u32 cast) requires ≥65,535% fill — unreachable; convergence property holds.
4. Stale line references in the untouched restore feature doc string (`AgentView.tsx:3661-3669/4287-4294` → now 3675-3677/4300-4302) — pre-existing drift.
5. TS file carries four defense-in-depth tests quoting store-feature steps while store coverage links the Rust test — no coverage confusion.
6. `AgentView.tsx` (~5,600 lines) massively exceeds the 300-line guidance — pre-existing; RPC-419's diff was a tight 32-line formula-only change.

#### Coverage Verification
- Feature files: `spec/features/context-fill-percentage-realtime-recompute.feature`, `...-ui.feature` — OK (@RPC-419 + @RPC-101 + @done, doc strings accurate, G/W/T ordering correct); `...-restore.feature` untouched — correct call (never referenced the discount).
- Test files: `codelet/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs`, `src/tui/__tests__/context-window-fill-percentage.test.tsx` — OK apart from Warning 1; `src/tui/utils/__tests__/tokenStateUtils.test.ts` untouched — correct call.
- Impl files: `codelet/fspec-tui/src/store/agent_view/token_state.rs`, `src/tui/components/AgentView.tsx` — OK, identical formula both sides, comments accurate.
- Scenario coverage: store 9/9, UI 4/4 (100%, test + impl links audited valid).

## Fix Results

### RPC-419: Context fill badge oscillates during streaming
- 🟡 Issue 1 (unverified final @step in TS override test) → ✅ Fixed: after the authoritative `ContextFillUpdate{fillPercentage: 62}`, the test now injects `TokenUpdate{inputTokens: 70000, outputTokens: 0}` and asserts `[70%]` renders — proving the cached 100000 threshold survives the override and drives subsequent recomputes. "Verified indirectly" comment removed.
- 🟡 Issue 2 (stale TUI-033 living documentation) → ✅ Fixed: `spec/features/context-window-fill-percentage-indicator.feature` doc string corrected (backend emits `effective_tokens = total_context()` with NO cache discount; the 0.9 discount belongs exclusively to compaction's `TokenTracker::effective_tokens`; corrected by RPC-419). Scenario renamed to "Percentage displays the backend's physical-occupancy calculation verbatim" with reality-matching steps; test describe/it names, @step comments and the 0.9 arithmetic comment synced; `@RPC-419` tag added; coverage re-linked (test 415-441; impl `stream_loop.rs:119-137` + `AgentView.tsx:1159-1183`).
- 🟡 Issue 3 (root-cause §6 plan vs execution) → ✅ Fixed: `root-cause-analysis.md` §6 row for `tokenStateUtils.test.ts` now records "No change needed — verified during testing phase: its RPC-101 section covers extractTokenStateFromChunks threshold surfacing only; no assertion encodes the 0.9 discount or rounding."
- ➕ Bonus: all 6 stale coverage mappings in the TUI-033 feature (deleted `AgentModal.tsx`, incl. one absurd 1650-line range) plus a 7th drifted mapping (`stream_loop.rs:717-724` pointing at stream-retry code) repaired with verified test+impl ranges (`SessionHeader.tsx`, `sessionHeaderUtils.ts`, `AgentView.tsx`); no `skipValidation` used. `audit-coverage` now: All files found (22/22), all mappings valid; coverage 100% (8/8).

## Final Verification
- Rust: `cargo test -p codelet-fspec-tui --test token_state_realtime_recompute_rpc101` → 9 passed; 0 failed ✅ (independently re-run by supervisor after all fixes)
- TS: `vitest run context-window-fill-percentage.test.tsx + tokenStateUtils.test.ts` → 37 passed (16 + 21) ✅ (independently re-run by supervisor)
- `cargo clippy -p codelet-fspec-tui -- -D warnings` → clean ✅
- `npm run build` → clean ✅
- `Fspec validate` → all 1583 feature files valid ✅
- `Fspec validate-tags` → both RPC-419 features + TUI-033 feature pass (482 pre-existing violations in unrelated files, unchanged count) ✅
- Coverage: store 9/9, UI 4/4, TUI-033 8/8 — 100% with valid test+impl mappings ✅

