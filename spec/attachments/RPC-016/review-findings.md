# Review: RPC-016 — Per-column scroll viewport + indicators + keyboard nav for BoardView

**Date:** 2026-05-15
**Reviewer:** Claude Code (fspec review skill)
**Work Unit:** RPC-016 (story, status `done`, estimate 8, parent RPC-002)

## Status: WARN (functionally correct, coverage line ranges need fixing)

---

## Fix Results

- **C1 — view_board_unit_rpc016.rs coverage line ranges wrong:** ✅ Fixed. All 11 scenarios in `rpc016-board-viewport.feature.coverage` re-linked with accurate test line ranges (164-190, 193-220, 223-247, 250-278, 281-301, 304-329, 332-362, 365-394, 397-423, 426-452, 455-489) and corrected implementation line ranges that point at the actual handler regions in `views/board.rs` and `views/board/viewport.rs`.
- **W1 — source_shape_rpc016.rs coverage line ranges off:** ✅ Fixed. All 8 scenarios in `rpc016-source-shape.feature.coverage` re-linked (42-52, 55-70, 73-98, 101-146, 149-159, 162-194, 197-223, 226-260) with accurate test ranges.
- **W2 — Misleading doc comment in `StateHistoryEntry`:** ✅ Fixed. Comment in `codelet/core/src/work_units.rs` now correctly states that `#[serde(default)]` is applied to the unused `from`/`to`/`actor` fields and that only `timestamp` is left strict.
- **O1 — End key UX gap:** Observation only, intentionally out of scope per architecture note [4]. No fix.

## Final Verification

- `cargo build -p codelet-fspec-tui` — ✅ clean build (no warnings)
- `cargo test --test store_board_viewport_rpc016 --test view_board_unit_rpc016 --test source_shape_rpc016` — ✅ 28/28 passed
- `fspec audit-coverage rpc016-board-viewport` — ✅ All files found (22/22), All mappings valid
- `fspec audit-coverage rpc016-board-store-viewport` — ✅ All files found (18/18), All mappings valid
- `fspec audit-coverage rpc016-source-shape` — ✅ All files found (16/16), All mappings valid
- Feature files valid (Gherkin syntax)

---

## 🔴 Critical Issues (Must Fix)

### C1. view_board_unit_rpc016.rs coverage line ranges are systematically wrong

All 11 scenarios in `spec/features/rpc016-board-viewport.feature.coverage` point to incorrect test line ranges. The coverage file was apparently linked before the test helper functions (`normalize_wide_char_padding`, `backlog_joined`) were added at lines 138–161, so all subsequent ranges are shifted by ~24 lines.

| Scenario | Coverage says | Actual test fn |
|----------|--------------|----------------|
| Column with no scroll renders the down arrow on the last viewport row | 140–168 | **164–190** |
| Column with mid-range scroll renders both up and down arrows | 169–198 | **193–220** |
| Column with fewer units than viewport_height renders no arrows | 199–225 | **223–247** |
| Most-recently-changed work unit renders the ⏩ ⏩ prefix and suffix | 226–256 | **250–278** |
| Work unit with an attached session renders the 🟢 prefix | 257–279 | **281–301** |
| Last-changed and session-attached indicators stack on the same unit | 280–307 | **304–329** |
| PageDown advances the focused column's selection by viewport_height rows | 308–340 | **332–362** |
| PageUp scrolls the focused column's selection back by viewport_height rows | 341–372 | **365–394** |
| Home jumps the focused column's selection to the first unit | 373–401 | **397–423** |
| End jumps the focused column's selection to the last unit | 402–430 | **426–452** |
| RPC-014 details strip and RPC-015 header are still painted after RPC-016 lands | 431–467 | **455–489** |

Range 140–168 in particular straddles the helper `normalize_wide_char_padding` (lines 143–157), so a maintainer navigating to that range would land inside a helper, not the test.

---

## 🟡 Warnings (Should Fix)

### W1. source_shape_rpc016.rs coverage line ranges are off

Most scenarios in `rpc016-source-shape.feature.coverage` are off by 2–8 lines:

| Scenario | Coverage says | Actual test fn |
|----------|--------------|----------------|
| WorkUnitInfo gains the last_state_change_at field | 44–54 | 42–52 |
| BoardStore declares the scroll_offsets field and viewport methods | 100–142 | 101–146 |
| Viewport painter module exists as a separate file | 143–155 | 149–159 |
| New and modified board modules stay under 300 lines | 156–190 | 162–194 |
| RPC-013 / RPC-014 / RPC-015 invariants preserved | 191–219 | 197–223 |
| Views still avoid encapsulated transport crates and host runtime construction | 220–254 | 226–260 |

### W2. Misleading doc comment in `codelet/core/src/work_units.rs::StateHistoryEntry`

The doc comment claims `#[serde(default)]` "is omitted" on the unused fields:

> `#[serde(default)]` on unused fields would be acceptable but is omitted here to keep the deserializer strict about the timestamp shape.

But the code immediately below applies `#[serde(default, rename = "from")]`, `#[serde(default, rename = "to")]`, and `#[serde(default, rename = "actor")]`. The comment contradicts the code. Either drop the `serde(default)` attributes or fix the comment to say they are applied.

---

## 🟢 Observations (Nice to Have)

### O1. End key UX gap vs. example map wording

Example [4] in the example map reads: *"…pressing End sets selected_index=units.len()-1 **with appropriate scroll**"*. The implementation of `select_last_in_focused()` only sets `selected_index`; the scroll offset is left untouched. Architecture note [4] explicitly documents this as a deliberate design (the Action variant is argument-free), and the corresponding Gherkin scenario only asserts `selected_index_for("backlog") returns 29`, so this is **not** a scenario failure — but a user on a column with more units than the viewport will still see the cell off-screen after pressing End until they press another arrow key.

This is preserved as documented behaviour, not a defect against the feature file. No fix required without expanding scope.

---

## Coverage Verification

- **Feature files** (3):
  - `spec/features/rpc016-board-store-viewport.feature` — ✅ OK (9 scenarios, valid Given/When/Then ordering, @RPC-016 tag, architecture doc string present)
  - `spec/features/rpc016-board-viewport.feature` — ✅ OK (11 scenarios)
  - `spec/features/rpc016-source-shape.feature` — ✅ OK (8 scenarios)
- **Test files** (3):
  - `codelet/fspec-tui/tests/store_board_viewport_rpc016.rs` — ✅ 9/9 tests pass with @step comments matching Gherkin verbatim
  - `codelet/fspec-tui/tests/view_board_unit_rpc016.rs` — ✅ 11/11 tests pass with @step comments; **coverage line ranges wrong (C1)**
  - `codelet/fspec-tui/tests/source_shape_rpc016.rs` — ✅ 8/8 tests pass; coverage line ranges slightly off (W1)
- **Implementation files**:
  - `codelet/rpc-types/src/lib.rs` — ✅ `last_state_change_at: Option<String>` added with NAPI rename
  - `codelet/core/src/work_units.rs` — ✅ `stateHistory` → `last_state_change_at` mapping with `#[serde(default)]` legacy fallback
  - `codelet/fspec-tui/src/components/mod.rs` — ✅ four new Action variants
  - `codelet/fspec-tui/src/store/board.rs` — ✅ 236 LoC, `scroll_offsets` field declared with `pub(super)` visibility
  - `codelet/fspec-tui/src/store/board_viewport.rs` — ✅ 126 LoC, all 6 mutation methods
  - `codelet/fspec-tui/src/views/board.rs` — ✅ 274 LoC, `last_viewport_height: Cell<u16>`, PageUp/PageDown/Home/End wiring
  - `codelet/fspec-tui/src/views/board/viewport.rs` — ✅ 162 LoC, viewport painter with ↑/↓/⏩/🟢 indicators
  - `codelet/fspec-tui/src/app/dispatch.rs` — ✅ all four Action variants routed; SelectNext/SelectPrev call `move_selection(±1, last_viewport_height)`
- **Scenario coverage:** 28/28 scenarios covered (100%)

## Build & Test Verification

- `cargo build -p codelet-fspec-tui` — ✅ clean build
- `cargo test --test store_board_viewport_rpc016 --test view_board_unit_rpc016 --test source_shape_rpc016` — ✅ 9 + 11 + 8 = **28 tests pass, 0 failures**

## ACDD Chain — Example Map → Rules → Scenarios → Tests → Impl

| Rule | Scenarios | Test | Impl |
|------|-----------|------|------|
| [0] per-column scroll_offset, viewport_height rows | store: default, set_scroll_offset_for | store_board_viewport_rpc016 | store/board.rs:49, store/board_viewport.rs:16,23 |
| [1] ↑ on first row when offset > 0, ↓ on last row when more below | view: no scroll, mid-range, fewer units | view_board_unit_rpc016 | views/board/viewport.rs:95–101 |
| [2] auto-scroll when selection crosses viewport | store: move_selection beyond/above | store_board_viewport_rpc016 | store/board_viewport.rs:29, adjust_scroll_offset |
| [3] PageUp/PageDown/Home/End scroll/jump | view: PageDown/PageUp/Home/End | view_board_unit_rpc016 | views/board.rs:117–134, store/board_viewport.rs:44,51,66 |
| [4] ⏩ {session_indicator}{id}{points} ⏩ | view: most-recently-changed | view_board_unit_rpc016 | views/board/viewport.rs:145–162 |
| [5] 🟢 prefix for attached sessions | view: attached session, stacked | view_board_unit_rpc016 | views/board/viewport.rs:150,156 |
| [6] WorkUnitInfo additive Option<String> | source-shape: gains, reads stateHistory | source_shape_rpc016 | rpc-types/src/lib.rs:59, core/src/work_units.rs:116 |
| [7] no new RPC methods | (cross-cutting) | rpc.rs unchanged | n/a |
| [8] no TS source modified | (cross-cutting) | `git status` — clean for src/tui/ | n/a |
| [9] < 300 LoC per file | source-shape: stays under 300 | source_shape_rpc016 | all files verified ≤ 274 |

## Files Reviewed

- spec/features/rpc016-board-store-viewport.feature
- spec/features/rpc016-board-viewport.feature
- spec/features/rpc016-source-shape.feature
- spec/features/rpc016-board-store-viewport.feature.coverage
- spec/features/rpc016-board-viewport.feature.coverage
- spec/features/rpc016-source-shape.feature.coverage
- spec/attachments/RPC-016/typescript-reference.md
- spec/attachments/RPC-016/ast-research-integration-sites.md
- codelet/rpc-types/src/lib.rs
- codelet/core/src/work_units.rs
- codelet/fspec-tui/src/components/mod.rs
- codelet/fspec-tui/src/store/board.rs
- codelet/fspec-tui/src/store/board_viewport.rs
- codelet/fspec-tui/src/views/board.rs
- codelet/fspec-tui/src/views/board/viewport.rs
- codelet/fspec-tui/src/views/board/columns.rs
- codelet/fspec-tui/src/app/dispatch.rs
- codelet/fspec-tui/tests/store_board_viewport_rpc016.rs
- codelet/fspec-tui/tests/view_board_unit_rpc016.rs
- codelet/fspec-tui/tests/source_shape_rpc016.rs
