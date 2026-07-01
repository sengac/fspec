# Epic Review: RPC-394 — Edit/Write diffs miss surrounding file context lines

**Date:** 2026-06-30
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (RPC-394, bug)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2
- 🟢 Observations: 3

## Work Unit Results

### RPC-394: Edit/Write diffs miss surrounding file context lines — PASS

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)
1. **`new_string` multiple-occurrence → wrong `start_line` / wrong context**
   (`diff_format.rs` `calculate_start_line` + `diff_context.rs:90-104`).
   `read_context` derives the changed span from `start_line = content.find(new_string)`
   — the FIRST occurrence. If `new_string` appears earlier than the actual edit, the
   injected before/after context anchors to the wrong region. Inherited from RPC-390 /
   TS-parity, but RPC-394 amplifies it (now real file lines are sliced around the wrong
   span). Not covered by the example map. → Document as a known limitation.

2. **No regression test for the shared-boundary-line (Equal-inside-fragment) no-double-print
   case** (Rule 6 of the example map). When `old_string`/`new_string` share a boundary line,
   `similar` emits an `Equal`/Context row inside the fragment AND `new_count` covers it, so
   it is correctly excluded from injected after-context — but this subtle invariant is
   untested. → Add a scenario + test locking the no-double-print behaviour.

#### 🟢 Observations (Nice to Have)
1. `read_context` treats `new_string.is_empty()` as Write/pure-addition → returns `None`
   (how the Write scenario reuses the Edit builder). Implicit but documented.
2. Test temp files are never cleaned up (left for OS to reap) — accumulates `/tmp/rpc394_ctx_*`.
3. `merge` concatenates before+fragments+after without re-windowing the merged sequence;
   an inline card can show up to ~31 lines (25 collapsed fragment + ≤6 context). Acceptable
   by design.

## Coverage Verification
- Feature file: spec/features/edit-diff-surrounding-file-context.feature — OK
- Test file: codelet/fspec-tui/tests/edit_diff_context_rpc394.rs — OK (@step exact, non-trivial)
- Impl files: diff_context.rs, diff_format.rs, pending_tool_diff.rs, chunk_processor.rs — OK
- Scenario coverage: 5/5 (100%)

## End-to-end wiring (verified)
`build_edit_diff_rows_with_context` ← `produce_diff_strings` (pending_tool_diff.rs) ←
`handle_tool_result` (chunk_processor.rs:158) → writes `source.text`/`full_text`, sets
`is_diff=true` → renders via ChunkSource.text. Reachable from a real Edit tool result.
## Fix Results

### RPC-394
- 🟡 Warning 1 (shared-boundary-line no-double-print untested) → ✅ Fixed: added scenario
  "A shared boundary line is shown once and never duplicated by injected context" + test
  `shared_boundary_line_is_shown_once_and_not_duplicated`
  (edit_diff_context_rpc394.rs:301-355). Characterization test — already-correct behaviour
  (`read_context` slices after-context strictly from `start_idx + new_count`); GREEN from
  start, no production change needed. Locks the invariant.
- 🟡 Warning 2 (multiple-occurrence limitation undocumented) → ✅ Fixed: added a
  `## Known limitation` section to the diff_context.rs module doc (first-occurrence
  `new_string` anchoring, inherited from RPC-390 / TS parity). Doc-only.
- 🟢 Observations 1–3: acknowledged as acceptable-by-design; no change.

## Final Verification
- All tests pass: ✅ (1987 passed, 0 failed; RPC-390 12, RPC-393 17, RPC-394 6)
- Build/clippy: ✅ (`cargo clippy --all-targets -- -D warnings` clean)
- Format: ✅ (`cargo fmt --check` clean)
- Coverage complete: ✅ 6/6 scenarios; `audit-coverage` 13/13 files, all mappings valid
- Feature file valid: ✅
- File sizes: ✅ diff_context.rs 143 LoC, all production files <300 LoC

## Final Status: ✅ PASS — all review warnings addressed

