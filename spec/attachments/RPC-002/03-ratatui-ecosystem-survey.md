# 03 — Ratatui Ecosystem Survey

> Source: parallel investigation by agent `4419f7de-2a81-4534-9ede-227af14279e9`,
> using the `DeepSearch` tool against crates.io, GitHub, and the
> `ratatui-org` mono-repos.

This document is a capability matrix of the ratatui ecosystem against the
specific behaviours that `VirtualList` and `Dialog` deliver in the current
Ink/React TUI. For each requirement we list every crate we found, the
verdict, and the gap we would still have to fill ourselves.

---

## A. Capability Matrix vs Our Requirements

### A.1 Virtualized lists (long beyond viewport)

| Crate | Verdict | Notes |
|---|---|---|
| **`tui-widget-list`** ([crates.io](https://crates.io/crates/tui-widget-list) / [GitHub](https://github.com/preiter93/tui-widget-list)) | Closest match | Variable per-item heights, viewport rendering, kbd nav (Up / Down / PgUp / PgDn / Home / End), wrap-around, scroll padding. v0.15.x, MIT, ~46 stars, ~52 k recent dl. |
| **`tui-scrollview`** ([crates.io](https://crates.io/crates/tui-scrollview) / part of [`ratatui-org/tui-widgets`](https://github.com/ratatui/tui-widgets)) | Buffer-based | Renders into oversized buffer + crops with paired Scrollbars. **NOT item-based virtualization** - entire inner buffer is built each frame. v0.6.x, MIT/Apache-2.0, mono-repo umbrella ~196 stars. |
| **ratatui core `List` / `Table`** | Windowed only | Visible rows are buffered, but `ListItem`s are still constructed every frame. Fine for hundreds, painful for tens of thousands without DIY work. |
| **`ratatui-cheese`** | Lightweight | Bubbletea-style list / tree / paginator suite. Not as tunable as `tui-widget-list`. |

**Gaps vs Ink VirtualList:**

- No crate offers **lazy `getItems(start, end)` provider** - you'd
  implement that yourself.
- No crate has **group-id selection preservation across mutations** -
  DIY (~30 LoC).
- No crate models **scroll vs item selection modes** as a first-class
  concept - DIY.
- No crate has **scrollToEnd auto-stick + user-scrolled-away detection**
  - DIY.

### A.2 Custom scrollbars / thumb rendering

| Source | Verdict |
|---|---|
| **ratatui core `Scrollbar` + `ScrollbarState`** | Render-only. Configurable orientation, thumb / track / arrow glyphs and styles. [Docs](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Scrollbar.html) |
| **`tui-scrollview`** | Bundles paired H+V scrollbars driven by inner-buffer extent. |
| **`rat-widget`** ([crates.io](https://crates.io/crates/rat-widget) / [GitHub](https://github.com/thscharler/rat-salsa)) | Containers (`View` / `Split` / `Tabbed` / `MultiPage`) with mouse-aware scrollbars baked in. |

**Gap:** No crate ships **draggable thumb hit-testing**. ~30 LoC DIY:
remember the scrollbar `Rect`, on `MouseEventKind::Down(Left)` test
containment, on `Drag` map `mouse.row -> ScrollbarState.position`.
Scrollbar example exists at
[`ratatui/examples/apps/scrollbar`](https://github.com/ratatui/ratatui/tree/main/examples/apps/scrollbar)
but is **keyboard-only**.

### A.3 Modal popups / floating dialogs

| Crate | Verdict |
|---|---|
| **`tui-popup`** ([crates.io](https://crates.io/crates/tui-popup) / part of [`ratatui-org/tui-widgets`](https://github.com/ratatui/tui-widgets)) | De-facto choice. Centered popup over rendered content, border, title, body, optional drag-to-move. Adopted into the official ratatui org mono-repo (long-term viability signal). |
| **`tui-overlay`** ([crates.io](https://crates.io/crates/tui-overlay) / [GitHub](https://github.com/jharsono/tui-overlay)) | Richer primitives: drawers, modals, popovers, toasts. Younger / less downloads. |
| **`tui-confirm-dialog`** ([GitHub](https://github.com/sephiroth74/tui-confirm-dialog)) | Narrow Yes/No use case. |
| **`rat-dialog`** ([crates.io](https://crates.io/crates/rat-dialog)) | `DialogStack` - stacks multiple dialogs above app, dialogs receive events first. Marked alpha / unstable. |
| **`ratatui-toaster`** | Toast notifications (not modal). |

**Gap vs Ink Dialog:** Dialog widget is solved; **input-priority routing
is not** - see A.4 below.

### A.4 Layered / priority input routing (CRITICAL priority capture)

ratatui itself has **no input dispatcher** - it's a render library.
Ecosystem options:

| Option | Verdict |
|---|---|
| **`rat-event`** (part of [`rat-salsa`](https://github.com/thscharler/rat-salsa)) | **Most idiomatic.** `HandleEvent<Event, Qualifier, Return>` trait with `Regular` / `MouseOnly` / `DoubleClick` / `Popup` / **`Dialog`** qualifiers. The `Dialog` qualifier is described in their docs as *"called first to be able to consume all events, thus blocking everything else"* - direct analogue of Ink's CRITICAL priority. v4.x, MIT/Apache-2.0, active May 2026. |
| **`rat-dialog` `DialogStack`** | Layered on top of rat-event. |
| **`tui-realm` Subscriptions** | Focus-based. View routes events to focused component; Subscriptions + `EventClause` / `SubClause` route to unfocused; subscription locks exist. Less explicit "intercept-before-background" than rat-event. |
| **`ratatui-interact`** | Smaller widget toolkit; basic focus traversal only. |
| **DIY Helix-style `Compositor`** | `Vec<Box<dyn Component>>` walked top-down, each returning `EventResult::{Consumed, Ignored}`. ~30 LoC. Lazygit's `Context.Kind` enum is the same idea. |

### A.5 Mouse support including scroll wheel

| Source | Notes |
|---|---|
| **crossterm** | Parses SGR 1006 internally. Delivers `MouseEvent { kind, row, column, modifiers }` with `kind ∈ {Down(b), Up(b), Drag(b), Moved, ScrollUp, ScrollDown}`. **Replaces our entire `mouseProtocol.ts`.** |
| **termion / termwiz** | Alternative backends; less commonly used with ratatui. |

ratatui has **no mouse event abstraction** of its own; we read crossterm
events directly. Hit-testing is the app's job.

### A.6 Flex-style layout (Yoga-like)

ratatui has **no Yoga**. The closest analogue:

- **ratatui core `Layout` + `Constraint`**: `Length(n)`,
  `Percentage(p)`, `Min(n)`, `Max(n)`, `Ratio(a, b)`, `Fill(weight)`.
  Composed by nesting `Layout::vertical/horizontal(...).split(area)`.
- The **`Fill(weight)`** constraint (added in ratatui 0.26+) is the
  closest equivalent to Ink's `flexGrow: weight`.
- Codex re-invented its own Flutter-inspired `Renderable` trait with
  `desired_height(width)` because ratatui's constraint model didn't fit
  their chat-cell-by-cell flow. We **don't need this** for fspec - our
  layouts are statically constrainable.

### A.7 Text-area / multi-line input

| Crate | Verdict | Notes |
|---|---|---|
| **`tui-textarea`** ([crates.io](https://crates.io/crates/tui-textarea) / [GitHub](https://github.com/rhysd/tui-textarea)) | **Recommended.** Stateful widget; rope-like buffer; `input(KeyEvent)` API; ships canonical `examples/vim.rs` state machine. Used by tenere. |
| **`tui-input`** ([GitHub](https://github.com/sayanarijit/tui-input)) | Single-line headless cursor + buffer state machine. |
| **`tui-prompts`** ([part of `ratatui-org/tui-widgets`](https://github.com/ratatui/tui-widgets)) | Stateful widget + caller-owned `TextState`. Polished prompt UX. |
| **reedline** ([GitHub](https://github.com/nushell/reedline)) | NOT a ratatui widget - a full REPL line editor. Used by aichat / nushell. |

---

## B. Crate-by-Crate Detail

### B.1 `tui-widget-list`

- **Repo:** https://github.com/preiter93/tui-widget-list
- **Crate:** https://crates.io/crates/tui-widget-list
- **Latest:** v0.15.x (2025), maintained
- **License:** MIT
- **API surface:**
  - `trait ListableWidget`: per-item `size_hint() -> ListableSize`,
    `highlight()` styling.
  - `struct ListBuilder<T>`: lazy item generator
    `Box<dyn Fn(usize, bool, &Context) -> T>` - **partial fit for our
    lazy mode**, but does not give viewport range, just per-index queries.
  - `struct ListView<T>`: stateful widget rendering visible window.
  - `struct ListState`: `selected: Option<usize>`, `offset: usize`,
    plus `next` / `previous` / `select_first` / `select_last`.
- **What it gives us:** virtualization, variable heights, wrap-around,
  default keymap.
- **What it doesn't:** group selection, scroll vs item modes, mouse,
  scrollbar, `getItems(start, end)` range accessor.

### B.2 `tui-popup`

- **Repo:** https://github.com/ratatui/tui-widgets/tree/main/tui-popup
- **Crate:** https://crates.io/crates/tui-popup
- **Latest:** v0.6.x, in the official `ratatui-org` mono-repo
- **License:** MIT / Apache-2.0
- **API surface:**
  - `struct Popup<'a, W: SizedWidgetRef>`: title, body widget, border, style.
  - `struct PopupState`: `area: Rect`, drag offset, `mouse_down_on(area)`.
- **What it gives us:** centered popup rendering, optional drag-to-move,
  composes with any inner widget (so `tui-textarea`, `Paragraph`, custom
  forms all work inside it).
- **What it doesn't:** Input priority. We provide that ourselves.

### B.3 `tui-textarea`

- **Repo:** https://github.com/rhysd/tui-textarea
- **Crate:** https://crates.io/crates/tui-textarea
- **Latest:** v0.7.x
- **License:** MIT
- **API surface:**
  - `struct TextArea<'a>`: `input(KeyEvent)`, `input_without_shortcuts`,
    `lines()`, `cursor()`, `set_block`, `set_cursor_style`, `paste`,
    `undo` / `redo`.
  - Built-in features: undo/redo, line numbers, soft-wrap, hard-tab handling,
    bracketed paste, search.
  - Canonical `examples/vim.rs` shows how to layer mode state machine
    on top.
- **What it gives us:** the rope buffer, undo, paste, word-wrap, cursor
  rendering.
- **What it doesn't:** slash-command palette, file-mention popup,
  history persistence, fspec-specific submit semantics.

### B.4 ratatui core `Scrollbar`

- **Docs:** https://docs.rs/ratatui/latest/ratatui/widgets/struct.Scrollbar.html
- **State:** `ScrollbarState::new(content_length).position(pos).viewport_content_length(view_len)`.
- **Render:** `frame.render_stateful_widget(Scrollbar::new(orientation), area, &mut state)`.
- Configurable thumb / track / start-end glyphs, styles.

### B.5 `rat-event` / `rat-salsa` framework

- **Repo:** https://github.com/thscharler/rat-salsa
- **Crates:** `rat-event`, `rat-widget`, `rat-dialog`, `rat-focus`,
  `rat-text`, `rat-window`, `rat-theme`, `rat-popup`, etc.
- **License:** MIT / Apache-2.0
- **Key design:** `HandleEvent<Event, Qualifier, Return>` trait.
  Qualifiers are typed marker structs (`Regular`, `MouseOnly`,
  `DoubleClick`, `Popup`, `Dialog`). The `Dialog` qualifier is documented
  as *"called first to consume all events, thus blocking everything
  else"* - exactly our `InputPriority::CRITICAL`.
- **Trade-off:** Adopting `rat-salsa` is essentially adopting another
  framework on top of ratatui. Substantial learning curve and lock-in.
  Benefit: well-thought-out solutions to focus, popup stacking, dialogs.

### B.6 `tui-realm`

- **Repo:** https://github.com/veeso/tui-realm
- **Crate:** https://crates.io/crates/tui-realm
- **Design:** React-like Component / View / Subscription model. Each
  component returns `Msg` from key events; the View dispatches.
- **Verdict:** Closest mental model to React/Ink, BUT its focus-based
  event routing fights our priority-based dispatch. Not recommended for
  this port.

### B.7 Other notable crates

| Crate | Use-case | Notes |
|---|---|---|
| `tui-tree-widget` | Hierarchical tree | Not central to our needs |
| `ratatui-image` | Inline images (Sixel, Kitty, iTerm2, halfblocks) | Capability detection, cell-aware over-render protection |
| `tui-big-text` | ASCII banner text | Cosmetic |
| `throbber-widgets-tui` | Spinners / loading indicators | Useful for ThinkingIndicator |
| `ratatui-explorer` | File tree explorer | Useful for AttachmentDialog |
| `tui-menu` | Dropdown menus | Less central |
| `tui-prompts` | Polished single-line prompts | Single-line variant of tui-textarea |
| `ratatui-toaster` | Toast notifications | Useful for status messages |

---

## C. Recommended Composition

To replicate `VirtualList` and `Dialog` (the two anchor components),
the smallest sufficient stack is:

```toml
[dependencies]
ratatui          = { version = "0.x", features = ["all-widgets"] }
crossterm        = "0.x"   # input + parsed SGR mouse events
tui-widget-list  = "0.15"  # virtualization core
tui-popup        = "0.x"   # dialog rendering
tui-textarea     = "0.7"   # MultiLineInput rope buffer
tui-input        = "0.x"   # single-line dialog fields (optional)
tokio            = { version = "1", features = ["full"] }
```

**Custom Rust to write:**

1. App-level `Compositor` (~30 LoC) - priority-routed event dispatch,
   replacing the InputManager.
2. `VirtualList` wrapper (~600 LoC) on top of `tui-widget-list` +
   ratatui `Scrollbar` - adds group selection, lazy mode, scroll vs item
   modes, scrollToEnd, mouse-wheel velocity, native text-selection toggle.
3. `Dialog` wrapper (~80 LoC) on top of `tui-popup` - adds CRITICAL
   priority registration with the Compositor.
4. Each concrete dialog as a thin Component over the wrapper.

---

## D. React-like Frameworks on ratatui

| Framework | Verdict |
|---|---|
| **`tui-realm`** | React-ish, but focus-based dispatch fights our priority model. Skip. |
| **`rat-salsa`** | Not React-like, but offers the most polished priority/focus/dialog story. Worth considering as a *foundation* if we want less DIY. |
| **`ratatui-async-app`** | Minimal scaffold; not a full framework. |
| **`ratatui-org/templates/component`** | Official template - Component trait + `mpsc<Action>` bus + tokio loop. **Recommended starting point.** |
| **`Cursive`** | Pre-dates ratatui; standalone TUI framework. Different model. Not a path forward. |

**Recommendation:** start from the `templates/component` template, add a
Helix-inspired `Compositor` for priority dispatch, and stay close to bare
ratatui. Adopt `tui-popup` / `tui-widget-list` / `tui-textarea` as
focused dependencies, not full frameworks.

---

## E. Awesome-Ratatui examples worth studying

- [`ratatui/examples/apps/scrollbar`](https://github.com/ratatui/ratatui/tree/main/examples/apps/scrollbar)
  - keyboard-only Scrollbar usage.
- [`ratatui/examples/apps/popup`](https://github.com/ratatui/ratatui/tree/main/examples/apps)
  - centered popup pattern using `Clear`.
- [`tui-widget-list/examples`](https://github.com/preiter93/tui-widget-list/tree/main/examples)
  - several variable-height list demos.
- [`tui-textarea/examples/vim.rs`](https://github.com/rhysd/tui-textarea/blob/main/examples/vim.rs)
  - canonical mode-state-machine pattern.
- [`ratatui-org/templates/component`](https://github.com/ratatui/templates/tree/main/component)
  - the recommended app skeleton.
