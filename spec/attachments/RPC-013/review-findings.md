# Epic Review: RPC-013 — View-aware footer (Board vs Agent) in Rust TUI

**Date:** 2026-05-13
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-013 — no children)

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 3 issues (all on RPC-013)
- 🟢 Observations: 1

## Work Unit Results

### RPC-013: View-aware footer (Board vs Agent) in Rust TUI — WARN

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Should Fix)

1. **Wrong Feature header in all 3 RPC-013 test files.** The test files
   reference `spec/features/rpc013-view-aware-footer.feature`, which does
   NOT exist on disk. The feature was split into three files during
   specifying:
   - `spec/features/rpc013-board-footer.feature`
   - `spec/features/rpc013-agent-footer.feature`
   - `spec/features/rpc013-source-shape.feature`

   Affected files (the `//! Feature: …` doc header at the top):
   - `codelet/fspec-tui/tests/view_board_unit_rpc013.rs:3`
   - `codelet/fspec-tui/tests/view_agent_unit_rpc013.rs:5`
   - `codelet/fspec-tui/tests/source_shape_rpc013.rs:3`

   This breaks the 1:1 test ↔ feature traceability convention required
   by docs/TESTING.md / CLAUDE.md.

2. **Fictional Gherkin step in `rpc013-source-shape.feature`.** Scenario
   "Navigator no longer reserves a Length(1) footer row" line 35 reads:

       Then the method contains exactly one Layout split call whose constraints are [Constraint::Min(0)]

   The actual `navigator.rs::render_with_stores` has **no** `Layout`
   split at all — it forwards `area` directly to the active child
   view. The companion test
   (`source_shape_rpc013.rs::navigator_no_longer_reserves_a_length_1_footer_row`)
   acknowledges this in a comment ("we assert the simpler post-condition
   …") and only asserts the And-steps (no `Constraint::Length(1)` and
   no `self.footer`). The Then step is describing behaviour that does
   not exist in the implementation, and rule [0] / example [4] both say
   the Navigator should NOT reserve a footer chunk.

3. **Dead helper in `source_shape_rpc013.rs`.** `fn _path_helper(p:
   &Path) -> &Path` at line 195-197 is never called. The leading
   underscore only silences the unused-warning. Should be removed.

## 🟢 Observations (Nice to Have)

1. `view_agent_unit_rpc013.rs::agent_view_footer_lives_on_the_bottom_row`
   is an extra row-position test that does not map to any Gherkin
   scenario. It is value-add (it locks the bottom-row invariant) but
   has no `@step` comments and is not linked into coverage. Leaving as-is.

## Coverage Verification
- Feature files: `spec/features/rpc013-board-footer.feature`,
  `spec/features/rpc013-agent-footer.feature`,
  `spec/features/rpc013-source-shape.feature` — OK
- Test files: `view_board_unit_rpc013.rs`, `view_agent_unit_rpc013.rs`,
  `source_shape_rpc013.rs` — Feature header references wrong file (W1)
- Impl files: `views/board.rs`, `views/agent.rs`, `views/navigator.rs`,
  `views/mod.rs` — OK
- Scenario coverage: 10/10 scenarios fully covered (100%)
- All 15 RPC-013 tests pass (`cargo test -p codelet-fspec-tui
  --test view_board_unit_rpc013 --test view_agent_unit_rpc013
  --test source_shape_rpc013`).
- File-size invariant: views/board.rs=241, views/agent.rs=266,
  views/navigator.rs=201, views/mod.rs=29 — all < 300 LoC.

## Files Reviewed
- spec/features/rpc013-board-footer.feature
- spec/features/rpc013-agent-footer.feature
- spec/features/rpc013-source-shape.feature
- codelet/fspec-tui/src/views/board.rs
- codelet/fspec-tui/src/views/agent.rs
- codelet/fspec-tui/src/views/navigator.rs
- codelet/fspec-tui/src/views/mod.rs
- codelet/fspec-tui/src/lib.rs
- codelet/fspec-tui/src/app/state.rs (header inspection)
- codelet/fspec-tui/tests/view_board_unit_rpc013.rs
- codelet/fspec-tui/tests/view_agent_unit_rpc013.rs
- codelet/fspec-tui/tests/source_shape_rpc013.rs
- spec/attachments/RPC-013/typescript-reference.md
- spec/attachments/RPC-013/ast-research-footer-call-sites.md

## Fix Results

### RPC-013: View-aware footer (Board vs Agent) in Rust TUI

- 🟡 W1 (wrong Feature header in 3 test files) → ✅ Fixed: updated the
  `//! Feature: …` doc header at the top of each test file to point at
  its true feature file
  (`tests/view_board_unit_rpc013.rs` → `spec/features/rpc013-board-footer.feature`,
   `tests/view_agent_unit_rpc013.rs` → `spec/features/rpc013-agent-footer.feature`,
   `tests/source_shape_rpc013.rs`   → `spec/features/rpc013-source-shape.feature`).
- 🟡 W2 (fictional Gherkin step in `rpc013-source-shape.feature`) →
  ✅ Fixed: removed the impossible
  `Then the method contains exactly one Layout split call whose constraints are [Constraint::Min(0)]`
  from the "Navigator no longer reserves a Length(1) footer row"
  scenario, leaving the two assertions the test actually proves
  (no `Constraint::Length(1)`, no `self.footer`). Updated the matching
  `// @step` comment in `tests/source_shape_rpc013.rs` to keep test
  ↔ feature step text aligned.
- 🟡 W3 (dead helper `_path_helper` in source_shape_rpc013.rs) →
  ✅ Fixed: removed both the function and the now-unused
  `use std::path::Path;` import.

### Coverage realignment after edits

Removing W3's dead helper + the `use std::path::Path;` import + W2's
`@step` comment shifted line numbers in `source_shape_rpc013.rs`.
Unlinked & re-linked all 5 source-shape scenarios with the new test
ranges + their original impl ranges:
- FooterView removal scenario: tests 42-69, impl views/mod.rs:1-29
- Navigator scenario: tests 73-87, impl views/navigator.rs:95-114
- AgentView splits scenario: tests 91-116, impl views/agent.rs:180-235
- BoardView source scenario: tests 154-179, impl views/board.rs:211-241
- File-size invariant scenario: tests 120-144, impl views/mod.rs:1-29

`audit-coverage` reports `All mappings valid` for all three feature
files (10/10, 6/6, 4/4).

## Final Verification

- All 15 RPC-013 tests pass (3 board + 6 agent + 6 source-shape).
- Full `codelet-fspec-tui` crate test suite: 119 tests passed, 0 failed
  (`cargo test -p codelet-fspec-tui`).
- Build succeeds: `cargo build -p codelet-fspec-tui`.
- Coverage complete: 100% on all three RPC-013 feature files; audit
  reports "All mappings valid".
- Feature files valid: `fspec validate` is clean on
  `rpc013-source-shape.feature` (the only feature touched).
- Tag registry on RPC-013 feature files unchanged (pre-existing
  registry violations elsewhere in the repo are out of scope).
