# AST Research — MENU-001 (MenuBar component: 2-zone bar + anchored dropdown)

Date: 2026-09-30 · Scope: `rust/fspec-tui/src/components`,
`rust/fspec-tui/src/views/board`, `rust/fspec-tui/src/theme.rs`,
`rust/fspec-tui/src/lib.rs`

## 1. The menu_bar module (new, this work unit)

Files (all under the 300-LoC ceiling except where noted):

- `menu_bar/mod.rs` (242 LoC) — `MenuSnapshot` / `MenuFocus` /
  `ZoneBCell`, ring math (`advance`, `dropdown_cursor`, `chip_for_cell`)
- `menu_bar/items.rs` (249) — the `MenuCategories` registry
  (`CATEGORIES`, `ACTIONS`, `HELP`, `MenuAction::to_action`)
- `menu_bar/chips.rs` (191) — `build_chips` / `index_prefix` (R4 glyphs,
  R9 Cleared drop)
- `menu_bar/layout.rs` (361 — 61 over the 300 ceiling; 80% is
  `#[cfg(test)]`) — `menu_bar_layout` + the R6 truncation ladder
- `menu_bar/paint.rs` (339 — 39 over; test module ~50%) —
  `paint_menu_bar` (stateless, R1 #333333 bg + 1-cell pad)
- `menu_bar/dropdown.rs` (87) — `dropdown_rect` / `row_content_width` /
  `visible_entry_rows` / `scroll_window` (R7 geometry)
- `menu_bar/dropdown_paint.rs` (~300) — `render_menu_dropdown`
- `menu_bar/dropdown_text.rs` (~90) — `key` / `truncate` helpers

Wiring: `components/mod.rs` gains `pub mod menu_bar;` (+1 line).

## 2. Consumed surfaces (the seams this unit reuses)

- `crate::theme::Theme` (`theme.rs:26`) — `fg`, `dim`, `border_focused`
  read by every menu painter (rule [10]/[16]: one shared palette).
- `crate::components::Action` — `MenuAction::to_action` maps onto the
  EXISTING bus variants (OpenAgentView, OpenWorkUnitSearch,
  OpenCheckpointsView, OpenChangedFilesView, OpenFoundation,
  OpenMuxConfigDialog, OpenAttachmentPicker, OpenBoardHelp,
  OpenBoardExitConfirmation). No new Action variants in MENU-001.
- `crate::components::spinner::{current_frame_glyph, DOTS_FRAMES,
  DOTS_INTERVAL_MS}` (`spinner.rs:36-48`) — the Running chip's braille
  frame (80 ms cadence) per R4.
- `ratatui::buffer::Buffer` + `ratatui::backend::TestBackend` — every
  painter is a pure `(area, &Buffer, snapshot, &Theme)` fn; tests assert
  on buffer cells (symbols, fg, bg, modifier).
- `unicode_width` (workspace dep) — display-width math for the R6
  truncation ladder + the 4-cell key column.
- `proptest` (dev-dep, already used by board filter_work_units tests) —
  pins the "painted row never exceeds area width" invariant.

## 3. The board_shortcuts.rs re-export shim

`views/board/board_shortcuts.rs` keeps the BOARD-023 surface
(`BoardShortcut` / `BoardShortcutAction` / `SHORTCUTS` / `CHORD_HINT`)
so `BoardKeybindingDialog` + `views/board/keys.rs` keep compiling
unchanged; the TUI-106 spinner-shim precedent. A new
`shortcuts_mirror_the_menu_registry` test pins the six bare board keys
against the `CATEGORIES` registry (labels + descriptions cannot drift).
Full dialog migration lands with MENU-005.

## 4. Test shape (pre-existing, verified green — 41 tests)

Inline `#[cfg(test)]` modules per file, TestBackend-based:
registry order/counts, per-status glyphs + colors, #n prefix, Cleared
drop, WU suffix capture, ring math (items → Zone B → wrap, mux display
order), dropdown cursor clamp, chip_for_cell, full-width row paint, WU
at level 0, tight-width ladder steps (level 1 WU drop → level 2 fold →
level 4 Zone A only), zero-area no-op, dark-bg on every cell,
empty-chips separator omission, focused-item/chip inverse video,
open-dropdown item activation, dropdown anchor/clamp/bottom-clip,
key-column + dim description rows, cursor row inverse across the inner
width, cursor auto-scroll in a clipped panel, ellipsis truncation,
proptest width bound. All 41 pass; `cargo test -p codelet-fspec-tui
--lib menu_bar` green at 2026-09-30.
