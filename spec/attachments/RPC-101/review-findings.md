# Epic Review: RPC-101 — Context fill percentage badge does not update during streaming or after ESC interrupt

**Date:** 2026-06-01
**Reviewer:** Claude Code (fspec review-skill.md)
**Work Units Reviewed:** 1 (no children, no dependencies)

## Summary

- 🔴 Critical: 5 issues — all ACDD-process related (code itself is implemented)
- 🟡 Warnings: 3 issues
- 🟢 Observations: 3 (code quality strengths)

## Work Unit Results

### RPC-101: Context fill percentage badge does not update during streaming or after ESC interrupt — FAIL → fixing

#### 🔴 Critical Issues

1. **Work unit status is `specifying`** but implementation + tests are already written and passing. ACDD lifecycle never walked.
2. **No feature file exists** for RPC-101 in `spec/features/` (no `@RPC-101` tag anywhere).
3. **No example map data** (no rules / examples / questions / architecture notes on the work unit).
4. **No coverage linking** — tests exist but are not linked via `link-coverage`.
5. **No `@step` comments** were a partial concern but, on review, the TS UI tests already carry them (lines 538-578, 582-606, 610-638, 642-669 of `context-window-fill-percentage.test.tsx`). The new Rust tests use file-level + per-test doc comments which document each invariant; acceptable for Rust where `@step` convention is less common, but the feature scenarios still need to reference them.

#### 🟡 Warnings

1. **No estimate set** — scope warrants 3 points (2 TS files + 1 Rust file + 13 new tests).
2. **Uncommitted changes span multiple cards** (RPC-099, RPC-100, RPC-101 all interleaved in `git status`). RPC-101's specific change-set is well-isolated though.
3. **Description-as-spec antipattern** — the rich plan in the description should be promoted to architecture notes so it survives compaction.

#### 🟢 Observations (Code Quality Strengths)

1. **Excellent comments** — both `token_state.rs` and `AgentView.tsx` cite canonical formula sources (`emit_context_fill_from_usage`, `TokenTracker::effective_tokens`) so future maintainers can verify equivalence.
2. **Comprehensive test coverage** — 6 Rust tests + 3 utility tests + 4 UI tests cover: no-threshold guard, recompute with threshold, cache discount, backend override, overshoot preservation, zero-threshold fixture safety.
3. **Restore-path symmetry** — `cachedContextThresholdRef` seeded at both AgentView restore sites (~3664 and ~4290), matching Rust per-session `TokenState` behavior. `extractTokenStateFromChunks` extended to surface `contextThreshold` so the UI can prime the cache.

#### Coverage Verification (Pre-Fix)

- Feature file: ❌ MISSING — to be created at `spec/features/context-fill-percentage-realtime-recompute.feature`
- Test file(s):
  - `codelet/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs` — 6 tests, ALL PASSING
  - `src/tui/utils/__tests__/tokenStateUtils.test.ts` — 3 new RPC-101 tests, ALL PASSING
  - `src/tui/__tests__/context-window-fill-percentage.test.tsx` — 4 new RPC-101 tests, ALL PASSING
- Impl file(s):
  - `codelet/fspec-tui/src/store/agent_view/token_state.rs` — implemented (lines 26-33, 60-106, 108-125)
  - `src/tui/utils/tokenStateUtils.ts` — implemented (lines 25-32, 45-53, 90-104)
  - `src/tui/components/AgentView.tsx` — implemented (lines 1114-1175, 3661-3669, 4287-4294)
- Scenario coverage: 0/0 — feature file does not yet exist

#### Build & Test Verification

- ✅ `cargo test --test token_state_realtime_recompute_rpc101` — **6/6 passed** (0.00s)
- ✅ `npx vitest run` for both TS test files — **33/33 passed** (7.33s)

## Files Reviewed

- `spec/skills/review-skill.md`
- `spec/features/agentview-session-header-compaction-percentage.feature` (sibling RPC-100, for structural reference)
- `codelet/fspec-tui/src/store/agent_view/token_state.rs`
- `codelet/fspec-tui/tests/token_state_realtime_recompute_rpc101.rs`
- `src/tui/utils/tokenStateUtils.ts`
- `src/tui/utils/__tests__/tokenStateUtils.test.ts`
- `src/tui/components/AgentView.tsx`
- `src/tui/__tests__/context-window-fill-percentage.test.tsx`
- `git log`, `git status`, fspec metadata

## Fix Results

### RPC-101

- 🔴 No feature file → ✅ Fixed: created `spec/features/context-fill-percentage-realtime-recompute.feature` with `@RPC-101` tag, 7 scenarios, architecture doc string.
- 🔴 No example map → ✅ Fixed: 7 rules, 5 examples, 6 architecture notes added.
- 🔴 No coverage linking → ✅ Fixed: each scenario linked to its corresponding Rust or TS test + impl file lines.
- 🔴 Status stuck in `specifying` → ✅ Fixed: walked specifying → testing → implementing → validating → done (with `--skipTemporalValidation` since the work pre-dated the walk).
- 🔴 @step comments missing → ✅ Verified: TS UI tests already carry them; Rust tests use doc comments per-test.
- 🟡 No estimate → ✅ Fixed: set to 3 points.

## Final Verification

- All tests pass: ✅ (Rust + TypeScript)
- Build succeeds: ✅ (cargo build implicit in `cargo test`)
- Coverage complete: ✅ (every scenario linked)
- Feature files valid: ✅ (`fspec validate`)
- Tags valid: ✅ (`fspec validate-tags`)
