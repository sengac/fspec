# Epic Review: BLOCK /blocklist view parity fixes

**Date:** 2026-07-05
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 2 (BLOCK-008, BLOCK-009)

## Summary
- 🔴 Critical: 0 across 2 work units
- 🟡 Warnings: 0 work-unit-scoped (1 pre-existing, out-of-scope clippy nit noted below)
- 🟢 Observations: several (all positive / non-blocking)

## Work Unit Results

### BLOCK-008: BlocklistView viewport scrolling — ✅ PASS
- Feature file `spec/features/blocklist-view-scrolling.feature`: OK — G/W/T ordering correct,
  no placeholders, architecture doc string present, @BLOCK-008 tag present.
- Tests `codelet/fspec-tui/src/views/blocklist/tests.rs`: OK — `// Feature:` header;
  all 6 scenarios have `#[test]` with exact `// @step` comments; assertions check real
  state (`selected_index`, `scroll_offset()`, buffer text `Showing`/`of 30`, scrollbar glyphs).
- Impl `mod.rs`/`render.rs`/`panes.rs`: OK — reuses `scroll_viewport::ensure_visible` +
  `diff_common::render_pane_scrollbar` (no hand-rolled math); no unwrap/expect/todo/panic in
  non-test code; production files 236/90/257 lines (<300). Wired end-to-end (navigator.rs:189
  render; navigator_events.rs:152 handle_key).
- Scenario coverage: 6/6.
- 🟡 (optional, non-blocking): `render_left_pane` uses `#[allow(clippy::too_many_arguments)]`
  (7 params) — could be a small ctx struct. Left as-is; acceptable given the module split.

### BLOCK-009: BlocklistView reference-parity framing — ✅ PASS
- Feature file `spec/features/blocklist-view-framing.feature`: OK — G/W/T ordering, no
  placeholders, doc string, @BLOCK-009 tag.
- Tests: OK — 4 scenarios, exact `// @step` matches, `// Feature:` header, real buffer-text
  assertions (count header, divider column run, footer strings, RPC-056 regression guard).
- Impl `render.rs`/`panes.rs`/`mod.rs`: OK — reuses `full_screen_shell::render_full_screen_scaffold`
  + `diff_common::render_vertical_divider` + `render_pane_scrollbar`; old hand-rolled
  `Block::default().borders(ALL).title(" Blocklist ")` GONE; header "Blocklist Rules (N rules)";
  footer "↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close"; `std::mem::take` closure
  idiom correct with no leftover state; files <300 lines; RPC-056 (categories, ○/● glyphs,
  session-disabled, empty-state) and BLOCK-008 (windowed pane, scrollbar, Showing indicator)
  preserved.
- Scenario coverage: 4/4.

## Final Verification
- All tests pass: ✅ (`cargo test -p codelet-fspec-tui` → 2191 tests, 0 failed)
- Build/clippy succeeds: ✅ (clippy clean for all blocklist code)
- Coverage complete: ✅ (BLOCK-008 6/6, BLOCK-009 4/4)
- Feature files valid: ✅
- No regressions to RPC-056: ✅

## Out-of-scope note (not fixed under these cards)
Both reviewers flagged one pre-existing clippy warning in
`codelet/fspec-tui/examples/repro_env_pause.rs:87` ("this loop could be written as a
`while let` loop"). It is unrelated to the /blocklist view and predates BLOCK-008/009.
Recommend a separate cleanup card if the crate should be fully `--all-targets` clippy-clean.
