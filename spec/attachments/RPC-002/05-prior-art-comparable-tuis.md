# 05 — Prior Art: Comparable Ratatui (and adjacent) TUIs

> Source: parallel investigation by agent `71796099-baf8-4dbe-9ffb-867a101403b3`,
> using the `DeepSearch` tool. Three of five sub-searches succeeded with
> rich detail; two timed out. Where the data is unconfirmed it is marked
> **NOT INVESTIGATED** and would benefit from a follow-up pass.

This document surveys ratatui-based AI / agent / chat CLIs and complex
stateful TUI apps for transferable patterns relevant to: virtualized
scrollable lists, modal popups, layered input handling, mouse support,
and real-time chat-style UIs.

---

## Summary Table

| Project | Virt-list approach | Popup / Modal approach | Input mgmt | Mouse | Link |
|---|---|---|---|---|---|
| **tenere** | Single `Paragraph::scroll((y, 0))` over a cloned `Text`; manual height calc; `AtomicBool` stick-to-bottom flag | Centered `Rect` via `Layout::vertical -> horizontal` + `Clear` widget; single `FocusedBlock` enum gates rendering | Custom Vim modes (Normal / Insert / Visual) on top of `tui-textarea` 0.7; multi-key seqs via `previous_key` field | `EnableMouseCapture` enabled | https://github.com/pythops/tenere |
| **oatmeal** | Custom `BubbleList` widget; per-message `BubbleCacheEntry { codeblocks_count, text_len, lines }`; manual word-wrap; `buf.set_line` direct buffer writes; ratatui `Scrollbar` | No popups; "command palette" = slash-commands typed into textarea; `/help` rendered as a chat bubble | Vanilla `tui-textarea` 0.4 (Emacs); raw events translated to domain `Event` enum *before* UI sees them | Not emphasized; bracketed paste supported | https://github.com/dustinblackman/oatmeal |
| **aichat** | **Not ratatui** - inline REPL on `reedline` writing to stdout; "head/tail" partition for streaming markdown re-render | None (REPL); `inquire` for selects; reedline `ColumnarMenu` for completion | reedline (vi or emacs), custom `Completer` / `Highlighter` / `Validator` | Terminal-native | https://github.com/sigoden/aichat |
| **gptui** | Effectively dead (Apr 2023) | - | reedline + syntect | - | https://github.com/xiuxiu62/gptui |
| **Helix** | N/A (text editor) | `Compositor` with `Vec<Box<dyn Component>>` + `EventResult::{Consumed, Ignored}` bubbling; callbacks for self-dismissal; `last_picker` field for re-open | `KeyTrie` modal keymap; `KeymapResult::Pending` drives which-key popup; `sticky` nodes; `merge_keys` for user overrides | Per-component hit testing | https://github.com/helix-editor/helix |
| **ratatui templates/component** | N/A | N/A | `Component` trait with `handle_events` / `update` / `draw`; `mpsc<Action>` bus; tokio `select!` over events / tick / render / cancel | Available via crossterm | https://github.com/ratatui/templates/tree/main/component |
| **lazygit** (gocui, comparison only) | Custom `ListRenderer` | Context system: `Kind ∈ {SIDE, MAIN, TEMPORARY_POPUP, PERSISTENT_POPUP, EXTRAS}`; `ParentContextMgr` for focus restore; `PopupHandler` facade (Confirm / Alert / Prompt / Menu / Toast) | Context-scoped keymaps | gocui-native | https://github.com/jesseduffield/lazygit |
| **gitui** | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | https://github.com/gitui-org/gitui |
| **atuin** | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | https://github.com/atuinsh/atuin |
| **bottom** | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | https://github.com/ClementTsang/bottom |
| **yazi** | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | https://github.com/sxyazi/yazi |
| **jjui** | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | https://github.com/idursun/jjui |
| **television** | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | NOT INVESTIGATED | https://github.com/alexpasmantier/television |

---

## Detailed callouts

### tenere ([github.com/pythops/tenere](https://github.com/pythops/tenere))

A ChatGPT-like terminal UI in Rust + ratatui.

- **Chat history rendering:** Single `Paragraph::scroll((y, 0))` over a
  cloned `Text`. No virtualization - the entire transcript is in memory
  and rendered every frame. Manual height calculation tracks total lines.
- **Stick-to-bottom:** `AtomicBool` flag toggled when user scrolls up
  manually. Confirms the pattern fspec uses for `scrollToEnd /
  userScrolledAway`.
- **Input:** Vim-style modes (Normal / Insert / Visual) layered on
  `tui-textarea` 0.7. Multi-key sequences tracked via `previous_key`
  field on the app state.
- **Modals:** Centered `Rect` via nested `Layout::vertical` then
  `Layout::horizontal`, with `Clear` widget to wipe the underlying
  buffer first. Single `FocusedBlock` enum gates which block receives
  input.
- **Mouse:** `crossterm::EnableMouseCapture` enabled at startup; mouse
  events handled in the app's match block.

**Most worth copying:** the `AtomicBool` stick-to-bottom pattern (or our
equivalent `bool` field) and the centered `Rect` + `Clear` modal recipe.

### oatmeal ([github.com/dustinblackman/oatmeal](https://github.com/dustinblackman/oatmeal))

A configurable LLM chat TUI.

- **Chat history rendering:** Custom `BubbleList` widget. Each message
  has a `BubbleCacheEntry { codeblocks_count, text_len, lines }` cached
  by content hash. Manual word-wrap + direct `buf.set_line(...)` calls
  for high control. Uses ratatui's built-in `Scrollbar`.
- **Code highlighting:** Pre-compiled `syntect` syntax sets bundled as
  binary blobs (huge performance win - syntect's runtime parser is
  slow).
- **No popups:** "Command palette" is just slash-commands typed into the
  textarea, with `/help` etc. rendered as chat bubbles.
- **Input:** Vanilla `tui-textarea` 0.4 with Emacs bindings. Raw
  crossterm events translated to a domain `Event` enum *before* the UI
  sees them - clean separation of input and rendering layers.
- **Mouse:** Not emphasized. Bracketed paste supported via tui-textarea.

**Most worth copying:** the per-item rendered-cache pattern
(`BubbleCacheEntry`) for our virtualised lists with heterogeneous items
- caching the rendered line count per item lets the virtualizer compute
total height in O(visible_count) instead of O(total_count).

### aichat ([github.com/sigoden/aichat](https://github.com/sigoden/aichat))

NOT a ratatui app - included for completeness.

- **REPL** on `reedline` (the same line editor nushell uses).
- "Head / tail" trick: when streaming markdown, re-render only the
  changed tail to avoid flicker.
- Selects via `inquire` crate; completion via reedline's
  `ColumnarMenu`.

**Most worth copying:** nothing directly applicable - we're going
ratatui, not reedline.

### Helix ([github.com/helix-editor/helix](https://github.com/helix-editor/helix))

A modal text editor in Rust. NOT ratatui-based, but its Compositor and
KeyTrie patterns are gold.

#### Compositor

```rust
pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
    // ...
}

pub trait Component {
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult;
    fn render(&mut self, area: Rect, surface: &mut Surface, ctx: &mut Context);
    fn cursor(&self, area: Rect, ctx: &Editor) -> (Option<Position>, CursorKind);
    fn should_update(&self) -> bool;
    fn id(&self) -> Option<&'static str>;
}

pub enum EventResult {
    Ignored(Option<Box<dyn FnOnce(&mut Compositor, &mut Context)>>),
    Consumed(Option<Box<dyn FnOnce(&mut Compositor, &mut Context)>>),
}
```

**Key insights:**

- Layers walked in **reverse order** (top of stack first) for
  `handle_event`.
- `EventResult::Consumed` carries an optional callback for self-removal,
  so a popup can dismiss itself by returning
  `Consumed(Some(Box::new(|c, _| { c.pop(); })))`. **This is the
  "ESC closes me" pattern in two lines of Rust.**
- `id()` enables programmatic re-opening of a layer (Helix uses this
  for "last picker" Ctrl+Shift+P).

#### KeyTrie

Helix's keymap is a tree of `KeyTrieNode`s. Multi-key sequences (like
`gd` for go-to-definition) are walked node by node. When a sequence is
incomplete, the trie returns `KeymapResult::Pending`, which drives the
which-key popup auto-render.

**Most worth copying:**

1. The `Component` trait with `handle_event` returning `EventResult` is
   a near-drop-in replacement for our `useInputCompat` registry.
2. The optional callback in `EventResult` for self-removal is elegant
   and avoids the "modal needs a closure to its parent's
   `setIsOpen(false)`" pattern we have today.
3. KeyTrie isn't directly relevant unless we add multi-key sequences,
   but if we do, copy verbatim.

### ratatui-org `templates/component` ([github.com/ratatui/templates/tree/main/component](https://github.com/ratatui/templates/tree/main/component))

The official ratatui app template. Recommended starting point.

```rust
trait Component {
    fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>>;
    fn update(&mut self, action: Action) -> Result<Option<Action>>;
    fn draw(&mut self, frame: &mut Frame<'_>, area: Rect) -> Result<()>;
}
```

- `mpsc<Action>` bus is the Redux-action equivalent.
- `tokio::select!` over `(events, ticks, render_signal, cancel_token)`.
- Each top-level component owns its sub-components.

**Most worth copying:** the entire skeleton - file layout, `Action` enum
shape, the `tokio::select!` loop. Adopt verbatim and bolt our Compositor
on as a layer above the root component.

### lazygit ([github.com/jesseduffield/lazygit](https://github.com/jesseduffield/lazygit))

NOT ratatui (it's gocui in Go), but its **Context** system maps 1:1 to
our `InputPriority` enum:

```go
type ContextKind int
const (
    SIDE_CONTEXT       ContextKind = iota  // BACKGROUND
    MAIN_CONTEXT                           // LOW
    TEMPORARY_POPUP                        // HIGH
    PERSISTENT_POPUP                       // CRITICAL
    EXTRAS                                 // (footer)
)
```

- Each Context owns its own keymap (subset of the global keymap).
- `ParentContextMgr` tracks which context to restore focus to when a
  popup closes.
- `PopupHandler` is a facade: `Confirm`, `Alert`, `Prompt`, `Menu`,
  `Toast` - five popup types covering ~all modal needs.

**Most worth copying:** the enum-of-context-kinds is exactly our
`InputPriority` enum. The `ParentContextMgr` is a useful pattern for
focus restore (when a modal closes, return focus to the previously
focused pane). The five-popup-type facade (`Confirm` / `Alert` /
`Prompt` / `Menu` / `Toast`) is a clean API surface to expose to
business logic.

---

## NOT INVESTIGATED projects worth a follow-up pass

Each of these is known to be either ratatui-based or comparable but was
not reached due to DeepSearch sub-agent timeouts:

- **gitui** ([github.com/gitui-org/gitui](https://github.com/gitui-org/gitui))
  - extracts `tui-utils` and splits panels (status / log / diff /
  blame). Likely a strong source of multi-pane layout patterns.
- **atuin** ([github.com/atuinsh/atuin](https://github.com/atuinsh/atuin))
  - shell-history search overlay. Likely a strong source of
  fuzzy-search-popup patterns.
- **bottom** ([github.com/ClementTsang/bottom](https://github.com/ClementTsang/bottom))
  - htop-style system monitor. Likely a strong source of multi-widget
  dashboard patterns.
- **yazi** ([github.com/sxyazi/yazi](https://github.com/sxyazi/yazi))
  - file manager with `ratatui-image` preview. Likely a strong source of
  multi-pane focus patterns.
- **jjui** ([github.com/idursun/jjui](https://github.com/idursun/jjui))
  - jujutsu (jj) frontend. Newer; smaller; possibly easier to read.
- **television** ([github.com/alexpasmantier/television](https://github.com/alexpasmantier/television))
  - fzf-like fuzzy finder. Channel architecture worth studying.

If RPC-002 is split into child stories, one of the early ones could be
**"Prior-art deep dive on gitui + atuin + bottom"** to fill these gaps
before we commit to architectural decisions.

---

## Reusable Crates Confirmed via this Survey

| Concern | Crate | Pattern |
|---|---|---|
| Multi-line text editor | `tui-textarea` | Stateful widget; rope-like buffer; `input(KeyEvent)` API; ships canonical `examples/vim.rs` state machine |
| Single-line input | `tui-input` | Headless cursor + buffer state machine |
| Polished prompts | `tui-prompts` | Stateful widget + caller-owned `TextState` |
| Centered popup | `tui-popup` | Wrapper; supports drag via `PopupState`; can host any `SizedWidgetRef` |
| Scrolling beyond viewport | `tui-scrollview` | Off-screen buffer + viewport offset; great for heterogeneous mixed-widget content |
| Inline images (Sixel / Kitty / iTerm / halfblocks) | `ratatui-image` | Capability detection; cell-aware over-render protection; offload `StatefulImage` to tokio task |
| Code highlighting (chat) | `syntect` (oatmeal-style precompiled blob) | Pre-compile syntax sets to binary for fast load |
| Spinners | `throbber-widgets-tui` | Drop-in spinner widgets |
| Toasts | `ratatui-toaster` | Status notifications |
