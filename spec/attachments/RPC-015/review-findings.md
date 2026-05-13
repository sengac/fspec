# Epic Review: RPC-015 — BoardView header: FSPEC Logo + CheckpointStatus + KeybindingShortcuts

**Date:** 2026-05-13
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (single work unit — no children)

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 3 issues (test header doc references)
- 🟢 Observations: 2 (Gherkin style + silent error handling)

---

## Work Unit Results

### RPC-015: BoardView header: FSPEC Logo + CheckpointStatus + KeybindingShortcuts — ⚠️ WARN

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)

1. **`codelet/git/tests/count_checkpoints_rpc015.rs` line 3** — header doc comment references a non-existent feature file `spec/features/rpc015-checkpoint-counts.feature`. The actual feature file backing this test is `spec/features/rpc015-count-checkpoints-helper.feature` (verified via `Fspec show-coverage`). TESTING.md requires the test file header to reference the feature file; a wrong reference breaks traceability.

2. **`codelet/fspec-tui/tests/checkpoint_counts_rpc015.rs` line 3** — header doc comment references the same non-existent feature file `spec/features/rpc015-checkpoint-counts.feature`. The actual feature file backing this test is `spec/features/rpc015-cross-transport-parity.feature`.

3. **`codelet/fspec-tui/tests/app_bootstrap_rpc015.rs` line 3** — header doc comment references `spec/features/rpc015-board-header.feature`, but the single scenario this test verifies ("BoardStore.checkpoint_counts is updated by Action::CheckpointCountsLoaded") lives in `spec/features/rpc015-app-bootstrap.feature`.

#### 🟢 Observations (Nice to Have)

1. The source-shape scenario "RPC-013 / RPC-014 invariants preserved" (`spec/features/rpc015-source-shape.feature:81`) has a `Given` followed directly by `Then` steps with no `When`. The Gherkin parser tolerates this and the file passes `fspec validate`, but for consistency with the other source-shape scenarios (which use `When a developer reads the file source raw`), an explicit `When` step would document intent more clearly. Non-blocking.

2. `FspecService::checkpoint_counts` (`codelet/rpc/src/lib.rs:410-420`) uses `unwrap_or_default()` to swallow unexpected `gix` errors. Because the RPC trait return type is `CheckpointCounts` (not `Result<CheckpointCounts>`), there is no clean way to surface the error to the caller. However, the sibling code path in `App::bootstrap` (`codelet/fspec-tui/src/app/bootstrap.rs:38`) traces failures at `debug!` so misconfigured cwds aren't silently undiagnosed. A matching `debug!` on the server side would close the diagnostic gap. Non-blocking; outside the strict scope of RPC-015's acceptance criteria.

#### Coverage Verification
- Feature file(s): all 5 RPC-015 feature files — OK
- Test file(s): 5 — OK (warnings on 3 header references — see above)
- Impl file(s): all wired correctly — OK
- Scenario coverage: 23/23 (100%) — every scenario is linked to test + implementation lines

#### Files Reviewed
**Feature files (5):**
- `spec/features/rpc015-app-bootstrap.feature`
- `spec/features/rpc015-board-header.feature`
- `spec/features/rpc015-count-checkpoints-helper.feature`
- `spec/features/rpc015-cross-transport-parity.feature`
- `spec/features/rpc015-source-shape.feature`

**Test files (5):**
- `codelet/fspec-tui/tests/app_bootstrap_rpc015.rs`
- `codelet/fspec-tui/tests/view_board_unit_rpc015.rs`
- `codelet/git/tests/count_checkpoints_rpc015.rs`
- `codelet/fspec-tui/tests/checkpoint_counts_rpc015.rs`
- `codelet/fspec-tui/tests/source_shape_rpc015.rs`

**Implementation files (12):**
- `codelet/git/src/ghost_commit.rs`
- `codelet/rpc-types/src/lib.rs`
- `codelet/rpc/src/lib.rs`
- `codelet/napi/src/git.rs`
- `codelet/fspec-tui/src/transport/mod.rs`
- `codelet/fspec-tui/src/transport/embedded.rs`
- `codelet/fspec-tui/src/transport/websocket.rs`
- `codelet/fspec-tui/src/views/board.rs`
- `codelet/fspec-tui/src/views/board/header.rs`
- `codelet/fspec-tui/src/views/board/logo.rs`
- `codelet/fspec-tui/src/views/board/checkpoint_status.rs`
- `codelet/fspec-tui/src/views/board/keybinding_shortcuts.rs`
- `codelet/fspec-tui/src/app/dispatch.rs`
- `codelet/fspec-tui/src/app/bootstrap.rs`
- `codelet/fspec-tui/src/components/mod.rs`
- `codelet/fspec-tui/src/store/board.rs`

**Helpers (2):**
- `codelet/fspec-tui/tests/common/mod.rs`
- `codelet/git/tests/common/mod.rs`

---

## Pre-Fix Test Verification

- `cargo test --test count_checkpoints_rpc015 -p codelet-git`: ✅ 4/4 passed
- `cargo test --test app_bootstrap_rpc015 -p codelet-fspec-tui`: ✅ 1/1 passed
- `cargo test --test view_board_unit_rpc015 -p codelet-fspec-tui`: ✅ 6/6 passed
- `cargo test --test checkpoint_counts_rpc015 -p codelet-fspec-tui`: ✅ 3/3 passed
- `cargo test --test source_shape_rpc015 -p codelet-fspec-tui`: ✅ 9/9 passed

**Total: 23/23 tests pass on a clean workspace.**

---

## Scope Discipline

Per the user's instruction "keep strictly to the requirements of this card — no scope creep":

- ✅ FIX: Test-header doc references (warnings 1–3) — these are documentation accuracy fixes, not new behaviour. TESTING.md requires the reference; fixing a wrong path is a typo-class correction.
- ⏭️ SKIP: Observations 1 and 2 — these touch concerns outside RPC-015's acceptance criteria. The source-shape Gherkin scenario already passes `fspec validate`; the silent `unwrap_or_default()` is a deliberate design choice given the tarpc `Result`-free return type.

---

## Fix Results

### RPC-015 — Three header-comment corrections

- 🟡 Warning 1: `codelet/git/tests/count_checkpoints_rpc015.rs` line 3 referenced `rpc015-checkpoint-counts.feature`
  → ✅ Fixed: replaced with `rpc015-count-checkpoints-helper.feature`

- 🟡 Warning 2: `codelet/fspec-tui/tests/checkpoint_counts_rpc015.rs` line 3 referenced `rpc015-checkpoint-counts.feature`
  → ✅ Fixed: replaced with `rpc015-cross-transport-parity.feature`

- 🟡 Warning 3: `codelet/fspec-tui/tests/app_bootstrap_rpc015.rs` line 3 referenced `rpc015-board-header.feature`
  → ✅ Fixed: replaced with `rpc015-app-bootstrap.feature`

## Post-Fix Test Verification

- `cargo test --test count_checkpoints_rpc015 -p codelet-git`: ✅ 4/4 passed
- `cargo test --test app_bootstrap_rpc015 -p codelet-fspec-tui`: ✅ 1/1 passed
- `cargo test --test view_board_unit_rpc015 -p codelet-fspec-tui`: ✅ 6/6 passed
- `cargo test --test checkpoint_counts_rpc015 -p codelet-fspec-tui`: ✅ 3/3 passed
- `cargo test --test source_shape_rpc015 -p codelet-fspec-tui`: ✅ 9/9 passed

**Total: 23/23 tests still pass after fixes.**

## Final Verification
- All RPC-015 tests pass: ✅
- Build succeeds: ✅ (no source files mutated — only comment text)
- Coverage complete: ✅ 23/23 scenarios linked
- Feature files valid: ✅ (`Fspec validate` passes for all five)
- Tags valid: ✅ (all RPC-015 feature-file tags are registered; project-wide `validate-tags` shows pre-existing violations unrelated to RPC-015)
