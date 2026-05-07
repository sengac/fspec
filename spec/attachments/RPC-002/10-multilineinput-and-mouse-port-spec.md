# 10 — MultiLineInput & Mouse Subsystem Port Specification

This document is the detailed port plan for two related subsystems:

- `src/tui/components/MultiLineInput.tsx` - the rich-text composer with
  history, slash-command palette, file mention popup, bracketed paste,
  and multi-line compaction.
- `src/tui/utils/mouseProtocol.ts` - the SGR 1006 parser and mouse
  enable / disable sequences.

---

## Part A. MultiLineInput on `tui-textarea`

### A.1 Why `tui-textarea`

[`tui-textarea`](https://github.com/rhysd/tui-textarea) provides:

- Rope-like buffer (efficient inserts at any position).
- Built-in undo / redo.
- Soft-wrap rendering.
- Bracketed paste support.
- Cursor styling.
- Line numbers (optional).
- Search.
- Scrolling within the textarea.
- Documented `examples/vim.rs` showing how to layer mode state on top.

It's used by tenere (the closest comparable Rust chat TUI) and is
maintained.

### A.2 What we add on top

| Feature | Source | Approx LoC |
|---|---|---|
| Slash-command palette popup (`/` at column 0) | `SlashCommandPalette.tsx` | ~80 |
| File-mention popup (`@`) | `FileSearchPopup.tsx` | ~120 |
| History (Up / Down at top / bottom) | `MultiLineInput.tsx` history logic | ~50 |
| Submit on Enter / newline on Shift+Enter | `MultiLineInput.tsx` | ~20 |
| Auto-grow up to max height | `MultiLineInput.tsx` UX-002 | ~30 |
| Persist history across sessions | New (file in fspec data dir) | ~40 |

Total custom Rust on top of `tui-textarea`: **~340 LoC**.

### A.3 Public API

```rust
pub struct MultiLineInput<'a> {
    id: String,
    textarea: TextArea<'a>,
    history: Vec<String>,
    history_index: Option<usize>,
    max_height: u16,
    on_submit: Option<Box<dyn Fn(String) -> Action + Send + Sync>>,
    /// Active inline popup (slash / file).
    active_popup: ActivePopup,
    action_tx: mpsc::UnboundedSender<Action>,
    /// Available slash commands (for filtering).
    slash_commands: Arc<Vec<SlashCommand>>,
    /// File search backend (for @-mention).
    file_search: Arc<dyn FileSearchBackend>,
}

enum ActivePopup {
    None,
    Slash(SlashPopupState),
    File(FilePopupState),
}

pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub action: Action,
}
```

### A.4 Lifecycle

```rust
impl<'a> Component for MultiLineInput<'a> {
    fn priority(&self) -> Priority { Priority::Medium }
    fn id(&self) -> &str { &self.id }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // 1. If a popup is active, give it first crack at the event.
        match &mut self.active_popup {
            ActivePopup::Slash(state) => {
                let r = state.handle_event(event, &self.textarea);
                if matches!(r, PopupResult::Handled) { return EventResult::Consumed(None); }
                if matches!(r, PopupResult::Dismiss) { self.active_popup = ActivePopup::None; }
                if matches!(r, PopupResult::Submit(_)) { /* insert into textarea, dismiss */ }
            }
            ActivePopup::File(state) => { /* same */ }
            ActivePopup::None => {}
        }

        // 2. Submit on Enter (if multi-line mode allows it).
        if let Event::Key(KeyEvent { code: KeyCode::Enter, modifiers, .. }) = event {
            if !modifiers.contains(KeyModifiers::SHIFT) {
                let text = self.textarea.lines().join("\n");
                if let Some(cb) = &self.on_submit {
                    self.action_tx.send(cb(text.clone())).ok();
                }
                self.textarea = TextArea::default();
                self.history.push(text);
                self.history_index = None;
                return EventResult::Consumed(None);
            }
        }

        // 3. History navigation at edges.
        if let Event::Key(k) = event {
            if k.code == KeyCode::Up && self.cursor_at_top() {
                self.history_prev();
                return EventResult::Consumed(None);
            }
            if k.code == KeyCode::Down && self.cursor_at_bottom() {
                self.history_next();
                return EventResult::Consumed(None);
            }
        }

        // 4. Detect popup triggers BEFORE forwarding to textarea.
        if let Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) = event {
            if *c == '/' && self.cursor_at_column_zero() {
                self.active_popup = ActivePopup::Slash(SlashPopupState::default());
                // fall through to insert the '/'
            } else if *c == '@' {
                self.active_popup = ActivePopup::File(FilePopupState::new(
                    self.file_search.clone()
                ));
                // fall through to insert the '@'
            }
        }

        // 5. Forward to tui-textarea.
        if let Event::Key(k) = event {
            self.textarea.input(*k);
            return EventResult::Consumed(None);
        }

        EventResult::Ignored(None)
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let h = self.compute_height(area);
        let input_area = Rect { height: h, ..area };

        self.textarea.set_block(/* border etc. */);
        self.textarea.render(input_area, buf);

        // Inline popup (anchored to cursor)
        match &mut self.active_popup {
            ActivePopup::Slash(state) => {
                let popup_area = self.popup_anchor(input_area);
                state.render(popup_area, buf);
            }
            ActivePopup::File(state) => {
                let popup_area = self.popup_anchor(input_area);
                state.render(popup_area, buf);
            }
            ActivePopup::None => {}
        }
    }
}
```

### A.5 SlashCommandPalette state

```rust
pub struct SlashPopupState {
    /// What the user has typed after `/`.
    filter: String,
    /// Filtered + selected.
    filtered: Vec<SlashCommand>,
    selected: usize,
}

impl SlashPopupState {
    pub fn handle_event(&mut self, event: &Event, textarea: &TextArea<'_>) -> PopupResult {
        match event {
            Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => PopupResult::Dismiss,
            Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {
                self.selected = self.selected.saturating_sub(1);
                PopupResult::Handled
            }
            Event::Key(KeyEvent { code: KeyCode::Down, .. }) => {
                self.selected = (self.selected + 1).min(self.filtered.len().saturating_sub(1));
                PopupResult::Handled
            }
            Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                self.filtered.get(self.selected)
                    .map(|cmd| PopupResult::Submit(cmd.action.clone()))
                    .unwrap_or(PopupResult::Dismiss)
            }
            Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                if self.filter.is_empty() {
                    PopupResult::Dismiss
                } else {
                    self.filter.pop();
                    self.refresh_filter();
                    PopupResult::Handled
                }
            }
            _ => PopupResult::NotHandled, // let textarea also see the event
        }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Render filtered list as a small popup above the input.
        // Anchored to cursor x; clamped to fit area.
    }
}

pub enum PopupResult {
    NotHandled,        // pass through
    Handled,           // consumed, don't pass through
    Dismiss,           // close popup, don't pass event through
    Submit(Action),    // execute action and close popup
}
```

### A.6 FileSearchPopup

Similar structure to slash palette, but the "filter" runs against a
fuzzy file search backend.

```rust
pub trait FileSearchBackend: Send + Sync {
    fn search(&self, query: &str, limit: usize) -> Vec<PathBuf>;
}

// Implementation can use `ignore` + `nucleo` (the matcher used by helix
// / television), or fall back to a simple `walkdir` + substring filter.
```

### A.7 History persistence

Stored in `~/.fspec/tui-history.jsonl` (one entry per line). Load on
startup, append on submit, cap at 1000 entries.

### A.8 Auto-grow (UX-002)

```rust
fn compute_height(&self, area: Rect) -> u16 {
    let line_count = self.textarea.lines().len() as u16;
    let with_border = line_count + 2; // top + bottom border
    with_border.min(self.max_height).max(3) // 3 = min height
}
```

### A.9 Test plan

| Scenario | Test type |
|---|---|
| Type a character; assert it appears in textarea.lines() | unit |
| Enter submits; assert on_submit called with full text | unit |
| Shift+Enter inserts newline | unit |
| Up at top of input pulls history | unit |
| Down at bottom moves to next history / clears | unit |
| `/` opens slash popup | unit |
| Up/Down navigates slash popup | unit |
| Enter on slash popup item dispatches action | unit |
| ESC closes slash popup | unit |
| `@` opens file popup | unit |
| Backspace before popup char closes popup | unit |
| Bracketed paste inserts atomically (one undo step) | unit |
| Auto-grows up to max_height | snapshot |
| History persists across runs | integration (temp dir) |
| Submitting empty input is ignored | unit |

---

## Part B. Mouse Subsystem

### B.1 Replacement for `mouseProtocol.ts`

The entire file - regex parser, button constants, enable / disable
sequences - is replaced by crossterm:

```rust
// Cargo.toml
crossterm = { version = "0.x", features = ["bracketed-paste"] }

// In main():
crossterm::execute!(
    std::io::stdout(),
    crossterm::event::EnableMouseCapture,
    crossterm::event::EnableBracketedPaste,
)?;

// On exit:
crossterm::execute!(
    std::io::stdout(),
    crossterm::event::DisableMouseCapture,
    crossterm::event::DisableBracketedPaste,
)?;
```

| TS constant / function | Rust equivalent |
|---|---|
| `parseSgrMouse(input)` | crossterm parses internally; you receive `Event::Mouse(MouseEvent)` |
| `MOUSE_ENABLE` | `EnableMouseCapture` |
| `MOUSE_DISABLE` | `DisableMouseCapture` |
| `SGR_BUTTON.LEFT` | `MouseButton::Left` |
| `SGR_BUTTON.MIDDLE` | `MouseButton::Middle` |
| `SGR_BUTTON.RIGHT` | `MouseButton::Right` |
| `SGR_BUTTON.SCROLL_UP` | `MouseEventKind::ScrollUp` |
| `SGR_BUTTON.SCROLL_DOWN` | `MouseEventKind::ScrollDown` |
| 1-based coords | 0-based `column` / `row` |

### B.2 Native text-selection toggle (TUI-078)

The Ink version writes raw escape sequences (`MOUSE_ENABLE` /
`MOUSE_DISABLE`) on focus / blur and on mouse press. The Rust port uses
crossterm and a tokio timer:

```rust
// mouse/text_selection_toggle.rs

pub struct MouseTrackingToggle {
    disabled: bool,
    re_enable_handle: Option<JoinHandle<()>>,
    action_tx: mpsc::UnboundedSender<Action>,
    component_id: String,
}

impl MouseTrackingToggle {
    pub fn new(component_id: String, tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { disabled: false, re_enable_handle: None, action_tx: tx, component_id }
    }

    pub fn temporarily_disable(&mut self) {
        // Cancel pending re-enable timer.
        if let Some(h) = self.re_enable_handle.take() { h.abort(); }
        // Disable mouse capture.
        let _ = execute!(stdout(), DisableMouseCapture);
        self.disabled = true;
        // Schedule re-enable in 5 seconds.
        let tx = self.action_tx.clone();
        let id = self.component_id.clone();
        self.re_enable_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            tx.send(Action::ReEnableMouse(id)).ok();
        }));
    }

    pub fn re_enable(&mut self) {
        if let Some(h) = self.re_enable_handle.take() { h.abort(); }
        if self.disabled {
            let _ = execute!(stdout(), EnableMouseCapture);
            self.disabled = false;
        }
    }
}

impl Drop for MouseTrackingToggle {
    fn drop(&mut self) {
        if let Some(h) = self.re_enable_handle.take() { h.abort(); }
        if self.disabled {
            let _ = execute!(stdout(), EnableMouseCapture);
        }
    }
}
```

### B.3 Hit-testing helper

ratatui has no widget hit-testing. We provide a small utility:

```rust
// mouse/hit_test.rs

pub fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x && x < rect.x + rect.width &&
    y >= rect.y && y < rect.y + rect.height
}
```

Components remember their last-rendered area as a field, then in
`handle_event` they hit-test against it for mouse events:

```rust
fn handle_event(&mut self, event: &Event) -> EventResult {
    if let Event::Mouse(m) = event {
        if !rect_contains(self.last_area, m.column, m.row) {
            return EventResult::Ignored(None);
        }
        // ... handle ...
    }
}
```

### B.4 Tests

| Scenario | Test type |
|---|---|
| Press disables, release re-enables | unit (mock execute!) |
| Repeated press resets the timer | unit |
| Drop while disabled re-enables on exit | unit |
| Hit-test for nested rects | unit |
| Wheel events ignored when outside rect | unit |

---

## Part C. Bracketed paste

`tui-textarea` handles bracketed paste internally if we enable
`crossterm::event::EnableBracketedPaste`. We just enable it on startup
and forward `Event::Paste(s)` to the focused textarea.

```rust
fn handle_event(&mut self, event: &Event) -> EventResult {
    if let Event::Paste(text) = event {
        // tui-textarea v0.7+ has insert_str
        self.textarea.insert_str(text);
        return EventResult::Consumed(None);
    }
    // ...
}
```
