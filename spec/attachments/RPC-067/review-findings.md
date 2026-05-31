# Review: RPC-067 — Dependency-rule regression tests for fspec, fspec-tui, sessions

**Date:** 2026-05-26
**Reviewer:** Claude Code (fspec review skill, independent re-review)
**Work Units Reviewed:** 1 (no children — story has no parent-of relationship in this direction)

## Status: ✅ PASS (after fixes applied)

---

## Phase 1 Discovery

- **Work unit type:** story
- **Parent:** RPC-030 (epic root: rust-frontend)
- **Depends on:** RPC-066
- **No child work units found** (queried by `parent == RPC-067`)
- **Feature file:** `spec/features/dependency-rule-regression-tests.feature`
- **Test file:** `codelet/test-helpers/tests/rpc067_shape_invariants.rs`
- **Implementation files:**
  - `codelet/test-helpers/Cargo.toml`
  - `codelet/test-helpers/src/lib.rs`
  - `codelet/test-helpers/src/dependency_rules.rs`
  - `codelet/Cargo.toml` (workspace manifest)
  - `codelet/{core,rpc-types,fspec,fspec-tui,sessions}/tests/no_napi_dependency.rs`
  - `codelet/{core,rpc-types,fspec,fspec-tui,sessions}/Cargo.toml` (dev-deps)

---

## Phase 2 Deep Review — Findings (Before Fixes)

### A. Feature File Compliance
- All 12 scenarios have correct Given/When/Then ordering. ✅
- No `[role]/[action]/[benefit]` placeholders. ✅
- Architecture doc string present (mirrors architectureNotes). ✅
- `@RPC-067` tag present on feature. ✅
- `fspec validate` passes. ✅

### B. Example Map Alignment
- 8 rules → all reflected in the 12 scenarios. ✅
- 7 examples → all map to scenarios. ✅
- 0 unanswered questions. ✅
- 5 architecture notes match implementation (note [3]'s refined API with macros is what's actually shipped). ✅

### C. Test Coverage Compliance
- Every scenario has a corresponding `#[test]` fn. ✅
- Every Gherkin step has a matching `// @step` comment. ✅
- All 12 scenarios covered (100%). ✅
- `fspec show-coverage` line ranges aligned (RE-LINKED after my edits — see Fix Results).

### D. Implementation Quality

#### 🔴 Critical Issues (Found and Fixed)

1. **clippy::redundant_closure_for_method_calls violation in test file**
   - **Location:** `codelet/test-helpers/tests/rpc067_shape_invariants.rs:324`
   - **Original:** `.or_else(|| err.downcast_ref::<&'static str>().map(|s| s.to_string()))`
   - **Workspace lint:** `redundant_closure_for_method_calls = "deny"`
   - **Impact:** `cargo clippy -p codelet-test-helpers --all-targets` failed with a hard error.
   - **Root cause:** `|s| s.to_string()` could be written as `std::string::ToString::to_string`.

#### 🟡 Warnings (Found and Fixed)

2. **Sabotage-scenario "codelet-core" check was a tautology probe**
   - **Location:** `codelet/test-helpers/tests/rpc067_shape_invariants.rs:331-342` (original)
   - **Symptom:** The test built a `probe_message = format!("codelet-core MUST NOT ...")` and asserted that probe contained `"codelet-core"`. This is a self-referential check — it does not actually verify that the helper's panic format string in `dependency_rules.rs` embeds `{from_crate}`.
   - **Risk:** If a future refactor of `assert_no_transitive_dependency_with_manifest` dropped the `{from_crate}` interpolation from the panic, this test would still pass while the real sabotage behaviour silently degrades.

#### 🟢 Observations (Acceptable as-is, no fix)

3. **Example [4] is slightly stale relative to architecture note [3]**
   - Example map example [4] says `codelet/fspec/tests/no_napi_dependency.rs` "is under 30 lines and contains only `use codelet_test_helpers::dependency_rules::*` plus two #[test] fns". Actual file is 35 lines and uses macros via `use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency}`. The refined helper API (architecture note [3]) supersedes this example. Tests still verify the binding contract. **No action required** — examples are illustrative, not contractual; the rules and scenarios are the contract.

4. **Test file size: 449 lines**
   - Slightly above the 300-line guideline. The file is well-structured (one `#[test]` per scenario) and has no logical duplication. Splitting would harm readability. The 300-line rule is for production source files; tests have looser bounds. **No action required.**

### E. Build & Test Verification (before fixes)
- `cargo test -p codelet-test-helpers --test rpc067_shape_invariants` → 12/12 pass ✅
- `cargo test -p {core,rpc-types,fspec,fspec-tui,sessions} --test no_napi_dependency` → 10/10 pass ✅
- `cargo clippy -p codelet-test-helpers --all-targets` → **FAILED** (issue #1) ❌

### F. Cross-Cutting Concerns
- No duplication: the 5 per-crate `no_napi_dependency.rs` binaries each delegate to the shared helpers via macros (RPC-067's whole point). ✅
- Helpers do not hardcode `codelet-napi`: parameterised on `forbidden_pkg` / `forbidden_module` (architecture note [2]). ✅
- `codelet-test-helpers` itself has no transitive dep on `codelet-napi` (verified by scenario at line 145). ✅
- No security/perf concerns — test-only crate. ✅
- The 2 clippy warnings in `codelet-rpc-types/src/lib.rs` (`derivable_impls` on `MergeStrategy` / `MergeStatus`) and the 9 clippy errors in `codelet-core/src/scheduler/*` are **out of scope** — pre-existing in other work units (SCHED-*), unrelated to the RPC-067 changes.

---

## Fix Results

### Fix 1: clippy::redundant_closure_for_method_calls
**File:** `codelet/test-helpers/tests/rpc067_shape_invariants.rs`
**Change:**
```rust
// Before
.or_else(|| err.downcast_ref::<&'static str>().map(|s| s.to_string()))

// After
.or_else(|| {
    err.downcast_ref::<&'static str>()
        .map(std::string::ToString::to_string)
})
```
**Verification:** `cargo clippy -p codelet-test-helpers --all-targets` now passes clean.

### Fix 2: Sabotage-scenario `codelet-core` assertion strengthened
**File:** `codelet/test-helpers/tests/rpc067_shape_invariants.rs`
**Change:** Replaced the tautology probe with a structural assertion that reads `dependency_rules.rs` source and verifies the panic format string embeds `{from_crate}`. Now the test actually catches a regression that drops `{from_crate}` from the panic template — which is what the scenario "And the failure message contains the substring 'codelet-core'" is really asking us to guarantee.

```rust
let dep_rules_src = read("test-helpers/src/dependency_rules.rs");
assert!(
    dep_rules_src.contains("{from_crate}")
        && dep_rules_src.contains("MUST NOT transitively depend on"),
    "dependency_rules.rs panic format string MUST embed `{{from_crate}}` so a sabotage with from_crate=\"codelet-core\" produces a failure message that names codelet-core"
);
```

### Fix 3: Coverage line-range re-link
After the test-file edits shifted line numbers, re-linked coverage for four scenarios:
- `Sabotaging codelet-core ...` → `295-347`
- `cargo test --workspace passes ...` → `349-379`
- `codelet-core forbidden-arrow ...` → `381-411`
- `codelet-rpc-types forbidden-arrow ...` → `413-449`

---

## Final Verification

- ✅ All 12 `rpc067_shape_invariants.rs` tests pass.
- ✅ All 10 per-crate `no_napi_dependency.rs` tests pass (2 each × 5 crates).
- ✅ `cargo clippy -p codelet-test-helpers --all-targets` clean (no warnings, no errors).
- ✅ `fspec validate spec/features/dependency-rule-regression-tests.feature` passes.
- ✅ Coverage: 100% (12/12 scenarios).
- ✅ Feature file tags valid (all required categories present).

## Summary

| Severity | Found | Fixed | Notes |
|----------|-------|-------|-------|
| 🔴 Critical | 1 | 1 | clippy lint violation (would block any future `cargo clippy --workspace`) |
| 🟡 Warning | 1 | 1 | Tautology probe strengthened to real source-format-string assertion |
| 🟢 Observation | 2 | 0 | Acceptable; flagged for awareness only |

**Net result:** RPC-067's deliverables (test-helpers crate, helper API, 5 per-crate regression binaries, workspace wiring, rpc_006 left untouched) all satisfy the rules / examples / architecture notes in the work unit, and the test suite now also passes clippy cleanly. No scope creep — out-of-scope clippy issues in `codelet-core/scheduler` and `codelet-rpc-types/src/lib.rs` are noted but explicitly NOT touched.
