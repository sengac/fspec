# Epic Review: RPC-381 — Port AgentView Tab turn-selection (SELECT) mode to Rust

**Date:** 2026-06-28
**Reviewer:** Claude Code (fspec review skill) + subordinate reviewer agent
**Work Units Reviewed:** 1 (standalone story, no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 3
- 🟢 Observations: 3
- Overall status: **PASS** (warnings to be addressed)

## Work Unit Results

### RPC-381: Port AgentView Tab turn-selection (SELECT) mode to Rust — PASS

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)
1. **Coverage line-range drift for the "Pressing Tab again exits" scenario.** The
   exit-via-Tab scenario is linked only to `app/dispatch_scroll.rs:27-36`
   (`handle_toggle_turn_select_mode`, the scrollback side-effect). The toggle the test
   actually asserts (second Tab flipping `AgentView.turn_select_mode` off) is driven by
   `views/agent/dispatch.rs:187-191`, which was not linked for that scenario. → Re-link to
   include `dispatch.rs:187-191`.
2. **`generate_arrow_bar` spacing-pattern parity is under-tested.** Behaviour is correct
   (`▼   ▼   ▼` every 4 cols) but the only assertions are `.contains('▼')` / `.contains('▲')`.
   Design doc §5 explicitly called for a unit test pinning the exact glyph/space pattern. →
   Add a `generate_arrow_bar` pattern-parity unit test.
3. **Stale breadcrumb placeholder contradicts shipped behaviour.**
   `tests/behaviour_parity_rpc065.rs:625-646` still carries
   `#[ignore = "Tab turn-selection mode pending future RPC card …"]` with a `panic!`
   placeholder, and references a store API (`agent_view_store().turn_selection_mode()`) that
   never materialised (the flag lives on `AgentView`). Tab turn-selection has now landed
   (RPC-381). → Update/remove the misleading placeholder.

#### 🟢 Observations (Nice to Have)
1. `navigate_turn` re-sums preceding chunk line counts each call
   (`scrollback_select.rs:166-169`) — bounded by `chunks.len()`, not a perf concern; could
   share a `chunk_row_span(idx)` helper with the paint walk for DRY.
2. Two independent row-layout walks exist (`scroll_selected_into_view` logical-row math vs
   `paint_selection_arrow_bars` screen-row math) — correctly separated concerns, not true
   duplication.
3. Full TS nav parity (PageUp/PageDown/Home/End group nav, design §2.5) is intentionally
   out of scope for RPC-381 (rules cover Up/Down only). Candidate for a follow-up card.

#### Coverage Verification
- Feature file: `spec/features/agentview-turn-select-mode.feature` — OK (G/W/T ordering,
  no placeholders, architecture doc-string present, `@RPC-381` + component/feature-group
  tags present, `@done`).
- Test files: `codelet/fspec-tui/tests/turn_select_mode_rpc381.rs`,
  `codelet/fspec-tui/src/views/agent/scrollback_tests.rs` — OK (all @step comments match
  feature step text exactly; real-behaviour assertions; no trivial asserts).
- Impl files: `dispatch.rs`, `dispatch_select.rs`, `scrollback.rs`, `scrollback_select.rs`,
  `scrollback_arrows.rs`, `scrollback_paint.rs`, `agent.rs`, `chrome_paint.rs`,
  `components/mod.rs`, `app/dispatch.rs`, `app/dispatch_scroll.rs` — OK (end-to-end wired;
  no unwrap/panic/todo; all files < 300 lines; clippy-clean except pre-existing tui093).
- Scenario coverage: 9/9 covered.

## Fix Results

### RPC-381 — all 3 warnings resolved
- 🟡 W1 Coverage drift → ✅ Fixed: re-linked "Pressing Tab again exits turn-selection mode"
  to additionally include `views/agent/dispatch.rs:187-191` (the view-flag toggle the test
  actually asserts). `audit-coverage` now 22/22 valid.
- 🟡 W2 Arrow-bar parity under-tested → ✅ Fixed: added char-by-char unit tests in
  `views/agent/scrollback_arrows.rs:124-170`
  (`top_bar_places_glyph_every_spacing_columns_char_by_char`,
  `bottom_bar_uses_up_glyph_at_same_columns`, `boundary_widths_lock_edge_behaviour`) —
  assert glyph at cols 0,4,8 + spaces elsewhere + length + edge widths. All pass.
- 🟡 W3 Stale placeholder → ✅ Fixed: removed the `#[ignore]`/`panic!` placeholder in
  `tests/behaviour_parity_rpc065.rs` (812→793 lines); banner updated to point to
  `tests/turn_select_mode_rpc381.rs`; suite now 28 passed / 0 ignored.

🟢 Observations 1-3 left as-is (intentional scoping / acceptable separation of concerns).

## Final Verification (post-fix)
- `cargo test -p codelet-fspec-tui`: all suites pass (lib 338/338, behaviour_parity_rpc065
  28/28, turn_select_mode_rpc381 11/11). No failures.
- `cargo clippy -p codelet-fspec-tui --lib --tests`: only pre-existing tui093 warnings.
- `cargo fmt -p codelet-fspec-tui -- --check`: clean.
- `audit-coverage agentview-turn-select-mode`: 22/22 valid; 9/9 scenarios covered.

## Build & Test (verified by supervisor)
- `cargo build -p codelet-fspec-tui`: OK
- `cargo test -p codelet-fspec-tui`: all suites pass (new suite 11/11, lib 335/335)
- `cargo clippy -p codelet-fspec-tui --lib --tests`: only pre-existing
  `tui093_default_thinking_restore_parity.rs` MutexGuard warnings (out of scope)
- `cargo fmt -p codelet-fspec-tui -- --check`: clean
