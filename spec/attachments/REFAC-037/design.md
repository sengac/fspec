# REFAC-037 design

BUG-350 must fix stale ring assertions in `rust/fspec-tui/tests/menu007_agent_board_view_item.rs` (470 lines, over the 300-line limit). Pure restructure:
- The const and helpers (`AGENT_BAR_ROW`, `fresh_app`, `drain_pending`, `key`, `click`, `render_app`, `row_text`, `find_x`, `board_view_x`, `chip_x`, `zone_c_button_x`, `seed_sessions`, `enter_agent_view`) move to `tests/menu007_support/mod.rs`.
- New helpers: `agent_view_with_sessions(n)` replaces the repeated fresh_app + seed + enter Given, `press(app, code)` replaces each bare key + drain pair, and `assert_focus` replaces the 5-line ring-focus `assert_eq!`. It uses `#[track_caller]` so failures still point at the test line.
- The header doc is condensed and the observation-point notes move to the support module. Every test and `@step` comment is unchanged. Result: 299 lines.
