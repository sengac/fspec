# 04 — OpenAI Codex CLI ratatui Frontend Deep Dive

> Source: parallel investigation by agent `815852ae-744c-4e25-8e3a-6ad6b97cf0e0`,
> using the `DeepSearch` tool against a clone of `openai/codex` at pinned
> commit `f7e8ff8e5026f92fc4b0be1478bf98f7ffcdd781`.

> **TL;DR:** Codex's TUI uses a fundamentally different rendering model
> from a typical multi-pane ratatui app. It does **not** virtualize chat
> in ratatui; finalised cells are written to the host terminal's native
> scrollback via ANSI escape sequences. ratatui only paints the bottom
> "live" region. **This means Codex is NOT the right reference for
> fspec's port** - fspec is a multi-pane Kanban / agent / file-search
> application that genuinely needs alt-screen + virtualised lists. We
> should follow Helix and lazygit instead. The detail below is preserved
> nonetheless because Codex *is* the canonical AI-agent-CLI implementation
> in Rust ratatui and several of its smaller patterns transfer.

---

## Repository

- **Repo:** https://github.com/openai/codex
- **TUI crate:** `codex-rs/tui/`
- **Pinned commit:** `f7e8ff8e5026f92fc4b0be1478bf98f7ffcdd781`

---

## 1. Scrollable / Virtualised Chat History

### Key Finding: There is NO virtualization of chat history in ratatui.

Codex runs ratatui in **inline viewport mode** (not alt-screen by
default). Finalised chat cells are written **directly into the
terminal's own scrollback** using ANSI escape sequences (DECSTBM = set
top/bottom margin + Reverse Index `ESC M`). The ratatui draw region only
contains:

1. The in-flight `active_cell` (mutating, currently streaming).
2. `active_hook_cell` (optional).
3. The `bottom_pane` (composer + status + popups).

### Files & Symbols

| Concern | File | Symbols / Lines |
|---|---|---|
| Scrollback writer | [`codex-rs/tui/src/insert_history.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/insert_history.rs) | `insert_history_lines_with_mode_and_wrap_policy` (L103), `enum InsertHistoryMode { Standard, Zellij }` (L44), `enum HistoryLineWrapPolicy { PreWrap, Terminal }` |
| Chat composition | [`codex-rs/tui/src/chatwidget.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/chatwidget.rs) | `ChatWidget::as_renderable` (~L10943), `flush_active_cell` (~L5530), `add_to_history` (L5537-5556) |
| In-memory cell store (for resize-reflow + transcript overlay) | [`codex-rs/tui/src/app.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/app.rs) | `App.transcript_cells: Vec<Arc<dyn HistoryCell>>` |
| HistoryCell trait | [`codex-rs/tui/src/history_cell.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/history_cell.rs) | `trait HistoryCell` (L156), 30+ concrete cell types |
| Event dispatch | [`codex-rs/tui/src/app/event_dispatch.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/app/event_dispatch.rs) | `AppEvent::InsertHistoryCell` arm (~L196-216) |

### The ONLY virtualised scrolling view: Transcript Pager (Ctrl+T)

When the user opens the full-history pager, Codex enters alt-screen and
renders a custom virtualizer in
[`codex-rs/tui/src/pager_overlay.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/pager_overlay.rs):

- `enum Overlay { Transcript(TranscriptOverlay), Static(StaticOverlay) }`
  (L50)
- `struct PagerView { renderables: Vec<Box<dyn Renderable>>, scroll_offset: usize, ... }`
  (L117)
- `render_content` (L182) - iterates renderables with running `y`
  cursor, **skip above / break below**.
- `render_offset_content` (L885) - for cells partially clipped at top,
  renders into oversized `Buffer` then copies visible slice (the actual
  virtualization trick).
- vi-style key bindings: `scroll_up` / `down`, `page_up` / `down`,
  `half_page_up` / `down`, `jump_top` / `bottom`.

**`ratatui::widgets::List` and `ListState` are NOT used anywhere.**
**`Paragraph::scroll(...)` appears exactly ONCE** - in
`history_cell.rs#L257`, used to top-clip an oversized active cell.

### Transferable patterns for fspec

- The `Renderable` trait with `desired_height(width)` is **Flutter-flex
  inspired**. fspec doesn't need this complexity - our items are
  generally flat lines or paragraphs that fit ratatui's `Constraint`
  model fine.
- The "render into oversized buffer then crop" trick is useful when an
  item exceeds the viewport. We may need it for `CheckpointViewer` diffs.
- The `Vec<Box<dyn Renderable>>` over `Vec<Box<dyn HistoryCell>>` is the
  same pattern as Helix's Compositor and our `InputHandlerRegistry` -
  trait objects in a Vec, rendered in order. **Confirmed pattern.**

---

## 2. Modal Popups / Dialogs

### Key Finding: A `Vec<Box<dyn BottomPaneView>>` view-stack inside `BottomPane`. NO `Clear` widget, NO centered-Rect helper, NO popup crate.

The active modal **replaces** the chat composer in the same `Rect`,
painted with a styled `Block` background via `render_menu_surface`.

### Two layers of "popup"

| Layer | Mechanism | Used for |
|---|---|---|
| **Modal views** | `view_stack: Vec<Box<dyn BottomPaneView>>` | Approval prompts, model picker, tool-approval, MCP elicitation, settings views |
| **Composer-inline** | `enum ActivePopup { None, Command(_), File(_), Skill(_) }` inside `ChatComposer` | `/`-palette, `@file` autocomplete, `$skill` mentions |

### Files & Symbols

| Concern | File | Symbols |
|---|---|---|
| BottomPane (view-stack owner) | [`bottom_pane/mod.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/bottom_pane/mod.rs) | `struct BottomPane` (~L199), `view_stack: Vec<Box<dyn BottomPaneView>>` |
| BottomPaneView trait | Same file | `trait BottomPaneView`: `desired_height(width)`, `render(area, buf)`, `handle_key(event)`, `is_dismissable()` |
| ChatComposer popups | [`bottom_pane/chat_composer.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/bottom_pane/chat_composer.rs) | `enum ActivePopup`, slash command palette, file palette, skill palette |
| Approval prompt | [`bottom_pane/approval_view.rs`](https://github.com/openai/codex/blob/main/codex-rs/tui/src/bottom_pane/approval_view.rs) | Yes / Yes-always / No buttons with focus index |

### Transferable patterns for fspec

- **The view-stack pattern (`Vec<Box<dyn ViewLike>>`) is the same as our
  Compositor recommendation.** Bottom of the stack = main app, top of
  the stack = the active modal. Events propagate top-down.
- Codex doesn't use `Clear` widget - it draws a filled `Block` over the
  bottom region. fspec's modals are full-screen-centered, so we *do*
  want `Clear` + `tui-popup`. Different design.
- "Composer-inline popup" pattern (`/` palette, `@file`) is identical to
  fspec's `SlashCommandPalette` + `FileSearchPopup`. Same design.

---

## 3. Input Handling

Codex routes input through the `BottomPane` first (top of view-stack
gets first crack), then to the `ChatComposer` if no modal is active,
then to global key handlers.

- **No explicit priority enum.** The order is implicit in the recursion
  order through `BottomPane.handle_key -> view_stack.last_mut() ->
  composer.handle_key -> global`.
- Each view's `handle_key` returns an `enum CompositionResult` of:
  `Consumed`, `Dismiss`, `Submit(value)`, `Bubble`.
- `Bubble` propagates up to the next handler.

### Transferable patterns for fspec

- **The `Consumed` / `Bubble` enum is exactly Helix's `EventResult`** and
  exactly what we want for the Compositor. Confirmed pattern.
- Codex doesn't have our 5-level priority enum. Their stack has 3
  implicit levels (modal -> composer -> global). fspec needs more
  granularity (CRITICAL, HIGH, MEDIUM, LOW, BACKGROUND), so we add a
  numeric priority field to each Compositor entry.

---

## 4. Mouse Support

**Codex has NO mouse support in the main UI.** Wheel scrolling is
delegated to the host terminal via DECSET 1007 (Alternate Scroll Mode)
**only in alt-screen overlays** (transcript pager, static overlay).

Search of the repo: `EnableMouseCapture`, `MouseEventKind`, `SGR`, all
absent from the main render path.

### Transferable patterns for fspec

- **None.** fspec must implement mouse properly because we have a
  Kanban board with column-to-column hit-testing, click-to-select, etc.
  Codex's "let the terminal handle it" approach doesn't work for us.

---

## 5. Layout

Codex re-invents its own Flutter-inspired layout system instead of using
ratatui's `Layout` / `Constraint`:

```rust
trait Renderable {
    fn desired_height(&self, width: u16) -> u16;
    fn render(&self, area: Rect, buf: &mut Buffer);
}
```

Then `ChatWidget::as_renderable()` returns a tree of `Renderable`s,
each computing its own desired height from a parent-supplied width. This
is needed because chat cells flow top-to-bottom in scrollback and need
to know their height before insert.

### Transferable patterns for fspec

- **Don't copy this.** fspec's panes are statically constrainable
  (board has 5 columns, each with `Constraint::Fill(1)`; agent view has
  header + virtual list + composer split with
  `Constraint::Length(n) | Min(0) | Length(m)`). ratatui's built-in
  layout is fine.
- The one place fspec might need `desired_height(width)` is for agent
  conversation cells with mixed text + tool-call cards of variable
  height inside a virtualised list. `tui-widget-list` already supports
  this via `ListableWidget::size_hint`.

---

## 6. Scrollbars

Codex uses ratatui's built-in `Scrollbar` widget in the transcript pager
only:

```rust
frame.render_stateful_widget(
    Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .symbols(scrollbar::VERTICAL),
    area,
    &mut state,
);
```

No draggable thumb, no mouse interaction. Keyboard-only.

### Transferable patterns for fspec

- We use the same widget but add mouse-drag thumb hit-testing
  (~30 LoC).

---

## 7. Architecture: Actor Model with Tokio Channels

Codex is **NOT Elm/TEA**. It uses an actor model:

```text
       +-------------------+    AppEvent
   ----| async event loop  |<---- (tokio mpsc)
   |   +-------------------+
   |       |   ^
   |       v   |
   |   +---------------+    Op
   |   | core (codex)  |<------ (tokio mpsc)
   |   +---------------+
   |       |   ^
   |       v   |
   |   +-----------+
   ---->|  TUI app  |
        +-----------+
```

- `App` owns a `tokio::sync::mpsc::Sender<AppEvent>`.
- `AppEvent` enum: `KeyEvent`, `MouseEvent`, `Resize`,
  `InsertHistoryCell`, `StreamingChunk`, `Op`, etc.
- The app's main loop is a `tokio::select!` over: terminal events, app
  events from the channel, render tick, cancellation.
- Each modal view is a stateful object (`Box<dyn BottomPaneView>`) that
  owns its data and handles its own key events.

### Transferable patterns for fspec

- **This is the architecture we want.** It's identical to the
  ratatui-org `templates/component` template and to lazygit's design.
- `tokio::select!` over `(events, ticks, render_signal, cancel_token)`
  is the canonical Rust event loop.
- `mpsc<Action>` bus replaces React's state-update + reconciliation.

---

## 8. What fspec should NOT copy from Codex

1. **Inline / scrollback rendering mode.** fspec is multi-pane; we need
   alt-screen.
2. **The `Renderable` trait with `desired_height(width)`.** ratatui's
   `Constraint` model is sufficient.
3. **Lack of mouse support.** We need it.
4. **Implicit 3-level event recursion.** We need 5-level priority.
5. **Custom DECSTBM scrollback handoff.** We render everything in our
   alt-screen frame.

## 9. What fspec SHOULD copy from Codex

1. **`Vec<Box<dyn ViewLike>>` view-stack pattern** for modals.
2. **`enum ResultEnum { Consumed, Bubble, Dismiss, Submit(_) }`** as the
   return type of every event handler.
3. **`tokio::select!` event loop** with `(events, ticks, render, cancel)`.
4. **`AppEvent` mpsc bus** as the Redux-action equivalent.
5. **Composer-inline popup pattern** (`/` palette, `@file`,
   `$skill`) - identical to fspec's SlashCommandPalette /
   FileSearchPopup. Direct port.
6. **`HistoryCell` trait** as a model for fspec's "any item that can
   render itself in a list" - useful for `AgentView`.
