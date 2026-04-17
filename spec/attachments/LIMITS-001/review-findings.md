# Epic-Wide ACDD Compliance Review — LIMITS-001 + CTX-005

**Reviewed:** 2026-04-16  
**Scope:** LIMITS-001 through LIMITS-007, CTX-005 through CTX-009  
**Reviewer:** ACDD Compliance Reviewer

---

## Executive Summary

Overall quality is **STRONG**. The LIMITS epic successfully resolves a critical architecture bug where `ProviderManager::context_window()` for Claude Opus 4.6 was returning 1,000,000 (models.dev value) instead of 200,000 (API hard limit). The ModelLimitsResolver trait + pure resolution function is a clean, testable design.

### Critical Invariant Verified ✅

**`ProviderManager::context_window()` for Claude Opus 4.6 returns 200,000 (NOT 1,000,000).**

Trace:
1. `ProviderManager::context_window()` → calls `provider_limits_resolver()` (line 801 manager.rs)
2. `provider_limits_resolver()` for Claude → returns `ConstantResolver` with `max_ctx: Some(claude::CONTEXT_WINDOW)` = `Some(200_000)` (line 741)
3. `resolve_context_window(Some(1_000_000), None, resolver)` → `clamp(1_000_000)` → `min(1_000_000, 200_000)` = **200,000** ✅
4. Integration test `claude_opus_4_6_clamps_1m_to_200k_and_128k_to_8192` passes ✅
5. `ProviderManager::for_testing` test confirms `claude.context_window() == 200_000` with 1M registry ✅

---

## Per-Work-Unit Findings

### LIMITS-001 (Parent — No Direct Implementation) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Work unit status | ✅ done | All 6 children done |
| Has no direct code | ✅ | Correctly delegates to children |
| Children all done | ✅ | LIMITS-002–007 all done |
| No estimate needed | ⚪ Info | Parent story, estimate not required |

---

### LIMITS-002 (ModelLimitsResolver Trait) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `modellimitsresolver-trait-provider-veto-authority.feature` with `@LIMITS-002` |
| Gherkin quality | ✅ | Given/When/Then correctly ordered, 8 scenarios |
| Architecture docstring | ✅ | Present in feature file |
| Implementation | ✅ | `codelet/providers/src/model_limits.rs` — trait + pure function |
| Tests with @step | ✅ | 11 unit tests with @step comments matching Gherkin |
| No unwrap in prod | ✅ | No unwrap in production code |
| No TODO/FIXME | ✅ | Clean |
| DRY/SOLID | ✅ | Pure function, single responsibility, testable |
| File length | ✅ | 1102 lines (includes 3 test modules — acceptable) |

**Finding:** `@done` tag is present on the feature file ✅

---

### LIMITS-003 (Provider Implementations) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `provider-model-limits-resolution.feature` with `@LIMITS-003` |
| Gherkin quality | ✅ | 12 scenarios, correct ordering |
| All 6 providers | ✅ | Claude, OpenAI, Gemini, Codex, Z.AI, Copilot |
| Claude max_ctx | ✅ | `Some(200_000)` — correct |
| OpenAI max_ctx | ✅ | `None` — trusts registry |
| Gemini max_ctx | ✅ | `None` — trusts registry, default 1M |
| Codex should_send | ✅ | `false` — API rejects max_output_tokens |
| Z.AI defaults | ✅ | 128k/8192 |
| Copilot defaults | ✅ | 200k/4096 |
| Tests with @step | ✅ | All in `provider_resolver_tests` module |

**Finding:** 🟡 Feature file missing `@done` tag — work unit is done but feature file not tagged.

---

### LIMITS-004 (ProviderManager Refactor) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `fix-providermanager-resolution-chain-use-modellimitsresolver.feature` with `@LIMITS-004` |
| Gherkin quality | ✅ | 8 scenarios including critical invariant |
| Implementation | ✅ | `manager.rs` — `provider_limits_resolver()` + `context_window()` + `max_output_tokens()` |
| ConstantResolver | ✅ | Lightweight stub avoids credential requirement |
| select_model stores raw | ✅ | Lines 349-350 store raw registry values |
| context_window() clamps | ✅ | Lines 800-807 resolve through resolver |
| override_model_limits | ✅ | Lines 827-838 store as user overrides |
| raw_model_context_window | ✅ | Lines 847-853 return clamped values |
| Tests | ✅ | Comprehensive test coverage in `manager::tests` |

**Finding:** 🟡 Feature file missing `@done` tag.  
**Finding:** 🟡 manager.rs is 2141 lines — significantly over the 300-line guideline. However, this includes ~1000 lines of tests.

---

### LIMITS-005 (Compaction Threshold Verification) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `compaction-threshold-clamped-inputs.feature` with `@LIMITS-005` |
| Gherkin quality | ✅ | 7 scenarios with correct ordering |
| Architecture note | ✅ | "Verification-only story" correctly documented |
| Tests | ✅ | Existing compaction tests consume clamped values |

**Finding:** 🟡 Feature file missing `@done` tag.

---

### LIMITS-006 (TUI Badge/Fill Display) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `tui-badge-and-fill-display-end-to-end-verification.feature` with `@LIMITS-006` and `@done` |
| Gherkin quality | ✅ | 5 scenarios |
| SessionHeader.tsx | ✅ | `compactionThreshold` prop, `badgeValue = compactionThreshold ?? contextWindow` (line 165) |
| AgentView.tsx | ✅ | Reads `rustModel.compactionThreshold` (line 1196), passes to SessionHeader (line 5246) |
| No `any` types | ✅ | Clean TypeScript |
| No console.log | ✅ | Clean |

**Finding:** 🟡 AgentView.tsx is 5650 lines — massively over the 300-line limit. This is a pre-existing issue, not caused by LIMITS.

---

### LIMITS-007 (Integration Tests) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `integration-tests-all-provider-model-combinations.feature` with `@LIMITS-007` and `@done` |
| Gherkin quality | ✅ | 10 scenarios covering all providers |
| Test coverage | ✅ | 32 integration tests in `integration_all_providers` module |
| All tests pass | ✅ | 254 total tests pass in codelet-providers |
| @step comments | ✅ | All tests have @step comments |
| Edge cases | ✅ | Zero values, user override priority, sub-agent propagation |

---

### CTX-005 (Parent — Unified Context Window) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Work unit status | ✅ done | All 4 children done |
| Children listed | ✅ | CTX-006, CTX-007, CTX-008, CTX-009 |
| No direct code | ✅ | Parent story |

---

### CTX-006 (Rust-Authoritative Context Window) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `rust-authoritative-context-window.feature` with `@CTX-006` and `@done` |
| Gherkin quality | ✅ | 10 scenarios including resume and sub-agent |
| Architecture docstring | ✅ | Detailed approach documented |

---

### CTX-007 (Per-Model Compaction Threshold) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `per-model-compaction-threshold.feature` with `@CTX-007` and `@done` |
| Gherkin quality | ✅ | 10 scenarios including edge cases |
| Implementation | ✅ | `compaction_threshold_override` field in ProviderManager |

---

### CTX-008 (TUI Config Fields) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `compaction-threshold-tui-config.feature` with `@CTX-008` and `@done` |
| Gherkin quality | ✅ | 13 scenarios covering parsing, UI, NAPI bridge |

---

### CTX-009 (SessionHeader Badge) ✅

| Check | Status | Notes |
|-------|--------|-------|
| Feature file | ✅ | `sessionheader-badge-threshold.feature` with `@CTX-009` |
| Gherkin quality | ✅ | 5 scenarios |
| Implementation | ✅ | `badgeValue = compactionThreshold ?? contextWindow` in SessionHeader.tsx |

**Finding:** 🟡 Feature file missing `@done` tag.

---

## Clippy Check ✅

```
cargo clippy -p codelet-providers -- -D warnings
```
Result: **0 warnings, 0 errors** — clean build.

---

## Code Quality Checks

### Rust Production Code
| Check | Result |
|-------|--------|
| unwrap() in prod | ✅ None (only in test code) |
| todo!()/unimplemented!() | ✅ None |
| TODO/FIXME/HACK/XXX | ✅ None (only in vendored markdown) |
| Clippy warnings | ✅ Zero |

### TypeScript Code
| Check | Result |
|-------|--------|
| `any` types | ✅ None in SessionHeader.tsx |
| `console.log` | ✅ None in SessionHeader.tsx |
| Files >300 lines | 🟡 AgentView.tsx (5650 lines) — pre-existing |

---

## Summary of Findings

### 🟢 Passing (No Action Required)
1. Critical invariant verified: Claude Opus 4.6 returns 200k ✅
2. All 254 Rust tests pass ✅
3. Clippy clean ✅
4. No unwrap/todo/unimplemented in production code ✅
5. ModelLimitsResolver trait is well-designed (pure function, testable) ✅
6. All 6 providers correctly implement the trait ✅
7. ProviderManager resolution chain correct ✅
8. TUI badge shows compactionThreshold (not raw contextWindow) ✅
9. All feature files have correct @WORK-UNIT-ID tags ✅
10. All feature files have architecture docstrings ✅
11. Gherkin quality is good across all feature files ✅

### 🟡 Warnings (Should Fix)
1. **Missing `@done` tags on 4 feature files:** LIMITS-003 (`provider-model-limits-resolution.feature`), LIMITS-004 (`fix-providermanager-resolution-chain-use-modellimitsresolver.feature`), LIMITS-005 (`compaction-threshold-clamped-inputs.feature`), CTX-009 (`sessionheader-badge-threshold.feature`) — work units are done but feature files not tagged.
2. **manager.rs is 2141 lines** — over 300-line guideline. ~1000 lines are tests. Production code portion is ~960 lines, still over limit but large proportion is boilerplate constructors and helper methods.

### 🔴 Critical Issues
None found.

---

## Fix Log

### Fix 1: Add @done tags to 4 feature files

**Files fixed:**
- `spec/features/provider-model-limits-resolution.feature` — added `@done`
- `spec/features/fix-providermanager-resolution-chain-use-modellimitsresolver.feature` — added `@done`
- `spec/features/compaction-threshold-clamped-inputs.feature` — added `@done`
- `spec/features/sessionheader-badge-threshold.feature` — added `@done`

**Status:** ✅ Fixed — verified via `fspec validate` (all 767 features valid)

### Fix 2: No 🔴 Critical issues found — no code fixes required

The epic is clean. All critical invariants verified, all tests pass (254/254), clippy clean (0 warnings).
