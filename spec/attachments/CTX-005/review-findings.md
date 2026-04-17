# Epic Review: CTX-005 — Unified Context Window and Configurable Compaction Thresholds

**Date:** 2026-04-16
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 4 (1 parent + 3 children)

## Summary
- 🔴 Critical: 2 issues across 1 work unit (CTX-007) — **FIXED**
- 🟡 Warnings: 12 issues across 3 work units — **8 FIXED, 4 pre-existing/accepted**
- 🟢 Observations: 7 (no action needed)

---

## Review Order
1. CTX-005 (parent story — no direct implementation)
2. CTX-006 (no dependencies among siblings)
3. CTX-007 (depends on CTX-006)
4. CTX-008 (depends on CTX-006 and CTX-007)

---

## Work Unit Results

### CTX-005: Unified Context Window (Parent) — PASS
Parent story with no direct implementation. All work delivered through children CTX-006, CTX-007, CTX-008. No issues found.

### CTX-006: Rust-Authoritative Context Window — PASS

#### 🔴 Critical Issues
None

#### 🟡 Warnings
1. AgentView.tsx is 5,659 lines — **Pre-existing, not caused by CTX-006**
2. session_manager.rs is 7,850 lines — **Pre-existing, not caused by CTX-006**
3. TODOs in AgentView.tsx (lines 2334, 3379, 4121) — **Pre-existing**
4. Extra test "modelEqual" beyond feature scope — **Acceptable bonus coverage**
5. contextFillPercentage through different path than badge — **Both from Rust, acceptable**

#### Coverage Verification
- Feature file: `spec/features/rust-authoritative-context-window.feature` — OK
- Test file: `src/tui/components/__tests__/rust-authoritative-context-window.test.ts` — OK (556 lines, all @step comments match)
- Impl files: `AgentView.tsx`, `session_manager.rs` — OK
- Scenario coverage: **10/10**

---

### CTX-007: Per-Model Configurable Compaction Threshold — WARN → FIXED

#### 🔴 Critical Issues
1. **Unused import `calculate_usable_context` in `stream_loop.rs:17`** — Clippy warning
   → ✅ Fixed: Removed unused import
2. **Duplicated string-to-CompactionThresholdConfig conversion in 4 locations (DRY violation)**
   - `stream_loop.rs:283-286`
   - `session_manager.rs:6712-6715`
   - `session_manager.rs:6783-6786`
   - `session_manager.rs:6844-6847`
   → ✅ Fixed: Added `CompactionThresholdConfig::from_type_value()` factory method, replaced all 4 occurrences

#### 🟡 Warnings
1. `compaction_threshold.rs` is 531 lines — Production code is 220 lines (under limit), tests push it over. **Accepted: idiomatic Rust pattern**
2. Test file is 435 lines — **Accepted: NAPI bridge tests require verbose mock setup**
3. `calculate_summarization_budget` naming potentially misleading — **Accepted: legacy function, callers understand context**

#### Additional Clippy Fixes Applied
- `stream_loop.rs:282`: Needless borrow `&t` → `t` (compaction_threshold_override returns `&str`)
- `session_manager.rs:5938`: Redundant closure → function reference for `get_footer_cwd`
- `session_manager.rs:6739`: `#[allow(clippy::too_many_arguments)]` for NAPI boundary function
- `codelet-tools/session.rs:424`: `trim()` before `split_whitespace()` redundant

#### Coverage Verification
- Feature file: `spec/features/per-model-compaction-threshold.feature` — OK
- Test file: `src/tui/components/__tests__/per-model-compaction-threshold.test.ts` — OK (all @step comments match)
- Impl files: `compaction_threshold.rs`, `stream_loop.rs` — OK
- Scenario coverage: **10/10**

---

### CTX-008: TUI Configuration Fields and NAPI Bridge — WARN → FIXED

#### 🔴 Critical Issues
None

#### 🟡 Warnings
1. **Grammar error in feature file Background** — Missing "I can" in So-that clause
   → ✅ Fixed: `"So that control when..."` → `"So that I can control when..."`
2. **Triple duplicate `@type-system` tag** on ModelSelection scenario
   → ✅ Fixed: Reduced to single `@type-system`
3. **Test file 429 lines** — Consolidated shared `beforeEach` blocks, fixed nesting. **Accepted: NAPI bridge tests require verbose setup**
4. **Dead-code `PROFILE_FORM_FIELDS` export in `provider.ts`** — Stale constant missing compactionThreshold
   → ✅ Fixed: Removed dead `PROFILE_FORM_FIELDS` constant and `ProfileFormField` interface from `types/provider.ts`
5. **Unnecessary dynamic imports in test file** for unmocked modules
   → ✅ Fixed: Converted to static imports at top of file

#### Coverage Verification
- Feature file: `spec/features/compaction-threshold-tui-config.feature` — OK
- Test file: `src/tui/services/__tests__/compactionThresholdTuiConfig.test.ts` — OK (all @step comments match)
- Impl files: 7 TypeScript files + NAPI declarations — OK
- Scenario coverage: **13/13**

---

## Fix Results

### CTX-007: Per-Model Configurable Compaction Threshold
- 🔴 Unused import in stream_loop.rs → ✅ Fixed: Removed `calculate_usable_context` from import
- 🔴 DRY violation (4 duplicate conversion blocks) → ✅ Fixed: Added `from_type_value()` factory method, replaced all 4
- 🟡 Additional clippy warnings → ✅ Fixed: needless borrow, redundant closure, trim_split_whitespace, too_many_arguments

### CTX-008: TUI Configuration Fields and NAPI Bridge
- 🟡 Grammar error in feature file → ✅ Fixed: Added "I can" to So-that clause
- 🟡 Triple duplicate tag → ✅ Fixed: Reduced to single `@type-system`
- 🟡 Dead code in provider.ts → ✅ Fixed: Removed stale PROFILE_FORM_FIELDS and ProfileFormField
- 🟡 Unnecessary dynamic imports → ✅ Fixed: Converted to static imports
- 🟡 Broken describe nesting from dedup → ✅ Fixed: Properly nested NAPI Bridge describe blocks

## Final Verification
- All Rust tests pass: ✅ (26 compaction_threshold tests including 3 new from_type_value tests)
- All TypeScript tests pass: ✅ (37 tests across 3 test files: 14 + 10 + 13)
- Cargo clippy --workspace clean: ✅ (0 warnings, 0 errors)
- Feature files valid: ✅ (758/758)
- Build succeeds: ✅
