# Review: RPC-399 — Settled tool card must stay pinned to end of output

**Date:** 2026-07-01
**Reviewer:** Claude Code (fspec review-skill), via subordinate ACDD reviewer
**Work Units Reviewed:** 1 (RPC-399, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 1
- 🟢 Observations: 3

## Status: PASS

## 🔴 Critical Issues (Must Fix)
None

## 🟡 Warnings (Should Fix)
1. **Stale comment describes the OLD (superseded) behavior in production code.**
   `codelet/fspec-tui/src/store/agent_view/chunk_processor.rs:175-177` still reads:
   *"a ToolResult settles the card — clear the streaming flag so `wrap_source`
   switches the inline view from the tail window to the **first-8 collapse**."*
   After RPC-399 the settled view is **last-8 / end-pinned**, not first-8. This
   comment sits directly on the settle path (`is_streaming = false` at line 178,
   then `rewrap_at(idx)` at line 184) that this fix depends on. Comment-only, no
   behavioral impact → warning, not critical. **FIX: update the comment to
   describe the last-8 end-pinned collapse.**

## 🟢 Observations (Nice to Have)
1. Two integration test files (`settled_tool_card_end_pinned_rpc399.rs` and
   `tool_call_output_collapse_rpc389.rs`) share ~90% helper code and overlapping
   assertions. Acceptable (RPC-389 keeps its regression suite; RPC-399 documents
   the new contract) but helpers could be lifted into `tests/common/`.
2. Windowing logic is centralized in `collapse_tool_body` (single function, both
   branches) — no duplicated windowing. Good DRY posture.
3. RPC-399 test's `has_line` helper uses whole-line equality, correctly avoiding
   the substring trap (`"line-1"` matching `"line-12"`). Key correctness guard,
   done right.

## Coverage Verification
- Feature file(s):
  `spec/features/settled-tool-card-pinned-to-end.feature` — OK (@RPC-399 tag,
  doc string, correct G/W/T, 5 end-pinned scenarios).
  `spec/features/tool-call-output-collapse.feature` — OK (RPC-389 revised to
  last-8; no lingering "first 8" wording in feature text).
- Test file(s):
  `codelet/fspec-tui/tests/settled_tool_card_end_pinned_rpc399.rs` — OK
  (@step exact-match, whole-line assertions of end-pinned behavior).
  `codelet/fspec-tui/tests/tool_call_output_collapse_rpc389.rs` — OK (updated
  settled assertions to last-8).
- Impl file(s):
  `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs` — OK (collapse_tool_body
  settled branch 210-215 slices `body_lines[total-8..]`; 266 lines; no
  unwrap/expect/panic/todo/FIXME; diff-card bypass unchanged). End-to-end wired
  via `chunk_processor::handle_tool_result` → `rewrap_at` → `wrap_source`.
- Scenario coverage: 5/5 (both features 100%).

## Build & Test
- `cargo test -p codelet-fspec-tui --test settled_tool_card_end_pinned_rpc399`: 5 passed, 0 failed
- `cargo test -p codelet-fspec-tui --test tool_call_output_collapse_rpc389`: 5 passed, 0 failed
- `cargo test -p codelet-fspec-tui` (full crate): 2013 passed, 0 failed, 0 ignored
- `cargo clippy -p codelet-fspec-tui --all-targets`: clean

## Files Reviewed
- spec/features/settled-tool-card-pinned-to-end.feature
- spec/features/tool-call-output-collapse.feature
- codelet/fspec-tui/tests/settled_tool_card_end_pinned_rpc399.rs
- codelet/fspec-tui/tests/tool_call_output_collapse_rpc389.rs
- codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs
- codelet/fspec-tui/src/store/agent_view/chunk_processor.rs
- spec/attachments/RPC-399/investigation.md

## Fix Results
- 🟡 Warning 1 (stale first-8 comment in chunk_processor.rs:175-177) → ✅ Fixed:
  comment updated to describe the last-8 end-pinned collapse (RPC-399).

## Final Verification
- All tests pass: ✅ (2013 passed, 0 failed)
- Build/clippy clean: ✅
- Coverage complete: ✅ (5/5 both features)
- Feature files valid: ✅
