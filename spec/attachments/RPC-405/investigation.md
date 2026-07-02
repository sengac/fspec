# RPC-405 Investigation — MultiLineInput never grows vertically on soft-wrap

**Date:** 2026-07-02
**Author:** Claude Code (supervisor session)
**Status:** Root cause confirmed with deterministic repro

## Symptom (user report)

- The agent input box at the bottom of the chat only ever occupies ONE terminal row while typing, no matter how much text is entered.
- When the text "flows" past the right edge, the input does NOT gain a second visual row; instead the visible text slides left and only the TAIL of the buffer is visible.
- Recalled multi-line history entries *appear* truncated (only last portion visible) — the buffer is intact (Enter submits the full multi-line text), so this is purely a rendering/geometry problem.
- The TS implementation (`src/tui/components/MultiLineInput.tsx`) grows the input box vertically and pushes the message area up (Ink/Yoga flexbox).

## Deterministic repro

`codelet/fspec-tui/tests/zz_repro_multiline_render.rs` (TestBackend 60x12):

- ONE logical line of ~84 chars (no `\n`):
  - `line_count = 1`, `visible_rows = 1` → input row height stays 1
  - Rendered row: `> word05 word06 ... word12` — **head of text is gone** (horizontal scroll), tail visible.
- Control: `"alpha\nbravo\ncharlie"` (real newlines) → `visible_rows = 3`, all three rows render. Growth works ONLY for logical `\n` lines.

## Root cause — two compounding defects

### Defect 1: tui-textarea cannot soft-wrap (upstream, by design)

Verified by reading the clone at `/tmp/tui-textarea` (v0.7):

- `widget.rs:95-104` `text_widget`: each buffer line becomes exactly ONE ratatui `Line` — strict 1 logical line = 1 visual row.
- `widget.rs:163-165`: overflow handled by `Paragraph::scroll((0, top_col))` — horizontal scroll keeps the CURSOR visible, truncating the head. This is exactly the "only the tail shows" symptom.
- No `Paragraph::wrap` / wrap flag anywhere in the crate. Scroll state lives in a `pub(crate)` `Viewport` (`AtomicU64` packing row/col/w/h, widget.rs:22-81) — NOT publicly readable.

### Defect 2: `visible_rows()` counts logical lines, ignores width

`codelet/fspec-tui/src/views/agent/multiline_input.rs:126-129`:

```rust
pub fn visible_rows(&self) -> u16 {
    let n = self.line_count() as u16;      // textarea.lines().len()
    n.clamp(1, self.max_visible_rows)
}
```

`views/agent.rs:228` sizes the input row from this (`Constraint::Length(input_height)` at :243). It takes no width parameter, so wrapped visual rows can never be counted. One long paragraph = height 1 forever.

### Historical note

The RPC-002 port spec (`spec/attachments/RPC-002/10-multilineinput-and-mouse-port-spec.md` §A.8) baked in the same mistake: its `compute_height` counts `textarea.lines().len()` and the feature table listed "Soft-wrap rendering" as a tui-textarea capability — it is not one.

## Why this also explains "Shift+Enter does nothing"

On terminals without the kitty keyboard protocol, Shift+Enter arrives as bare CR (== Enter → submit; `multiline_input_enter.rs` `mods.is_empty()` branch). And even where newline insertion works (Alt+Enter, paste), the missing wrap means overflow-typing never produces a second visual row — so the input *looks* single-line in all cases. Note TS parity: the TS handler submits on `key.return` unconditionally too (MultiLineInput.tsx:141-147); the TS multi-line "feel" comes from soft-wrap + flexbox growth, not from Enter chords.

## Related work units

- RPC-402 (done): Enter routing + kitty enhancement flags.
- RPC-403 (done): bracketed paste routing (multi-line pastes now reach the buffer — and are invisible without this fix).
- RPC-404 (backlog, now depends on RPC-405): hardware cursor escapes the input viewport; `cursor_position()` (agent.rs:135-145) uses logical row with no viewport/scroll compensation. The wrap-aware renderer must produce the visual cursor mapping that RPC-404 consumes.

## TS reference behavior (parity target)

`src/tui/components/MultiLineInput.tsx` + `src/tui/hooks/useMultiLineInput.ts`:

1. Buffer = logical lines (`value.split('\n')`).
2. Renders one `<Text>` per visible logical line — Ink WRAPS long lines; Yoga grows the Box to wrapped content height (`flexShrink={0} minHeight={1}`).
3. Conversation area above has `flexGrow={1} flexBasis={0}` → shrinks as input grows.
4. `maxVisibleLines=5` viewport over LOGICAL lines with scroll-follow (`ensureCursorVisible`); `setValue` scrolls to end.
5. Cursor drawn as inverse cell at (cursorRow, cursorCol); empty lines render `' '` to preserve height; empty buffer renders dim placeholder + inverse-space cursor.
