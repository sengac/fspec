# Epic Review: Edit/Write Colored Diff Parity (RPC-390 + RPC-391)

**Date:** 2026-06-30
**Reviewer:** Claude Code (fspec review skill) + 2 parallel reviewer agents
**Work Units Reviewed:** 2 (RPC-390, RPC-391) — reviewed in dependency order.

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 2 issues (both RPC-390, documentation-accuracy)
- 🟢 Observations: several (non-blocking)

## Work Unit Results

### RPC-390: Port Edit/Write diff generation and [R]-/[A]+ marker encoding — WARN
**Build/test:** `diff_format_rpc390` 11/11 pass; clippy zero warnings; fmt clean.
**Coverage:** 11/11 scenarios; links accurate; golden-string byte-for-byte parity verified.
**TS parity:** computeLineDiff/changesToDiffLines/formatDiffForDisplay/formatWithTreeConnectors/calculateStartLine all verified line-by-line. No unwrap/expect/panic/todo in production. File 279 LoC.

- 🟡 **W1 — Misleading scenario title.** `spec/features/agentview-edit-diff-generation.feature:64`
  titles the scenario *"A mid-file change in a large edit shows **leading** and trailing gap
  markers."* The implementation (`diff_format.rs:182-219`, faithful to TS `AgentView.tsx:730-760`)
  never emits a *leading* `... (N lines)` marker — the leading region before the first shown
  index is **dropped**; only a *trailing* gap marker is emitted. Reword the title.
- 🟡 **W2 — Title vs. steps mismatch.** The same scenario's Then-step (line 67) correctly says
  "earlier lines are dropped," contradicting its own title. Align once W1 is fixed.
- 🟢 O1 — `strip_prefix` slices first `char` vs TS `slice(1)` (UTF-16): identical for ASCII prefix; already commented.
- 🟢 O2 — Spec attachment (lines 54-56) explicitly requested a dedicated trailing-newline parity
  test (trailing-newline + no-trailing-newline). Behaviour is covered incidentally; add an
  explicit named test to satisfy the requested coverage.
- 🟢 O3 — No DRY violation vs `views/diff_common/diff_render.rs` (distinct responsibilities).

### RPC-391: Render colored Edit/Write diffs in the Rust agent view — PASS
**Build/test:** full suite GREEN (no regressions); `edit_diff_rendering_rpc391` 8/8 pass; clippy
zero warnings; fmt clean.
**Coverage:** 8/8 scenarios; color assertions inspect REAL ratatui `Span.style.bg`
(`Rgb(139,0,0)` removed / `Rgb(0,100,0)` added, fg White); Bash regression asserts no diff
coloring AND that the RPC-389 8-line collapse still fires; >25-line case asserts inline collapse
vs full modal; malformed-input fallback asserts raw text + no panic.
**Wiring:** capture (`handle_tool_call`) → produce (`handle_tool_result`) → decode (`chunk_wrap`,
`is_diff` bypasses 8-line collapse) → modal (`turn_modal::decode_modal_row`) confirmed
end-to-end and reachable. `pending_tool_diffs` consumed on ToolResult + cleared on reset (no leak).
All touched files < 300 LoC. is_diff flag drives both decode and collapse-bypass (no string-sniffing).

- 🔴 None. 🟡 None.
- 🟢 O1 — Two diff renderers coexist by design (fg-color unified-diff pane vs bg-fill marker decode); correct call.
- 🟢 O2 — `produce_diff_strings` formats twice (collapsed + full); negligible, memoization opportunity.
- 🟢 O3 — `context_gutter_len` hand-rolled regex scanner; correct + panic-safe.

## Fix Plan (Phase 4)
- RPC-390 → implementing: reword scenario title (W1) + align with steps (W2); add explicit
  trailing-newline parity test (O2); update example-map example to match; re-validate; → done.
- RPC-391 → no required fixes (PASS). Observations left as future nice-to-haves.
