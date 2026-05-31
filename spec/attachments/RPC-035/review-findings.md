# Review: RPC-035 — Reduce codelet-napi persistence to thin napi_bindings.rs shims

**Date:** 2026-05-20
**Reviewer:** Claude Code (fspec review-skill)
**Work Units Reviewed:** 1 (leaf story; no children)

## Summary
- 🔴 Critical: 1 issue (coverage line range) — **FIXED**
- 🟡 Warnings: 3 issues — **1 FIXED, 2 LEFT-AS-IS WITH JUSTIFICATION**
- 🟢 Observations: 1

## Status: PASS (after fixes)

---

## Findings

### 🔴 C1. Scenario 7 coverage points to the wrong test function — **FIXED**

**Scenario:** `codelet-rpc-embedded continues to consume persistence without re-introducing the forbidden rpc to napi arrow`

- **Recorded (pre-fix):** `codelet/core/tests/persistence_napi_shim_layout_test.rs:302-335` (this range is *inside* the lazy-init test for Scenario 5).
- **Actual function:** `test_codelet_core_persistence_surface_still_exposes_expected_symbols` at lines **342-373**.

**Fix applied:** unlinked + relinked with `--test-lines 342-373` and `--impl-file codelet/core/src/persistence/mod.rs --impl-lines 1-68`.

### 🟡 W1. Stale doc comment in `codelet/core/src/persistence/mod.rs` — **FIXED**

Pre-fix lines 17-21 described the post-RPC-034 state ("~30-line thin facade that wraps set_data_directory"). RPC-035 reduces the NAPI mod.rs to 16 lines and removes the `set_data_directory` wrapper.

**Fix applied:** rewrote the comment to describe the post-RPC-035 state (mod.rs is now a ~15-line facade; credentials + knowledge-graph reset is inlined into `persistence_set_data_directory`; the 48-test + 9-test suites moved to codelet-core).

### 🟡 W2. Minor coverage off-by-N drifts — **LEFT-AS-IS** (within tolerance)

Drifts of 1-5 lines on Scenarios 1, 2, 5, 8. The `link-coverage` ranges cover the correct test function in every case (only the precise start/end boundary differs by a few lines). Re-linking would not materially improve traceability. The critical wrong-function issue (C1) is the only one that mattered and has been fixed.

### 🟡 W3. Gherkin Scenario 9 references a non-existent feature flag — **LEFT-AS-IS** (out of scope)

Scenario 9 line 123 says ``cargo build -p codelet-napi --features napi`` but `codelet/napi/Cargo.toml` only declares two features: `noop` and `__full_runtime`. The "napi" build is the default cargo build (no `--features`).

`cargo build -p codelet-napi --features noop` fails BEFORE and AFTER RPC-035 due to a transitive interaction between `codelet-rpc-types` (`#[cfg_attr(feature = "napi", napi(...))]`) and `napi-derive/noop`. This is a pre-existing condition that was not introduced by RPC-035.

The actual test (`test_both_codelet_napi_feature_builds_succeed_after_relocation`) only asserts source-level preconditions (mod.rs shape, the `#[cfg(not(feature = "noop"))]` gate, and Cargo.toml feature string presence). It does not spawn cargo. The implementation is therefore consistent with the test, but the Gherkin step text is misleading.

**Left as-is reasoning:** rewriting the Gherkin step text to match what the test actually asserts would require coordinated edits to the `@step` comments in the test file. The scenario's *intent* (both feature builds should keep working after relocation) is documented; the broken noop build is a pre-existing transitive issue outside RPC-035 scope. Documenting in this findings file is sufficient — adjusting the scenario would constitute scope creep into a separate Cargo.toml/cfg consistency fix.

### 🟢 O1. Rule [4] wording vs actual test caller

Rule [4]: "test-time callers in codelet-napi switch to calling `codelet_core::persistence::ensure_directories()` directly."
The remaining caller in `codelet/napi/tests/subordinate_session_persistence_test.rs:134` uses `codelet_napi::persistence::ensure_directories()`. Functionally identical (the NAPI mod.rs re-exports `pub use codelet_core::persistence::*;`), so the symbol resolves to the codelet-core implementation — no behavioural deviation, only spelling.

---

## Coverage Verification (post-fix)

- Feature file: `spec/features/reduce-codelet-napi-persistence-to-thin-napi-bindings-rs-shims.feature` — OK (well-formed, 9 scenarios, architecture doc string present, `@RPC-035` tag present, `fspec validate` passes)
- Test file: `codelet/core/tests/persistence_napi_shim_layout_test.rs` — OK (9 tests, 48 `@step` markers exactly matching the 48 Gherkin steps)
- Impl files reviewed:
  - `codelet/napi/src/persistence/mod.rs` (16 lines, ≤ 20 limit ✅, no `pub fn`, one `pub use codelet_core::persistence::*;`)
  - `codelet/napi/src/persistence/napi_bindings.rs` (966 lines — explicit `use codelet_core::persistence::{...}` block, no `use super::*;` in file, `Napi*` wire structs preserved with locked field order)
  - `codelet/core/src/persistence/mod.rs` (doc comment now accurate)
  - `codelet/core/src/persistence/tests.rs` (48 tests, `use crate::persistence::*;`, `TEST_MUTEX` is `pub(super)`)
  - `codelet/core/src/persistence/lazy_init_tests.rs` (9 tests, `use super::tests::TEST_MUTEX`)
  - `codelet/napi/src/test_support.rs` (rewritten as documented: codelet_common::set_data_directory + reset_stores_for_tests + reset_credential_store + reset_graph_db inline)
- Scenario coverage: 9/9 fully covered; Scenario 7 now correctly points to lines 342-373.

## Test Verification Results (post-fix)

- `cargo test -p codelet-core --test persistence_napi_shim_layout_test`: **9 passed**, 0 failed
- `cargo test -p codelet-core --lib persistence` (all persistence tests including relocated suites): **83 passed**, 0 failed
- `cargo test -p codelet-napi --test session_persistence_test`: **23 passed**, 0 failed
- `cargo test -p codelet-napi --test subordinate_session_persistence_test`: **4 passed**, 0 failed
- `cargo test -p codelet-rpc-embedded --test rpc_006_source_shape`: **6 passed**, 0 failed (forbidden-arrow guarantee intact)
- `shasum -a 256 codelet/napi/index.d.ts` = `e5b4e8d7fa24f8cd3081c0cabcb318d1d77c44e21aedf661891e2dd46d03145e` — matches `PRE_CARD_INDEX_DTS_SHA256` constant ✅
- `cargo build -p codelet-napi` (default = "napi" build): succeeds
- `cargo build -p codelet-napi --features noop`: PRE-EXISTING transitive failure unrelated to RPC-035

## Files Modified During Review

1. `codelet/core/src/persistence/mod.rs` — refreshed stale post-RPC-034 doc comment to describe post-RPC-035 state
2. `spec/features/.coverage` data — re-linked Scenario 7 to the correct test function

## Files Reviewed

- spec/features/reduce-codelet-napi-persistence-to-thin-napi-bindings-rs-shims.feature
- codelet/napi/src/persistence/mod.rs
- codelet/napi/src/persistence/napi_bindings.rs (full, 966 lines)
- codelet/napi/src/test_support.rs
- codelet/napi/Cargo.toml
- codelet/core/src/persistence/mod.rs (modified)
- codelet/core/src/persistence/tests.rs (partial — setup helper, headers, test count)
- codelet/core/src/persistence/lazy_init_tests.rs (partial — header, TEST_MUTEX import, setup helper)
- codelet/core/tests/persistence_napi_shim_layout_test.rs (full, 679 lines)
- codelet/napi/tests/subordinate_session_persistence_test.rs (partial — ensure_directories caller)
