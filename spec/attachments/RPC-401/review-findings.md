# Review: RPC-401 — AgentView message line-spacing parity

**Date:** 2026-07-01
**Reviewer:** Claude Code (fspec review-skill) via subordinate ACDD reviewer
**Work Units Reviewed:** 1 (RPC-401, standalone bug — no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2 (1 fixed, 1 accepted observation)
- 🟢 Observations: 3

## Status: PASS

## 🔴 Critical Issues
None.

## 🟡 Warnings
1. **`chunk_wrap.rs` at 293/300 LoC (source-shape ceiling).** Not a violation
   today (7 lines of headroom), pinned by `rpc024-source-shape.feature`.
   → **Accepted as observation.** Flagged for pre-emptive extraction on the
   next touch. No change made — extracting now would be churn without cause.
2. **`chunkprocessor_parity_rpc091.rs::full_round_trip_renders_four_chunks_in_order`
   was weakened to ordering-only assertions.** The RPC-401 separator adjustment
   replaced fixed-index content assertions with `position()` + `<` ordering
   checks, which no longer verify gutter presence/placement.
   → **FIXED.** Restored a single STRONG exact-vector `assert_eq!` over the full
   9-element rendered `session_lines` output (content + one trailing blank gutter
   per chunk; tool-call chunk emits header + `"ok"` body before its gutter). The
   `@step` comment and surrounding checks were left intact.
   - Investigated the review's secondary claim of a `contains()` loosening at
     L417–422 (`tool_result_attaches...`): confirmed via `git diff HEAD` that this
     `contains()` predates RPC-401 and was NOT introduced by this work unit — the
     RPC-401 diff touches only the three `visible`-vector blocks. Nothing to restore.

## 🟢 Observations
1. `is_blank_line` (scrollback_arrows.rs) uses trim-based whitespace detection,
   slightly broader than the exact `Line::default()` the producer emits.
   Defensively correct and harmless; no duplicate helper exists (DRY satisfied).
2. Separator is `Line::default()` (empty spans) — no prefix/marker/color, per rule 4.
3. `paint_selection_arrow_bars` gates the ▲-on-gutter placement behind
   `selected_has_separator`, so legacy source-less chunks preserve the original
   RPC-381 `ly+1` geometry — prevents regression for pre-rendered chunks.

## Coverage Verification
- Feature file: `spec/features/agentview-message-line-spacing-parity-missing-per-message-separator-gutter.feature` — **OK** (Given/When/Then ordering correct; no placeholders; architecture doc string present; tags `@RPC-401 @tui @agent-view @ts-parity @scrollback @rust @done`).
- Test file: `codelet/fspec-tui/tests/message_line_spacing_rpc401.rs` — **OK** (6 tests, verbatim `@step` comments, real store/render-path assertions, feature-file header present).
- Impl files: `codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs`, `codelet/fspec-tui/src/views/agent/scrollback_arrows.rs` — **OK** (no banned constructs; end-to-end wired via `wrap_source`; modal isolation confirmed).
- Scenario coverage: **6/6 (100%)**; `audit-coverage` 12/12 files, all mappings valid.

## Implementation Quality
- End-to-end wiring CONFIRMED: `wrap_source` is the sole wrap entry; `push_source`,
  `insert_source_at`, `rewrap_chunk` all flow through it → separator participates
  in `total_visual_rows`, resize rewrap, painting, and the selection gutters.
- Uniform across all `ChunkKind` via a single `out.push(Line::default())` exit site
  (cleaner than the 3-site approach the arch note anticipated; identical outcome).
- TurnContentModal isolation CONFIRMED: sources raw `ChunkSource.text` /
  `full_text_for_seq` (scrollback_select.rs), never the cached `lines` — separator
  does not leak into the modal.
- No `unwrap`/`expect`/`panic!`/`todo!`/`unimplemented!`/`TODO`/`FIXME`/`HACK` in
  production files. `chunk_wrap.rs` = 293 LoC, `scrollback_arrows.rs` = 201 LoC.

## Existing tests updated for separator parity (verified correct)
5 of 6 are correct +1-per-chunk exact/exact-row adjustments that ADD explicit
blank-gutter checks (not deletions): `chunk_rendering_parity_rpc071.rs`,
`chunk_rendering_parity_rpc078.rs`, `thinking_streaming_parity_rpc093.rs`,
`supervisor_message_rendering_rpc387.rs`, `scrollback_wrap_rpc078.rs`. The 6th
(`chunkprocessor_parity_rpc091.rs`) had one block weakened → restored to a strong
exact-vector assertion (Warning #2 fix).

## Final Verification
- `cargo test -p codelet-fspec-tui`: **2030 passed / 0 failed** ✅
- `cargo clippy -p codelet-fspec-tui --all-targets`: clean ✅
- `cargo fmt --check -p codelet-fspec-tui`: clean ✅
- `fspec validate` (feature): valid ✅
- `fspec show-coverage` / `audit-coverage`: 6/6 scenarios, 12/12 files ✅

## Files Reviewed
- spec/features/agentview-message-line-spacing-parity-missing-per-message-separator-gutter.feature
- codelet/fspec-tui/tests/message_line_spacing_rpc401.rs
- codelet/fspec-tui/src/store/agent_view/chunk_wrap.rs
- codelet/fspec-tui/src/views/agent/scrollback_arrows.rs
- codelet/fspec-tui/src/views/agent/scrollback_select.rs
- codelet/fspec-tui/src/store/agent_view/session_context.rs
- codelet/fspec-tui/src/views/agent/scrollback.rs
- codelet/fspec-tui/src/views/agent/turn_modal.rs
- (diff-reviewed) chunk_rendering_parity_rpc071.rs, chunk_rendering_parity_rpc078.rs,
  chunkprocessor_parity_rpc091.rs, thinking_streaming_parity_rpc093.rs,
  supervisor_message_rendering_rpc387.rs, scrollback_wrap_rpc078.rs

## Fix Results
- 🟡 Warning #2: weakened multi-chunk assertion → ✅ Fixed (strong exact-vector `assert_eq!` restored; full crate re-verified green).
- 🟡 Warning #1: 293/300 LoC ceiling → ✅ Accepted as observation (no change; flagged for next touch).

## Rebuild note
Release binary must be rebuilt to see the fix live:
`cd codelet && cargo build --release -p codelet-cli`.
