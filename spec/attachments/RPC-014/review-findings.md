# RPC-014 Review Findings — Rich box-drawing Kanban grid + work-unit details strip

**Date:** 2026-05-13
**Reviewer:** Claude Code (fspec review skill)
**Reviewed Files:**

- `spec/features/rpc014-board-grid.feature`
- `spec/features/rpc014-grid-helpers.feature`
- `spec/features/rpc014-source-shape.feature`
- `codelet/fspec-tui/src/views/board.rs` (orchestrator, 248 LoC)
- `codelet/fspec-tui/src/views/board/grid.rs` (130 LoC)
- `codelet/fspec-tui/src/views/board/details_strip.rs` (173 LoC)
- `codelet/fspec-tui/src/views/board/columns.rs` (124 LoC)
- `codelet/rpc-types/src/lib.rs` (`WorkUnitInfo` definition)
- `codelet/core/src/work_units.rs` (`WorkUnitRecord` parser)
- `codelet/fspec-tui/tests/view_board_unit_rpc014.rs` (8 render tests)
- `codelet/fspec-tui/tests/grid_unit_rpc014.rs` (4 pure-function tests)
- `codelet/fspec-tui/tests/source_shape_rpc014.rs` (7 source-shape regressions)

## Status: PASS (after fixes)

## 🔴 Critical Issues (Must Fix)

None.

## 🟡 Warnings (Found & Fixed)

### 1. Source-shape test `new_and_modified_board_modules_stay_under_300_lines` did not match its Gherkin

**Location:** `codelet/fspec-tui/tests/source_shape_rpc014.rs:56-79` (pre-fix).

The Gherkin scenario reads:

```
Given the directory codelet/fspec-tui/src/views/board/
When a test counts the line-count of every .rs file in views/board/ plus views/board.rs
```

The test however used a **hardcoded three-file list** (`board.rs`, `grid.rs`, `details_strip.rs`) and silently omitted `columns.rs` — the third helper module that actually exists in `views/board/`. This created a coverage gap: any future module added to `views/board/` could violate the 300-LoC ceiling without the regression firing.

**Fix:** Replaced the hardcoded list with a `read_dir` scan of `views/board/` that collects every `*.rs` file plus `views/board.rs`. The scan is supplemented by a sanity assertion that `grid.rs` and `details_strip.rs` are present (defending rule [7]).

### 2. Example map rule [7] stated "two new modules" but the implementation has three

**Location:** Work unit RPC-014, rule [7] (pre-fix).

The rule wording was:

> "The new logic lives in **two new modules** under codelet/fspec-tui/src/views/board/: `grid.rs` and `details_strip.rs`. The orchestrator stays in `views/board.rs` and remains < 300 LoC."

The actual implementation introduces **three** helper modules: `grid.rs`, `details_strip.rs`, **and** `columns.rs`. The third was forced by the 300-LoC ceiling the same rule imposes on `board.rs`: column-header + content-row painters merged into `board.rs` would push it from 248 → ~372 LoC. The fix is a textual correction, not a code change.

**Fix:** Removed the original rule [7] and added a replacement that names all three modules and explains the `columns.rs` extraction as a 300-LoC budget consequence.

## 🟢 Observations (Nice to Have)

- Doc comment in `details_strip.rs` says `row 1: first line of description, truncated to width - 4`. The implementation uses `area.width.saturating_sub(2)` where `area.width` is already `terminal_width - 2` (after `inner_rect`). Effective truncation = `terminal_width - 4`, so doc and behavior agree mathematically; just slightly indirect.
- Stories fall into the `else` branch of the work-type style switch (theme.fg). Rule [5] specifies stories use theme.fg — matches.
- The "centered placeholder" is centered both horizontally and vertically; rule [3] only specified "centered string". Match accepted.

## Coverage Verification

- **Feature files:** `rpc014-board-grid.feature` (8 scenarios), `rpc014-grid-helpers.feature` (4 scenarios), `rpc014-source-shape.feature` (7 scenarios) — all linked at 100%.
- **Test files:** `view_board_unit_rpc014.rs` (8 tests, all @step comments present and exact), `grid_unit_rpc014.rs` (4 tests, all @step comments present), `source_shape_rpc014.rs` (7 tests, all @step comments present).
- **Implementation files:** all reachable via `App::render` → `Navigator::render` → `BoardView::render_with_store` → `details_strip::render` / `columns::paint_*` / `grid::build_border_row`. Wired end-to-end.
- **Coverage:** 100% across all 19 scenarios after fixes.

## Fix Results

| Item | Status |
|------|--------|
| Source-shape test scans all `views/board/*.rs` | ✅ Fixed (`source_shape_rpc014.rs:56-101`) |
| Rule [7] updated to acknowledge three modules | ✅ Fixed (rule [10] now carries the corrected wording) |

## Final Verification

- `cargo test -p codelet-fspec-tui`: **124 tests passed, 0 failed**.
- `cargo build --workspace`: **success**.
- `fspec validate spec/features/rpc014-*.feature`: all three feature files valid.
- Coverage: **100%** across all 19 RPC-014 scenarios.

## Files Reviewed

- spec/features/rpc014-board-grid.feature
- spec/features/rpc014-grid-helpers.feature
- spec/features/rpc014-source-shape.feature
- spec/attachments/RPC-014/typescript-reference.md (work-unit attachment, referenced)
- codelet/fspec-tui/src/views/board.rs
- codelet/fspec-tui/src/views/board/grid.rs
- codelet/fspec-tui/src/views/board/details_strip.rs
- codelet/fspec-tui/src/views/board/columns.rs
- codelet/fspec-tui/src/store/board.rs
- codelet/rpc-types/src/lib.rs
- codelet/core/src/work_units.rs
- codelet/fspec-tui/tests/view_board_unit_rpc014.rs
- codelet/fspec-tui/tests/grid_unit_rpc014.rs
- codelet/fspec-tui/tests/source_shape_rpc014.rs
- src/tui/components/WorkUnitDescription.tsx (TS reference for description truncation)
- src/tui/components/UnifiedBoardLayout.tsx (TS reference for grid math)
