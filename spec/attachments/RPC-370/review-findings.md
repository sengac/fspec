# Epic Review: RPC-370 — Render markdown tables with box-drawing characters in Rust chat view

**Date:** 2026-06-27
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 1 (file-size headroom)
- 🟢 Observations: 5

## Work Unit Results

### RPC-370: Render markdown tables with box-drawing characters in Rust chat view — PASS

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)
1. **markdown_tables.rs is 295 lines — within 4 lines of the 300-line ceiling.** The file is
   dominated by its `#[cfg(test)] mod tests` block (~253 lines of tests vs ~40 lines of
   production code). It passes the guard today, but any future test addition will breach 300.
   Extract the test module to a sibling file to restore headroom.

#### 🟢 Observations
1. Intentional parity gap (no ANSI/bold; char-count width vs TS `chalk.bold`/`getVisualWidth`)
   is documented in assumptions + architecture notes + feature doc string. Faithful and justified.
2. TS `looksLikeTable`/code-fence path intentionally not ported (no markdown lexer in the Rust
   Done path) — covered by assumption #2. Rust detects contiguous pipe blocks via
   `is_table_row` + `is_separator_row`, a reasonable equivalent.
3. Colon-alignment test uses wider cell content (`aaaa/bbbb/cccc`) so center/right padding is
   actually exercised; `@step` comment still quotes the feature text verbatim. Good test design.
4. `pad_text` early-returns on `pad == 0` and uses `saturating_sub`; over-wide/truncated cells
   never panic.
5. End-to-end wiring confirmed: `handle_done` (chunk_processor.rs:206) → `format_markdown_tables`
   → `source.text` rewrite → `rewrap_at` → scrollback. Single call site, matching arch note [1].

## Coverage Verification
- Feature file: spec/features/markdown-table-box-drawing-rendering-in-rust-chat-view.feature — OK
- Test file: codelet/fspec-tui/src/store/agent_view/markdown_tables.rs — OK (6 tests, verbatim @step)
- Impl files: markdown_tables.rs (entry) + markdown_table_render.rs (renderer) — OK, both < 300 LoC
- Scenario coverage: 6/6 (100%)

## Build & Test Verification
- cargo test -p codelet-fspec-tui — all pass (incl. source_shape_rpc013 300-LoC guard)
- cargo clippy -p codelet-fspec-tui — clean for touched files
- cargo fmt --check — RPC-370 files clean (pre-existing navigator.rs single-line import unrelated)

## Fix Results

### RPC-370
- 🟡 Warning 1 (file-size headroom) → ✅ Fixed: extracted the `#[cfg(test)] mod tests` block from
  markdown_tables.rs into a sibling `markdown_tables_tests.rs` declared via
  `#[cfg(test)] #[path = "markdown_tables_tests.rs"] mod tests;`. Production file dropped to ~46
  lines; tests retain identical `use super::*` privacy/access. Coverage test mappings re-linked to
  the new file. All 6 tests still pass.

## Final Verification
- All tests pass: ✅
- Build succeeds: ✅
- Coverage complete (6/6): ✅
- Feature file valid: ✅
- Tags valid: ✅
