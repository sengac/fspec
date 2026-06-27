# Epic Review: RPC-367 — Restore pane border/divider parity in Rust TUI Changed Files and Checkpoints views

**Date:** 2026-06-27
**Reviewer:** Claude Code (fspec review skill) + subordinate ACDD reviewer
**Work Units Reviewed:** 1 (RPC-367 — no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2 (both non-blocking)
- 🟢 Observations: 5 (all confirming correctness)

## Work Unit Results

### RPC-367: Restore pane border/divider parity — PASS

#### 🔴 Critical Issues
None.

#### 🟡 Warnings
1. **Unfocused heading label uses `Color::White` while the underline rule uses `Color::Reset`** — `codelet/fspec-tui/src/views/diff_common/pane.rs:46,66`. The divider and heading-rule glyphs correctly use the default colour (`Color::Reset`) per Rule 4. The heading *label* text uses `Color::White` when unfocused. This is **correct TypeScript parity** — the Ink reference sets `color={focusedPane === '...' ? 'black' : 'white'}` on the heading text. It is a deliberate, pre-existing, cross-view-consistent choice and NOT a divider/border concern. **Resolution: won't fix — matches the reference behavior.**
2. **No regression test for the height-0 / height-1 small-terminal edge** — `pane.rs:28` guards `area.height == 0`, and `render_heading_underline` / `render_vertical_divider` guard zero width/height, so rendering on a 1-row body cannot panic. There was no explicit test pinning this. **Resolution: fix — add a small-terminal regression test for the shared helpers.**

#### 🟢 Observations (confirmed correct)
1. Vertical divider spans only the intended pane height: `changed_files/render.rs:62` paints the full body-height gutter `panes[1]`; `checkpoints/render.rs:71` paints `top[1]`, confined to the top row, so it does not bleed into the bottom full-width Diff pane.
2. `pane_header` is now a single shared definition (`grep "fn pane_header"` → one hit in `diff_common/pane.rs`); the old per-view copies were deleted. DRY satisfied.
3. Cached `last_*_rect` content rects remain correct; the divider occupies its own `Length(1)` layout constraint, so `pane_at` hit-testing and page-step math operate on the reduced content area.
4. The `diff_rows` divider-skip is correctly scoped to the changed-files helper (divider on the same rows as the diff pane); the checkpoints helper needs no skip. Intent-preserving, not test-weakening.
5. RPC-367's new code is cleanly formatted; no egregious misformatting. (`cargo fmt --check` is repo-wide non-compliant across many untouched files — out of scope.)

#### Coverage Verification
- Feature files: OK — both carry `@RPC-367`, `@tui` (component), `@diff-viewer` (feature-group), `@done`; architecture doc strings present; correct Given/When/Then ordering; no placeholders.
- Test files: OK — every scenario has a test with `@step` comments matching the Gherkin step text verbatim; assertions check real glyph presence + `Color::Reset`.
- Impl files: OK — no `unwrap()`/`expect()`/`todo!()`/`unimplemented!()` in production render code; no dead code/unused imports; all files < 300 lines.
- Scenario coverage: 4/4 (3 changed-files + 1 checkpoints), 100%.

#### Build & Test
- `cargo test -p codelet-fspec-tui`: 306 lib tests pass, 0 failed; all integration suites pass.
- `cargo clippy -p codelet-fspec-tui`: clean, no warnings.

## Fix Results

### RPC-367
- 🟡 Warning 1 (heading-label `Color::White` vs rule `Color::Reset`) → ✅ Resolved as **won't-fix**: the unfocused heading label being white is correct TypeScript-reference parity (`color={focused ? 'black' : 'white'}`) and unrelated to divider colour. No change required.
- 🟡 Warning 2 (no small-terminal edge regression test) → ✅ **Fixed**: added two guard tests to `codelet/fspec-tui/src/views/diff_common/api_tests.rs`:
  - `pane_header_on_tiny_areas_does_not_overflow_or_panic` (lines 76–94) — 1-row and 0-row areas return a height-0 content Rect without panic.
  - `render_vertical_divider_is_a_noop_for_zero_size_gutters` (lines 96–117) — zero-height/zero-width gutters write no `│` and do not panic.

## Final Verification
- All tests pass: ✅ (308 lib tests, 0 failed; all integration suites green; 0 FAILED crate-wide)
- Build succeeds: ✅ (`cargo build -p codelet-fspec-tui`)
- Clippy clean: ✅ (no warnings)
- Coverage complete: ✅ (4/4 scenarios, 100% on both feature files)
- Feature files valid: ✅ (`fspec validate`)

