# RPC-404 Design Spec — Hardware cursor containment in the input viewport

**Depends on:** RPC-405 (wrap-aware renderer — provides the logical→visual cursor mapping and the visual-row viewport `scroll_top`).

## Problem

`AgentView::cursor_position()` (`codelet/fspec-tui/src/views/agent.rs:135-145`) computes:

```rust
let (row, col) = self.input.cursor();          // LOGICAL (row, col), char-wise
let x = area.x + 1 + 2 + col;                  // pad + "> " prompt
let y = area.y + row;                          // ← unclamped LOGICAL row
```

Failure modes (all reachable since RPC-402 typed newlines + RPC-403 multi-line paste):

1. **Buffer > cap rows:** logical row 9 with a 6-row input area puts the hardware cursor 3+ rows BELOW the input area — often past the terminal bottom (found in RPC-403 review, Warning 1, `spec/attachments/RPC-403/review-findings.md`).
2. **Wrapped lines (post RPC-405):** the logical row is not the visual row at all — cursor lands on the wrong row even inside short buffers.
3. **Long line horizontal overflow:** `col` is a char index, not a display column; wide chars (CJK/emoji) shift the true cell; pre-RPC-405 horizontal scroll made x wrong too.

## Fix (consumes RPC-405 APIs)

```
(vrow, vcol) = cursor_visual_position(lines, cursor, body_width)   // RPC-405 pure fn
y = area.y + (vrow - scroll_top)                                    // scroll_top after cursor-follow
x = area.x + 1 (pad) + 2 (prompt) + vcol                            // vcol is a DISPLAY column
```

Then clamp defensively so the cursor can NEVER leave the input rect:

- `y` clamped to `[area.y, area.y + area.height - 1]`
- `x` clamped to `[area.x, area.x + area.width - 1]`

`cursor_position()` returns `None` when there is no input area yet (unchanged) and when the transition state hides the cursor (unchanged — gating lives in `is_cursor_visible`).

## Acceptance sketch

1. 10-line buffer, 6-row cap, cursor on last logical line → hardware cursor on the LAST row of the input area (not below the footer/terminal).
2. Cursor moved (Up) to a row above the viewport → follow scrolls; cursor stays on the FIRST input row.
3. Single logical line wrapped to 3 visual rows, cursor mid-line → cursor on the correct wrapped row/column cell.
4. Wide-char line (e.g. CJK) → x accounts for display width (col ≠ char index).
5. For ANY buffer/cursor/area combination, the returned (x, y) lies inside the input rect (property-style assertion over a grid of cases is acceptable).

## Testing notes

- Unit-level: call `cursor_position()` after driving the input + render (render populates `last_input_area` and runs the follow) via TestBackend.
- Integration pattern: `zz_repro_multiline_render.rs` / `agent_input_multiline_newline_rpc402.rs` show how to build AgentView + store + TestBackend.
- `cargo test -p codelet-fspec-tui` from `codelet/`; clippy/fmt clean; no unwrap/expect/panic in prod code.
