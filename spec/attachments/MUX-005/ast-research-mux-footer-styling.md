# AST research — MUX-005 (mux footer bar styling)

Research tooling note: `fspec research --tool=ast` is not yet ported to the
Rust binary; equivalent AST searches were run with the AstGrep tool directly
(patterns below) and ripgrep for cross-file pattern confirmation.

## 1. The footer painter (target of the change)

- `rust/fspec-tui/src/views/multiplex/render.rs:135`
  `fn paint_footer(layout: &MultiplexLayout, area: Rect, buf: &mut Buffer)`
  - Row: `area.y + area.height - 1` (the footer row reserved by `render_with_stores`, lines 57-63: `body` = `area.height - 1`).
  - Currently writes each label char via `buf[(x, row)].set_symbol(...)` with **no style** — inherits terminal defaults.
  - Label: `"MUX {n} panes [Board|Agent|...]"` + `"  ●pane {focus}"` + `"  /mux config · Shift+←/→ focus · drag divider"`; clipped at `area.x + area.width`.
  - No cells past the label are written today (trailing cells keep terminal bg).

## 2. Existing bg-row painting pattern to mirror

- `rust/fspec-tui/src/views/agent/footer.rs:29`
  `pub(crate) const FOOTER_BG: Color = Color::Rgb(0x33, 0x33, 0x33);`
- `rust/fspec-tui/src/views/agent/footer.rs:55` — `paint_row_bg(area, buf, FOOTER_BG);`
- `rust/fspec-tui/src/views/agent/chrome.rs:21` — `pub(crate) fn paint_row_bg(area: Rect, buf: &mut Buffer, color: Color)` paints a full row background across the area width.
- `rust/fspec-tui/src/views/agent/header.rs:41` — same constant pattern (`HEADER_BG`).

Pattern decision: define `MUX_FOOTER_BG: Color = Color::Rgb(74, 44, 112)` in
`views/multiplex/render.rs` (the multiplex module does not currently import
agent `chrome.rs`; inlining the full-row loop inside `paint_footer` keeps the
change confined to one file — `paint_footer` already loops over cells).

## 3. Call sites / integration

- `paint_footer` is called exactly once: `render_with_stores`
  (`render.rs:109`), after `paint_dividers`.
- `render_with_stores` short-circuits (`layout.config.enabled || area.height < 3 || area.width < 2`) before `paint_footer` — mux off ⇒ no footer ⇒ single-view rendering untouched (R10 from rust-mux-mode.feature).
- Existing tests asserting footer presence by text only (`tests/mux001.rs`
  lines ~448, 1056, 1474: "no mux footer may be painted when mux is off",
  "the mux footer row must be painted") check row text, not styles — the
  styling change cannot break them.

## 4. No other footer paint sites in the mux module

Grep for `footer|Color::` in `views/multiplex/` confirms `render.rs`
`paint_footer` is the only footer paint site; `mouse.rs` treats the footer
row as a "gap" for clicks (no rendering there).
