# RPC-013 — AST research findings

## Question 1: Where does `FooterView` currently appear in source?

`ast-grep` query: `FooterView` (identifier) over `codelet/fspec-tui/src/`

Hits:
- `codelet/fspec-tui/src/lib.rs:51` — re-export from `pub use views::{... FooterView ...}`.
- `codelet/fspec-tui/src/views/mod.rs:26` — submodule + re-export.
- `codelet/fspec-tui/src/views/navigator.rs:21` — `use crate::views::{... FooterView}`.
- `codelet/fspec-tui/src/views/navigator.rs:52` — `footer: FooterView::new(theme)` field init.

Plus the source file itself: `codelet/fspec-tui/src/views/footer.rs` (60 LoC).

**Implication for RPC-013:** the deletion touches exactly the four call-sites above. No other crate references `FooterView` (verified via `Grep FooterView codelet/`), and the production binary does not depend on it transitively beyond `Navigator`.

## Question 2: Layout shape of `Navigator::render_with_stores`

`navigator.rs:99-117`:

```rust
let outer = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(1)])
    .split(area);
let main = outer[0];
let footer = outer[1];

match self.active_view {
    ViewMode::Board => self.board.render_with_store(main, buf, board_store),
    ViewMode::Agent => self.agent.render_with_store(main, buf, agent_store),
}
self.footer.render(footer, buf);
```

After RPC-013 this becomes:

```rust
match self.active_view {
    ViewMode::Board => self.board.render_with_store(area, buf, board_store),
    ViewMode::Agent => self.agent.render_with_store(area, buf, agent_store),
}
```

— full area handed to the active child, no footer field.

## Question 3: Existing render_with_store signatures

`ast-grep fn render_with_store(...)` returns no matches at the inherent-method
pattern level (ast-grep needs the inherent-method pattern inside `impl` block,
which is fine — read-grep confirms both views have it):

- `BoardView::render_with_store(&self, area: Rect, buf: &mut Buffer, store: &BoardStore)`
  at `views/board.rs:115`.
- `AgentView::render_with_store(&mut self, area: Rect, buf: &mut Buffer, store: &AgentViewStore)`
  at `views/agent.rs:177` (note `&mut self` because it stores `last_input_area`).

Both will accept a new `theme: &Theme` argument only if needed; in fact, both
views already hold their own `Arc<Theme>` (`BoardView.theme`) or can use a
local default (`AgentView` currently has no theme field — RPC-013 will not
add one; instead it will use plain styled spans matching the current footer
style).

## Question 4: Existing tests that assert on the current footer string

`Grep '? help'` and `Grep 'switch pane'` across `codelet/fspec-tui/`:

- `codelet/fspec-tui/src/views/footer.rs:51-57` — the literal strings live in
  the FooterView render body. Inline tests under `mod tests` in the same file
  do not assert on them (there are no inline tests in `footer.rs`).
- `codelet/fspec-tui/tests/snapshots/*` — no current snapshot pins the footer
  string (only board/agent body snapshots).
- `spec/features/fspec-tui-root-layout-rpc009.feature` — describes the old
  RPC-009 FooterView contract but is now superseded by RPC-012 and will be
  superseded again by RPC-013.

**Conclusion:** removing the file is safe; only RPC-009 feature spec text
references the deleted footer hint, and that is acceptable historical drift
(RPC-012 already broke the strict RPC-009 layout contract).

## Question 5: File-size budget for the three files we will modify

Current LoC (production code only, including `mod tests`):

- `codelet/fspec-tui/src/views/navigator.rs` — 205 LoC → after deletion ~185.
- `codelet/fspec-tui/src/views/board.rs` — 195 LoC → after +footer ~220.
- `codelet/fspec-tui/src/views/agent.rs` — 292 LoC → after +footer ~310 ⚠️.

**Mitigation:** AgentView's inline tests (lines 230–292 = 62 LoC) will be
extracted to a new integration test file
`codelet/fspec-tui/tests/view_agent_unit_rpc013.rs` (mirroring the RPC-012
pattern for `view_board_unit_rpc012.rs`), freeing ~62 LoC. Final agent.rs
projected to ~250 LoC — comfortable under 300.

BoardView's footer scenarios will land in
`codelet/fspec-tui/tests/view_board_unit_rpc013.rs` (a NEW file, not the
RPC-012 one which is closed/done).

## Tooling decision

No new external crates. The footer is a single
`Line::from(vec![Span::styled(...)])` rendered via `Paragraph`. The styling
mirrors the existing FooterView (`theme.dim` + `key_style = fg(theme.fg).bold()`).
