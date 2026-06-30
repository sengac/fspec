# Review: RPC-389 — Tool-result body collapse + streaming window

**Date:** 2026-06-29
**Reviewer:** Claude Code review-skill (subordinate reviewer e308d5f0)
**Status:** PASS

## 🔴 Critical Issues
None.

## 🟡 Warnings (Should Fix)
None.

## 🟢 Observations
1. `collapse_tool_body` is exercised only indirectly through the render path
   (correct ACDD choice). The integration tests cover 5/20/25-line bodies but not
   the exact `== COLLAPSED_LINES` (8) or `== STREAMING_WINDOW_SIZE` (10)
   boundaries. Code is provably correct (strict `>` at chunk_wrap.rs:178, :184);
   adding two boundary tests is preventive hardening. → Add them.
2. Indicator dim styling (`Modifier::DIM`) matches the existing convention.
3. Diagnosis doc is thorough; good template.

## Edge cases (all verified correct)
- 8 settled → no indicator; 10 streaming → no cut; no-body → header only (no panic);
  `+N` equals hidden count (12, 17); is_streaming transition proven by finished-stream test.

## Cross-cutting (no regressions)
- Only one `is_streaming` read for collapse (chunk_wrap.rs:133); in-flight assistant
  placeholder untouched; `stick_to_bottom`/streaming-suffix unaffected.
- INVARIANT: `full_text_for_seq` returns full `src.text` unmodified; modal shows all lines
  (asserted by `full_body_preserved_for_modal`).

## Build & Test
- New test 5/5; broad `cargo test -p codelet-fspec-tui` 1926/0; clippy clean in RPC-389 files; chunk_wrap.rs 190 lines.

## Coverage
- Feature file: OK (clean capability name, arch doc string, @RPC-389).
- Test file: OK (header references feature, @step char-for-char, real render path).
- Impl: OK. Scenario coverage: 5/5.

## Fix Results
- 🟢 Obs 1 (boundary tests) → ✅ Fixed: added exact-8 (settled) and exact-10 (streaming) boundary tests.
