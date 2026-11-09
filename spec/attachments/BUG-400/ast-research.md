# BUG-400 AST research

- `rust/fspec-tui/src/views/board/menu_snapshot.rs` `chip_inputs` (~line 68-80).
- `rust/fspec-tui/src/components/menu_bar/chips.rs` `ChipInput` / `Chip` (`status` field) and the glyph/style painter.
- ChipInput literals in tests: `menu001_menu_bar.rs`, `menu008_menu_bar_reorg.rs`, `menu009_zone_c_buttons.rs`,
  `bug199_agent_bar_zone_c_semantics.rs`.
