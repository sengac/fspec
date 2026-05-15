# Review: RPC-023 — Mouse handling: wheel-scroll BoardView columns + click focus + native text-selection toggle

**Date:** 2026-05-15
**Reviewer:** Claude Code (fspec review skill, self-review)
**Status:** ✅ PASS (after fixes)

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 4 — all fixed
- 🟢 Observations: 1

## Files Reviewed

### Feature files
- `spec/features/app-mouse-dispatch.feature`
- `spec/features/boardview-mouse-handling.feature`
- `spec/features/mouse-tracking-toggle.feature`
- `spec/features/rpc023-source-shape.feature`
- All four `.feature.coverage` files (100% scenario coverage; 21 scenarios across 4 features)

### Implementation
- `codelet/fspec-tui/src/mouse/mod.rs` (29 lines)
- `codelet/fspec-tui/src/mouse/hit_test.rs` (93 lines, inline tests)
- `codelet/fspec-tui/src/mouse/toggle.rs` (135 lines)
- `codelet/fspec-tui/src/views/board.rs` (299 lines — at ceiling, intentional)
- `codelet/fspec-tui/src/views/board/mouse.rs` (106 lines)
- `codelet/fspec-tui/src/views/navigator.rs` (Event::Mouse forwarding doc)
- `codelet/fspec-tui/src/app/events.rs` (App::handle_event no longer drops Event::Mouse)
- `codelet/fspec-tui/src/app/dispatch.rs` (SetFocusedColumn / SelectIndexInFocused / ReEnableMouseTracking routing)
- `codelet/fspec-tui/src/components/mod.rs` (3 new Action variants)
- `codelet/fspec-tui/src/store/board_viewport.rs` (select_index_in_focused)

### Tests
- `codelet/fspec-tui/tests/app_mouse_dispatch_rpc023.rs`
- `codelet/fspec-tui/tests/board_mouse_rpc023.rs`
- `codelet/fspec-tui/tests/mouse_toggle_rpc023.rs`
- `codelet/fspec-tui/tests/source_shape_rpc023.rs`

## 🔴 Critical Issues

None.

## 🟡 Warnings (all fixed)

### 1. Three test files referenced the wrong feature in their `//!` header

The project standard (per CLAUDE.md / agent guidelines): _"Test file header must reference the feature file."_

Three of the four RPC-023 test files all incorrectly referenced
`spec/features/boardview-mouse-handling.feature` even though they cover
different features:

| Test file | Was | Now |
|-----------|-----|-----|
| `tests/app_mouse_dispatch_rpc023.rs` | `boardview-mouse-handling.feature` | ✅ `app-mouse-dispatch.feature` |
| `tests/mouse_toggle_rpc023.rs` | `boardview-mouse-handling.feature` | ✅ `mouse-tracking-toggle.feature` |
| `tests/source_shape_rpc023.rs` | `boardview-mouse-handling.feature` | ✅ `rpc023-source-shape.feature` |

**Status:** ✅ Fixed — all four test file headers now point at the
feature file they actually exercise.

### 2. Navigator::handle_event doc-comment said "keyboard event" only

`codelet/fspec-tui/src/views/navigator.rs` was modified by RPC-023 to
participate in the Event::Mouse routing chain (architecture note [8]:
"extend Navigator::handle_event if needed so BoardView::handle_event
sees Event::Mouse"). The actual `match` arm already passes _any_ event
through — but the doc comment still claimed it routes "a keyboard
event to the active sub-view". Stale doc was misleading future readers
about RPC-023's wiring.

**Status:** ✅ Fixed — doc now reads _"Route a keyboard or mouse event
to the active sub-view. RPC-023 extended this from Event::Key-only
forwarding…"_.

## 🟢 Observations (no fix needed)

### O1. board_mouse_rpc023.rs drains a SetFocusedColumn that the scroll_offset scenario does not assert

`Scenario: Click on a content row adds scroll_offset to the clicked row
index` (spec/features/boardview-mouse-handling.feature lines 85–90)
asserts only `Action::SelectIndexInFocused(6)`. The implementation
emits `SetFocusedColumn` first (per rule [4]), so the test must
`rx.try_recv()` to drain it before asserting on the second action. The
drain line has no `@step` because it is not part of the scenario — it
is plumbing the receiver, not asserting behaviour. This is the right
trade-off: the scenario stays focused on the offset arithmetic (per
the example map's example [7]) and the dual-action emission is already
covered by the previous scenario `Left-click on a content row emits
SetFocusedColumn and SelectIndexInFocused`. Leaving as-is.

## Compliance Checks

### A. Feature File Compliance — ✅ PASS

- All four feature files have correct Given/When/Then ordering
- No placeholder text (`[role]`, `[action]`, `[benefit]`)
- Architecture doc strings present on every feature
- `@RPC-023` tag present on every feature
- `fspec validate spec/features/boardview-mouse-handling.feature` → valid

### B. Example Map Alignment — ✅ PASS

- 15 rules → reflected across the 21 scenarios (rules [0]–[14])
- 14 examples → mapped to scenarios (e.g. example [0] → "Wheel-down …
  SelectNext"; example [8] → "temporarily_disable writes
  DisableMouseCapture bytes"; example [12] → "rect_contains is
  half-open …")
- No unanswered questions remain on the work unit
- 9 architecture notes match the code (Decisions Q5, Q6, Q7, Q8, Q9
  all verifiable in source)

### C. Test Coverage Compliance — ✅ PASS

- 21/21 scenarios linked (coverage = 100% across all four features)
- Every Gherkin step has a matching `@step` comment in the test file
- `@step` comment text matches the Gherkin step text exactly (verified
  for all 21 scenarios)
- Tests assert real behaviour (not trivial `expect(true).to_be(true)`)
- Test file headers reference correct feature files (after fix #1)

### D. Implementation Quality — ✅ PASS

- **SRP:** `hit_test.rs` does ONLY hit-testing; `toggle.rs` does ONLY
  capture-toggle lifecycle; `views/board/mouse.rs` does ONLY mouse →
  Action translation.
- **DRY:** `rect_contains` is the single hit-test helper reused by all
  mouse paths — no duplicated band-math (improvement over the TS
  reference which rolled its own per consumer).
- **No shortcuts:** `rg TODO|FIXME|HACK|XXX|todo!|unimplemented!`
  returns no hits inside `codelet/fspec-tui/src`.
- **Wired up end-to-end:** `App::handle_event` (events.rs:34) →
  `Navigator::handle_event` (navigator.rs:68) → `BoardView::handle_event`
  (board.rs:106) → `mouse::handle_mouse` (board/mouse.rs:40) →
  `view.emit(Action::*)` → `App::dispatch` (dispatch.rs:168, 176, 183).
- **Error handling:** `let _ = execute!(...)` correctly discards TTY
  write errors (acceptable in Drop / button-down hot paths); the timer
  uses `tokio::spawn` and `let _ = tx.send(...)` — channel errors mean
  the receiver was dropped, which is the shutdown path.
- **Type safety:** No `as unknown as`, no escape hatches. The Rust
  port uses `W: Write + Send = std::io::Stdout` for clean injection.
- **File size:** All RPC-023 files under the 300 LoC ceiling
  (`views/board.rs` is 299, intentionally at the ceiling per
  architecture note [5]; `mouse.rs` branch lives in `views/board/`).
- **No `unwrap()` / `expect()` / `panic!()` in production code** —
  these are confined to test code, which is gated by
  `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`.

### E. Build & Test Verification — ✅ PASS

- `cargo build -p codelet-fspec-tui` succeeds (zero warnings on the
  RPC-023 surface).
- `cargo test -p codelet-fspec-tui` runs 200+ tests across 36 binaries,
  all green. The four RPC-023 test binaries pass: 1 + 9 + 4 + 7 = 21
  scenarios.
- `fspec validate spec/features/*.feature` → 894/894 files valid.

### F. Cross-Cutting Concerns — ✅ PASS

- No security concerns (no input sanitisation paths in this slice).
- No performance concerns (hit-test is O(1) per event; `for (idx,
  rect) in headers.iter().enumerate()` is bounded by 7 columns).
- Source-shape invariants (rules [11]–[14]) are themselves test-pinned
  in `tests/source_shape_rpc023.rs` so future cards cannot regress
  them silently.
- DisconnectDialog + HelpDialog remain Event::Key-only (rule [13],
  decision Q5).
- Raw SGR escape strings exist nowhere outside crossterm's own crate
  (rule [11]).

## Coverage Verification

| Feature | Scenarios | Coverage | Status |
|---------|-----------|----------|--------|
| `app-mouse-dispatch.feature` | 1 | 100% | OK |
| `boardview-mouse-handling.feature` | 9 | 100% | OK |
| `mouse-tracking-toggle.feature` | 4 | 100% | OK |
| `rpc023-source-shape.feature` | 7 | 100% | OK |
| **TOTAL** | **21** | **100%** | ✅ |

## Fix Results

### RPC-023

- 🟡 Wrong feature header in `app_mouse_dispatch_rpc023.rs` → ✅
  Fixed: header now references `spec/features/app-mouse-dispatch.feature`.
- 🟡 Wrong feature header in `mouse_toggle_rpc023.rs` → ✅ Fixed:
  header now references `spec/features/mouse-tracking-toggle.feature`.
- 🟡 Wrong feature header in `source_shape_rpc023.rs` → ✅ Fixed:
  header now references `spec/features/rpc023-source-shape.feature`.
- 🟡 Stale "keyboard event"-only doc on `Navigator::handle_event` → ✅
  Fixed: doc now mentions mouse routing and the RPC-023 extension.

## Final Verification

- All RPC-023 tests pass: ✅ (21/21)
- Full `cargo test -p codelet-fspec-tui` passes: ✅
- `cargo build -p codelet-fspec-tui` succeeds with zero warnings: ✅
- `fspec validate` passes for all four RPC-023 feature files: ✅
- Feature-level tag validation: ✅ (the four RPC-023 features are not
  among the 283 project-wide tag violations)
- Coverage complete: ✅ 100% across all 21 scenarios

## Conclusion

RPC-023 is **complete and correct**. The four issues identified during
review were all cosmetic / documentation issues that have been fixed.
No implementation, test, feature, or coverage changes were required.
The work unit is ready to return to `done`.
