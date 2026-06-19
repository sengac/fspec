# RPC-339 AST Research — SearchHistoryView shell refit

AST-based analysis of the code touched by RPC-339. Paths are under
`codelet/fspec-tui/`.

## 1. The shell — `src/views/full_screen_shell.rs` (295 LoC, not LoC-pinned)

AstGrep `pub(crate) fn render_full_screen_scaffold(...)` — present
(lines 32–62). Signature:

```rust
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_full_screen_scaffold<F>(
    area: Rect, buf: &mut Buffer,
    title: &str, count: usize, suffix: &str,
    footer_hint: &str, body_fn: F, overlay: Option<&ConfirmDialog>,
) where F: FnOnce(Rect, &mut Buffer)
```

Body: `Clear.render(area, buf)` → `Layout` vertical
`[Length(1), Length(1), Min(0), Length(1)]` → `(title_area, body_area,
footer_area) = (split[0], split[2], split[3])` →
`render_title_with_count(title_area, buf, title, count, suffix)` →
`body_fn(body_area, buf)` → `render_footer_hint(footer_area, buf,
footer_hint)` → `if let Some(dialog) = overlay { dialog.render(area,
buf) }`.

**Precedent for a custom title:** `render_full_screen_scaffold_raw_title`
(lines 69–101) — same Clear/split/overlay shape but paints a verbatim
title string with a Paragraph. The new `render_full_screen_scaffold_with_title`
follows this exact precedent but takes a `title_fn: T where T: FnOnce(Rect,
&mut Buffer)` instead of a `&str`.

`render_title_with_count` is imported from
`crate::views::agent::mode_view_render` (line 18) — it is NOT defined in
the shell. `CHROME_ROWS: u16 = 3` const at line 22.

## 2. SearchHistoryView — `src/views/agent/search_history_view.rs` (264 LoC)

AstGrep `pub fn render(&self, area: Rect, buf: &mut Buffer) { $$$BODY }`
→ match at line 243. Current body (lines 243–257) hand-rolls the
identical scaffold:

```rust
pub fn render(&self, area: Rect, buf: &mut Buffer) {
    Clear.render(area, buf);
    let split = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Length(1), Length(1), Min(0), Length(1)])
        .split(area);
    render_title(self, split[0], buf);
    render_body(self, split[2], buf);
    render_footer(self, split[3], buf);
}
```

- `&self` (immutable) → closures can capture `&self` with plain FnOnce.
- Local `CHROME_ROWS` const + `visible_rows_for` helper (lines 261–263).
- Local render helpers in `search_history_view_render.rs` (179 LoC):
  - `render_title` (lines 28–36): `(search): ` + `view.query()` +
    `Span::styled(" ", REVERSED)` cursor cell. **Editable query title —
    must NOT be flattened to count form.**
  - `render_footer` (lines 39–41): static `"Enter Select | ↑↓ Navigate |
    Esc Cancel"` — droppable in favour of shell `render_footer_hint`.
  - `render_body` (lines 45+): placeholder `(type to search history)` /
    scroll-windowed list with REVERSED selected row + BOLD query
    highlight via `highlight_query`.

## 3. Source-shape gate — `tests/rpc026_source_shape.rs` (HARD BLOCKER)

Lines 84–94: locates SearchHistory's `render` fn and asserts the first
statement `s_trimmed.starts_with("Clear.render(area, buf)")` with **no
delegate alternative**. The resume check (lines 70–82) was already
relaxed to also accept `crate::views::full_screen_shell::render_full_screen_scaffold`.

Refit makes the first statement the shell delegate, so this assertion
MUST be relaxed to mirror resume (accept the shell delegate too), and
the lines 63–69 "deferred to RPC-339" comment must be revised. Also
enforces `search_history_view.rs < 300` (currently 264) and no
`tui_popup`/`popup_body`.

## 4. Snapshots

No insta `.snap` files cover SearchHistory. Validation is buffer-walking
in `tests/search_view_rpc064.rs` (asserts BOLD/REVERSED cells +
placeholder). New/updated tests assert title row `(search):` + REVERSED
cell after the refit, plus body/footer parity.

## Plan summary
1. Add `render_full_screen_scaffold_with_title<T, B>` to the shell.
2. Re-express `render_full_screen_scaffold` on top of it (DRY).
3. Refit `SearchHistoryView::render` to delegate (title_fn = render_title,
   body_fn = render_body, footer hint static, overlay None).
4. Relax the `rpc026_source_shape.rs` first-statement assertion + comment.
