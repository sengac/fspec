# RPC-405 Design Spec — Wrap-aware rendering for AgentView MultiLineInput

**Goal:** Feature parity with `src/tui/components/MultiLineInput.tsx` — the input auto-grows vertically as text wraps or newlines are added (up to a cap), shrinking the scrollback above; the head of the text never disappears off-screen; the hardware cursor lands on the correct wrapped cell.

## Architecture decision

Keep `tui_textarea::TextArea` as the **state engine only** (buffer, cursor, editing ops, insert_newline/insert_str already wired via RPC-402/403). Replace the **render layer** entirely — do NOT call `(&self.textarea).render(...)` anymore. tui-textarea's Widget impl is structurally wrap-hostile (1 logical line = 1 visual row, horizontal `Paragraph::scroll`, `pub(crate)` viewport that is unreadable from outside).

New module: `codelet/fspec-tui/src/views/agent/multiline_wrap.rs` (keep every file < 300 LoC; split into `multiline_wrap.rs` (pure geometry) and render code inside `multiline_input.rs` or a sibling if needed).

## 1. Pure wrap geometry (unit-testable, no Buffer)

```rust
/// One visual row: a slice of a logical line.
pub struct VisualRow {
    pub logical_row: usize,   // index into textarea.lines()
    pub char_start: usize,    // char offset of the segment start
    pub text: String,         // the segment (display width <= wrap_width)
}

pub fn wrap_line(line: &str, wrap_width: u16) -> Vec<VisualRow-segments>;
pub fn wrap_lines(lines: &[String], wrap_width: u16) -> Vec<VisualRow>;
```

Width accounting MUST use `unicode_width::UnicodeWidthChar` (`c.width().unwrap_or(0)`), accumulating per char and breaking BEFORE the char that would exceed `wrap_width` (never split a wide char). This ports the loop from tui-textarea `highlight.rs:70-86` (`DisplayTextBuilder::build`). Tabs: the buffer never contains hard tabs today (input is typed/pasted through crossterm; keep tab width = accumulate via `c.width()`, treat `\t` as width via the same fallback — do NOT implement tab stops, out of scope, but must not panic).

- Empty logical line → exactly one empty VisualRow (preserves height, TS parity: empty lines render `' '`).
- A logical line of width exactly `wrap_width` → 1 row; `wrap_width + 1` → 2 rows.
- `wrap_width == 0` → degenerate: return 1 row per logical line, render nothing (guard, no divide-by-zero/underflow).

## 2. Cursor mapping (logical → visual)

```rust
/// (visual_row_index, visual_col) for the logical cursor.
pub fn cursor_visual_position(lines: &[String], cursor: (usize, usize), wrap_width: u16) -> (usize, u16);
```

Compute the display column of the cursor within its logical line = sum of `c.width()` for chars before `cursor.1` (same as tui-textarea textarea.rs:1055-1059), then locate which wrapped segment contains it. Cursor at end-of-line whose width is an exact multiple of `wrap_width`: cursor sits at col 0 of the NEXT visual row of that line (a new row appears — matches how typing continues there).

## 3. Wrap-aware height for the layout

Replace the layout call site (`views/agent.rs:228`):

```rust
pub fn visible_rows_for_width(&self, wrap_width: u16) -> u16 {
    total_visual_rows(lines, wrap_width).clamp(1, self.max_visible_rows) // cap stays 6
}
```

The AgentView knows the body width at layout time: `area.width - 2*pad(1) - 2 (for "> " prompt)`. IMPORTANT: compute the input height from the SAME width the renderer will use, or rows will misalign. Keep the existing `visible_rows()` (logical) only if still referenced by tests; the layout must use the wrap-aware version.

- Empty buffer → 1 (placeholder row).
- The existing layout (`Constraint::Min(0)` scrollback + `Constraint::Length(input_height)`) already yields space to the input — that part works today and needs no change.

## 4. Visual-row viewport with cursor-follow

The widget owns `scroll_top: usize` (first visible VISUAL row). On render, apply tui-textarea's follow algorithm (widget.rs:84-92) in visual-row space:

```rust
fn next_scroll_top(prev_top, cursor_row, viewport_height) -> usize {
    if cursor_row < prev_top { cursor_row }
    else if prev_top + height <= cursor_row { cursor_row + 1 - height }
    else { prev_top }
}
```

Also clamp `scroll_top <= total_rows.saturating_sub(height)` so deleting text scrolls back. `set_value` puts the cursor at End → follow scrolls to the bottom (TS parity: `setValue` scrolls to end). `reset()` → scroll_top = 0.

## 5. Renderer (`render_with_prompt` replacement)

- Row 0 of the input area: green `"> "` prompt (unchanged), body starts at x+2.
- Paint `viewport_height` visual rows starting at `scroll_top`, one `Line` per row, plain style (no horizontal scroll — segments already fit).
- Empty buffer → dim placeholder hint (unchanged).
- NO cursor cell painting in the buffer — the hardware cursor is positioned by the app loop (see §6). (Existing behavior: `set_cursor_line_style(default)` hides tui-textarea's own highlight; keep the hardware-cursor approach.)

## 6. `AgentView::cursor_position()` fix (feeds RPC-404)

Current (`agent.rs:135-145`): `y = area.y + logical_row` — wrong whenever wrap or scroll occurs. New:

```
(vrow, vcol) = cursor_visual_position(lines, cursor, body_width)
x = area.x + 1(pad) + 2(prompt) + vcol
y = area.y + (vrow - scroll_top)     // scroll_top after follow
```

Clamp: if `vrow` outside `[scroll_top, scroll_top+height)` after follow (shouldn't happen), clamp y into the input area. RPC-404 (separate card, depends on this one) covers the acceptance criteria for cursor containment; RPC-405 must expose the mapping API RPC-404 needs.

## 7. What does NOT change

- Enter/submit routing (`multiline_input_enter.rs`), paste (`multiline_input_paste.rs`), gates (RPC-095), Shift+arrow chords, popups, history recall plumbing (`dispatch_history_recall.rs` — it calls `set_value`, which now renders correctly).
- The 6-row cap (`max_visible_rows`) — now counts VISUAL rows (TS caps logical lines at 5 but lets wraps exceed; the ratatui port needs a hard row cap for its explicit layout, cap-by-visual-rows is the correct terminal-space interpretation — record as architecture note).
- `value()`, `set_value()`, `line_count()`, `is_empty()` semantics.

## 8. Acceptance sketch (drive Example Mapping from these)

1. Typing past the right edge grows the input to 2 rows; BOTH the head and tail of the text are visible; scrollback shrinks by one row.
2. A recalled multi-line history entry renders all its lines (up to 6) — nothing appears truncated.
3. An entry wrapping to more than 6 visual rows shows the 6-row window containing the cursor (follow), scrolling as the cursor moves (Up/Down/Home).
4. Empty buffer renders 1 row with the placeholder.
5. Deleting text shrinks the input back down; scrollback regains rows.
6. Wide chars (CJK/emoji) never split across rows; wrap points respect display width.
7. Hardware cursor is on the exact wrapped cell being edited, always inside the input area (RPC-404 asserts containment).
8. The 60x12 repro (`zz_repro_multiline_render.rs` wrap case) now shows `word01` AND `word12`.

## 9. Testing notes

- Pure geometry: unit tests on `wrap_lines` / `cursor_visual_position` (no terminal).
- Integration: TestBackend renders through `AgentView::render_with_store` (pattern in `zz_repro_multiline_render.rs`) asserting row contents + input area height.
- Update `view_agent_multiline_input_rpc019.rs` / `agent_input_multiline_newline_rpc402.rs` expectations ONLY where they encode the buggy logical-only height; do not weaken unrelated assertions.
- Run from `codelet/`: `cargo test -p codelet-fspec-tui`; clippy + fmt must be clean; no unwrap/expect/panic in prod code.
- Release rebuild for manual verification: `cd codelet && cargo build --release -p codelet-cli`.
