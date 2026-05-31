# Review: RPC-038 — Create codelet-sessions crate skeleton

**Date:** 2026-05-20
**Reviewer:** Claude Code (review-skill)
**Status:** ✅ PASS (after fixes)

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 1 (fixed)
- 🟢 Observations: 5

## Findings (Pre-Fix)

### 🔴 Critical Issues
None.

### 🟡 Warnings

1. **Stale coverage line ranges** — All six scenarios in
   `spec/features/codelet-sessions-crate-skeleton.feature.coverage`
   pointed at test line ranges that were 6–49 lines off from the
   actual functions in `codelet/sessions/tests/skeleton_invariants.rs`.
   Coverage said scenarios 1–6 spanned 32-72, 74-107, 109-201,
   203-242, 244-289, and 291-323; the real `#[test] fn` bodies live
   at 32-78, 80-113, 115-234, 236-282, 284-333, and 335-372
   respectively. The test file grew after the original
   `link-coverage` call and the metadata was never refreshed.

### 🟢 Observations

1. Build + all 7 tests + clippy (`-D warnings`) all pass cleanly on
   the existing skeleton.
2. Feature file Given/When/Then ordering is correct in every
   scenario (And steps only appear after Then as additional
   assertions).
3. All 10 example-map rules and 6 examples map cleanly onto the 6
   Gherkin scenarios — Rule [1] (every dep workspace-versioned) is
   covered indirectly by Scenario 2 (cargo build succeeds), which
   is the right granularity for a skeleton card.
4. The attachment's hint for `[dev-dependencies] tokio = { workspace
   = true, features = ["test-util"] }` is not present in
   `codelet/sessions/Cargo.toml` (only `serde_json` is in
   dev-deps). This is intentional: tokio test-util is not needed
   by `skeleton_invariants.rs` or `smoke.rs`, so adding it would
   violate the YAGNI tone of the skeleton.
5. The clippy test inside `skeleton_invariants.rs` runs
   `cargo clippy -p codelet-sessions --all-targets -- -D warnings`,
   which recursively re-checks the test file itself. This is
   intentional and the relaxed `#![allow(clippy::unwrap_used,
   clippy::expect_used, clippy::panic)]` header keeps the loop
   stable.

## Coverage Verification

- Feature file: `spec/features/codelet-sessions-crate-skeleton.feature` — OK
- Test file: `codelet/sessions/tests/skeleton_invariants.rs` — OK (line ranges fixed)
- Smoke test: `codelet/sessions/tests/smoke.rs` — OK
- Impl files: `codelet/Cargo.toml`, `codelet/sessions/Cargo.toml`, `codelet/sessions/src/lib.rs` — OK
- Scenario coverage: 6/6 scenarios linked with up-to-date line ranges

## Files Reviewed

- `spec/features/codelet-sessions-crate-skeleton.feature`
- `spec/features/codelet-sessions-crate-skeleton.feature.coverage`
- `spec/attachments/RPC-038/codelet-sessions-skeleton.md`
- `spec/attachments/RPC-038/ast-research-existing-crate-skeletons.md`
- `codelet/Cargo.toml`
- `codelet/sessions/Cargo.toml`
- `codelet/sessions/src/lib.rs`
- `codelet/sessions/src/background_session.rs`
- `codelet/sessions/src/session_manager.rs`
- `codelet/sessions/tests/smoke.rs`
- `codelet/sessions/tests/skeleton_invariants.rs`

## Fix Results

### RPC-038 — Coverage line ranges

- 🟡 Stale test line ranges in `.feature.coverage` → ✅ Fixed:
  unlinked all six scenarios and re-linked each with the correct
  `#[test] fn` line ranges (32-78, 80-113, 115-234, 236-282,
  284-333, 335-372).

## Final Verification

- `cargo build -p codelet-sessions`: ✅
- `cargo test -p codelet-sessions` (7/7 tests pass): ✅
- `cargo clippy -p codelet-sessions --all-targets -- -D warnings`: ✅
- `fspec validate spec/features/codelet-sessions-crate-skeleton.feature`: ✅
- Coverage: 100% (6/6 scenarios) with accurate line ranges: ✅
