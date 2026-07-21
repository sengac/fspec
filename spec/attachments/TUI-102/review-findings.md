# Epic Review: TUI-102 — AgentView scrollback scrollbar click-and-drag integration

**Date:** 2026-07-21T08:55:00Z
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: **0** issues
- 🟡 Warnings: **2** issues → **2 fixed** ✅
- 🟢 Observations: **3** → **1 fixed** ✅

## Work Unit Results

### TUI-102: AgentView scrollback scrollbar click-and-drag integration — ✅ PASS (was WARN)

---

## 🔴 Critical Issues (Must Fix)
None

## 🟡 Warnings (Should Fix)

1. **Hardcoded `current_offset: 0` in ScrollbarGeometry causes incorrect thumb position for quick clicks**
   - **File:** `codelet/fspec-tui/src/views/agent/mouse_dispatch.rs`, line 162
   - **Status:** ✅ **FIXED**
   - **What was done:** Added `last_scrollback_scroll_offset` field to `AgentView` struct. During render, the actual scroll offset is now cached via `ctx.scrollback.scroll_state().offset`. The `ScrollbarGeometry` in `handle_scrollback_mouse` now uses `self.last_scrollback_scroll_offset` instead of hardcoded `0`.

2. **Test file `mouse_dispatch_tests.rs` exceeds 300-line limit (385 lines)**
   - **File:** `codelet/fspec-tui/src/views/agent/mouse_dispatch_tests.rs`
   - **Status:** ✅ **FIXED**
   - **What was done:** Extracted 3 integration tests (lines 237-385) into a new file `mouse_dispatch_integration_tests.rs`. The original file is now 228 lines, the new file is 152 lines — both under the 300-line limit.

## 🟢 Observations (Nice to Have)

1. **`last_scrollback_sb_rect` field is set but never read**
   - **File:** `codelet/fspec-tui/src/views/agent.rs`, line 137
   - **Status:** ✅ **FIXED**
   - **What was done:** Removed the `last_scrollback_sb_rect` field entirely. The scrollbar column is derived directly from `last_scrollback_area` in the mouse handler, so the cached rect was redundant.

2. **`dispatch.rs` (310 lines) and `scrollback.rs` (307 lines) slightly exceed 300-line limit**
   - **Status:** ⚠️ Noted — monitor as codebase grows
   - **Detail:** Both files are marginally over the 300-line guideline. Not a blocker.

3. **Stick-to-bottom exit tested indirectly**
   - **Status:** ⚠️ Noted — acceptable as-is
   - **Detail:** The test verifies the action is emitted but doesn't directly verify `stick_to_bottom = false`. The actual `stick_to_bottom` change happens in `dispatch.rs` via `jump_to_offset()`. Acceptable since the `jump_to_offset` method itself is tested in `scrollbar_drag` tests.

## Coverage Verification
- Feature file: `spec/features/agentview-scrollback-scrollbar-click-and-drag-integration.feature` — ✅ OK
- Test file(s): `codelet/fspec-tui/src/views/agent/mouse_dispatch_tests.rs`, `mouse_dispatch_integration_tests.rs` — ✅ OK (5/5 scenarios covered)
- Impl file(s): `codelet/fspec-tui/src/views/agent/mouse_dispatch.rs` — ✅ OK (wired up correctly)
- Scenario coverage: 5/5 scenarios covered (100%)

## Files Reviewed
- `spec/features/agentview-scrollback-scrollbar-click-and-drag-integration.feature`
- `codelet/fspec-tui/src/views/agent/mouse_dispatch.rs`
- `codelet/fspec-tui/src/views/agent/mouse_dispatch_tests.rs`
- `codelet/fspec-tui/src/views/agent/mouse_dispatch_integration_tests.rs` (NEW)
- `codelet/fspec-tui/src/mouse/scrollbar_drag.rs`
- `codelet/fspec-tui/src/views/agent.rs` (AgentView struct, render_with_store)
- `codelet/fspec-tui/src/views/agent/scrollback.rs` (jump_to_offset, render_count_visited)
- `codelet/fspec-tui/src/app/dispatch.rs` (ScrollbackJumpToOffset handler)
- `codelet/fspec-tui/src/components/mod.rs` (Action enum)
- `spec/attachments/TUI-102/ast-research-scrollbar-integration.md`

## Build & Test Verification
- ✅ `cargo build` — Succeeds without errors
- ✅ `cargo test --lib -- mouse_dispatch` — 8 tests pass (5 unit + 3 integration)
- ✅ `cargo test --lib -- scrollbar_drag` — 8 tests pass
- ✅ Zero warnings in compilation

## Fix Results

### TUI-102: AgentView scrollback scrollbar click-and-drag integration
- 🟡 Issue 1 (Hardcoded offset): ✅ Fixed — Added `last_scrollback_scroll_offset` field, cached during render, used in mouse handler
- 🟡 Issue 2 (Test file >300 lines): ✅ Fixed — Split into `mouse_dispatch_tests.rs` (228 lines) and `mouse_dispatch_integration_tests.rs` (152 lines)
- 🟢 Observation 1 (Unused `last_scrollback_sb_rect`): ✅ Fixed — Removed redundant field, simplified render logic

## Final Verification
- All tests pass: ✅
- Build succeeds: ✅
- Coverage complete: ✅ (5/5 scenarios, updated line ranges)
- Feature files valid: ✅
- Tags valid: ✅
- File sizes under 300 lines: ✅ (all modified files now compliant)
