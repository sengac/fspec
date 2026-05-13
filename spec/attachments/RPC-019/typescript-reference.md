# RPC-019 — Multi-line input + VirtualList scrollback

## TypeScript reference

### MultiLineInput
`src/tui/components/MultiLineInput.tsx` (308 lines)

Features:
- Wraps to multiple lines as the user types past the terminal width.
- Supports `Enter` to insert a newline IF a modifier is held (Shift, Alt,
  or specific terminal escape sequence); plain `Enter` submits.
- Cursor positioning is line-aware (up/down moves between visual lines,
  not characters).
- Pastes preserve newlines (`\n` in clipboard becomes literal newlines).
- Custom compaction logic in
  `src/tui/components/multiline-input-compaction-logic.ts` collapses
  whitespace-only lines for screen efficiency.
- Cursor blink + visual cursor offset accounting for visual width
  (`getVisualWidth` from `src/tui/utils/stringWidth.ts`).

### VirtualList
`src/tui/components/VirtualList.tsx` (681 lines)

The scrollback rendering primitive used throughout the AgentView. Features:
- O(1) render per frame regardless of total scrollback length.
- Maintains a windowed slice (`startIndex..endIndex`) based on scroll
  position.
- Supports stick-to-bottom (default), explicit scroll position, and
  jump-to-top / jump-to-bottom.
- Mouse wheel via SGR mouse protocol (`src/tui/utils/mouseProtocol.ts`).
- Keyboard PageUp/PageDown by viewport height.
- Item heights are computed lazily and cached.

### Input prompt
`src/tui/components/ConversationInputArea.tsx`

Prefixes the input line with `>` and renders the placeholder hint text
`('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)` when
the input is empty.

## Current Rust state

`codelet/fspec-tui/src/views/agent.rs`:
- Input is a **single-line** `tui_input::Input` (line 39).
- Scrollback is a flat `Vec<RenderedChunk>` with manual scroll math
  (line 38, 199-216) — no windowing, redraws all lines every frame.
- `Wrap { trim: false }` on the `Paragraph` widget handles soft-wrap.

For modest scrollback (< 1000 chunks) this is fine; long agent sessions
with 10k+ chunks WILL get slow.

## Target Rust behavior

### Multi-line input widget

New file: `codelet/fspec-tui/src/views/agent/multiline_input.rs`.

Recommended approach: port from `tui-textarea` crate (Rust equivalent of
the TS MultiLineInput). It already supports:
- Multi-line editing.
- Cursor up/down between lines.
- Shift+Enter for newline, Enter to submit (with custom event handling).
- Paste with embedded newlines.

The widget exposes:
```rust
pub struct MultiLineInput {
    inner: tui_textarea::TextArea<'static>,
    visible_rows: u16,  // computed from area.height
}

impl MultiLineInput {
    pub fn handle_event(&mut self, ev: &Event) -> InputEventOutcome { ... }
    pub fn value(&self) -> String { ... }
    pub fn reset(&mut self) { ... }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) { ... }
    pub fn cursor_position(&self) -> Option<(u16, u16)> { ... }
}

pub enum InputEventOutcome {
    Submitted(String),    // plain Enter pressed
    Continued,            // any other key handled internally
    Ignored,              // forwarded to caller (e.g. Shift+↑ for history)
}
```

The Shift+↑/↓/←/→ chords are forwarded to the caller because they trigger
history navigation / session cycling (RPC-021), not input editing.

### VirtualList scrollback widget

New file: `codelet/fspec-tui/src/views/agent/scrollback.rs`.

```rust
pub struct ScrollbackList {
    items: Vec<RenderedChunk>,
    scroll_state: ScrollState,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollState {
    pub offset: usize,       // first visible chunk index
    pub stick_to_bottom: bool,
}

impl ScrollbackList {
    pub fn push(&mut self, chunk: RenderedChunk) { ... }
    pub fn scroll_up(&mut self, lines: usize) { ... }
    pub fn scroll_down(&mut self, lines: usize) { ... }
    pub fn jump_to_top(&mut self) { ... }
    pub fn jump_to_bottom(&mut self) { ... }
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) { ... }
}
```

The render algorithm:
1. Compute `viewport_lines = area.height`.
2. Lazily compute item heights for items in the visible window (cache).
3. Walk from `offset` forward until `viewport_lines` are filled.
4. When `stick_to_bottom`: compute the largest `offset` that still
   fills the viewport with the latest items, render from there.

Mouse SGR scroll handling reuses the algorithm planned for RPC-019b
(or include here if scope allows).

### AgentView integration

Replace `AgentView`'s `input: Input` field with `MultiLineInput`. Replace
`scrollback: Vec<RenderedChunk>` with `ScrollbackList`. The `record_chunk`
method delegates to `ScrollbackList::push`.

The `handle_event` method routes:
- Submitted text → `Action::InputSubmitted(text)`.
- Ignored events (Shift+arrows etc.) → forwarded as new actions
  (`HistoryPrev`, `HistoryNext`, `SessionPrev`, `SessionNext` — wired
  in RPC-021).
- ESC → `Action::BackToBoard` (existing behavior).

### Input area visual treatment

Mirror the TS `ConversationInputArea`:
- Prefix the first visual line with `> `.
- When input is empty, render the hint placeholder in dim style:
  `Type a message... ('Shift+↑/↓' history | 'Shift+←/→' sessions | 'Tab' select turn)`.

## RPC/NAPI boundary

**No new RPC methods required.** This card is pure client-side widget work.
The chunk stream that feeds the scrollback already arrives over the
existing `chunks_rx` subscription.

## Existing TypeScript behavior preserved

- `src/tui/components/MultiLineInput.tsx` — UNCHANGED.
- `src/tui/components/VirtualList.tsx` — UNCHANGED.
- `src/tui/components/ConversationInputArea.tsx` — UNCHANGED.

## Acceptance criteria sketch

- The AgentView input box accepts multi-line input. Plain Enter submits;
  Shift+Enter inserts a newline.
- The input box renders at least 1 line tall and grows up to a configured
  cap (e.g. 6 lines) as the user types.
- Cursor moves between visual lines on Up/Down arrows when the input is
  multi-line.
- Pasted text with embedded newlines becomes a multi-line input.
- The scrollback widget renders 1000+ chunks without dropping frames
  (verified by perf test injecting 10k chunks).
- PageUp/PageDown in the scrollback move by viewport height.
- `stick_to_bottom` snaps back to the latest chunk when a new chunk
  arrives during sticky mode.
- When the input is empty, the placeholder hint is visible in dim
  style with the `>` prefix.
- ESC still emits `Action::BackToBoard`.
