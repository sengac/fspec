# 09 — Dialog & Input Priority Manager Port Specification

This document is the detailed port plan for:

- `src/components/Dialog.tsx` (~75 LoC) - the base Dialog overlay.
- `src/tui/input/InputManager.tsx`, `InputHandlerRegistry.ts`,
  `useInputCompat.ts`, `types.ts`, `InputContext.ts` - the centralised
  priority-based input dispatcher.

Both pieces ride on the **Compositor** described in
[`07-recommended-architecture.md`](07-recommended-architecture.md).

---

## Part A. Input Priority Manager

### A.1 Priority enum

```rust
// input/priority.rs

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u32)]
pub enum Priority {
    Background = 100,
    Low = 200,
    Medium = 500,
    High = 800,
    Critical = 1000,
}

impl Priority {
    pub fn as_u32(self) -> u32 { self as u32 }
}
```

Mirrors `src/tui/input/types.ts`:

| TS | Rust |
|---|---|
| `InputPriority.CRITICAL = 1000` | `Priority::Critical` |
| `InputPriority.HIGH = 800` | `Priority::High` |
| `InputPriority.MEDIUM = 500` | `Priority::Medium` |
| `InputPriority.LOW = 200` | `Priority::Low` |
| `InputPriority.BACKGROUND = 100` | `Priority::Background` |

### A.2 Conventions

| Use-case | Priority |
|---|---|
| Modal dialogs | Critical |
| HITL prompts, important overlays | High |
| Primary text input (composer, dialog fields) | Medium |
| Mode / global shortcuts (Ctrl+C, /quit) | Low |
| Passive scroll / nav (VirtualList) | Background |

### A.3 EventResult

```rust
pub type Callback = Box<dyn FnOnce(&mut Compositor) + Send>;

pub enum EventResult {
    /// Event ignored; propagate to the next handler.
    Ignored(Option<Callback>),
    /// Event consumed; stop propagation. Optional follow-up to mutate
    /// the compositor (e.g., pop self).
    Consumed(Option<Callback>),
}
```

### A.4 Component trait (recap from arch doc)

```rust
pub trait Component: Send {
    fn priority(&self) -> Priority { Priority::Medium }
    fn is_active(&self) -> bool { true }
    fn id(&self) -> &str;

    fn handle_event(&mut self, event: &Event) -> EventResult {
        EventResult::Ignored(None)
    }
    fn update(&mut self, _action: Action) -> Option<Action> { None }
    fn render(&mut self, area: Rect, buf: &mut Buffer);
}
```

### A.5 Compositor (full impl)

```rust
// app/compositor.rs

pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
    /// FIFO tiebreak counter. Newer registrations win at equal priority.
    registration_counter: u64,
}

impl Compositor {
    pub fn new() -> Self {
        Self { layers: vec![], registration_counter: 0 }
    }

    pub fn push(&mut self, component: Box<dyn Component>) {
        self.layers.push(component);
        self.registration_counter += 1;
        self.sort();
    }

    pub fn pop(&mut self) -> Option<Box<dyn Component>> {
        let popped = self.layers.pop();
        self.sort();
        popped
    }

    /// Remove the layer with the given id, returning it.
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn Component>> {
        let pos = self.layers.iter().position(|l| l.id() == id)?;
        Some(self.layers.remove(pos))
    }

    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        // Walk in priority order: highest priority first, then within
        // the same priority, last-registered first (FIFO of stack).
        for layer in self.layers.iter_mut().rev() {
            if !layer.is_active() { continue; }
            match layer.handle_event(event) {
                EventResult::Ignored(_) => continue,
                consumed => return consumed,
            }
        }
        EventResult::Ignored(None)
    }

    pub fn update(&mut self, action: Action) -> Option<Action> {
        for layer in self.layers.iter_mut() {
            if let Some(follow_up) = layer.update(action.clone()) {
                return Some(follow_up);
            }
        }
        None
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Render bottom-up so highest priority paints last (= on top).
        for layer in self.layers.iter_mut() {
            layer.render(area, buf);
        }
    }

    fn sort(&mut self) {
        // Sort ascending by priority; iter_mut().rev() then yields
        // highest priority first.
        self.layers.sort_by_key(|a| a.priority());
    }
}
```

### A.6 Behaviours preserved from `useInputCompat`

| TS behaviour | Rust equivalent |
|---|---|
| `useLayoutEffect` race-fix | Not needed - `push` is synchronous |
| `isActive` closure called per event | `is_active(&self) -> bool` called per event |
| FIFO tiebreak | Sort is stable; equal priority preserves insertion order |
| `unregister` on unmount | `pop()` or `remove(id)` when modal closes |
| Standalone fallback (no InputManager) | Compositor is mandatory; tests use a `Compositor` directly |

### A.7 Tests

```rust
#[test]
fn higher_priority_intercepts_first() {
    let mut c = Compositor::new();
    c.push(Box::new(BackgroundComponent));   // Priority::Background
    c.push(Box::new(CriticalComponent));     // Priority::Critical
    let result = c.handle_event(&key('a'));
    assert!(matches!(result, EventResult::Consumed(_)));
    assert_eq!(BACKGROUND_HIT.load(Ordering::SeqCst), false);
    assert_eq!(CRITICAL_HIT.load(Ordering::SeqCst), true);
}

#[test]
fn ignored_consumed_propagates_correctly() { /* ... */ }

#[test]
fn is_active_false_skips_handler() { /* ... */ }

#[test]
fn fifo_tiebreak_at_equal_priority() { /* ... */ }

#[test]
fn callback_in_consumed_runs_after_event() { /* ... */ }

#[test]
fn modal_pushed_at_critical_intercepts_keystroke() { /* ... */ }
```

---

## Part B. Dialog (base overlay)

### B.1 Public API

```rust
// components/dialog.rs

pub struct Dialog<C: Component> {
    id: String,
    border_color: Option<Color>,
    is_active: bool,
    inner: C,
    /// Called when ESC is pressed. Defaults to popping self.
    on_close: Option<Box<dyn FnOnce() -> Action + Send>>,
}

impl<C: Component> Dialog<C> {
    pub fn new(id: impl Into<String>, inner: C) -> Self { /* ... */ }
    pub fn border_color(mut self, c: Color) -> Self { /* ... */ }
    pub fn on_close(mut self, f: impl FnOnce() -> Action + Send + 'static) -> Self { /* ... */ }
}
```

### B.2 Component impl

```rust
impl<C: Component + 'static> Component for Dialog<C> {
    fn priority(&self) -> Priority { Priority::Critical }
    fn is_active(&self) -> bool { self.is_active }
    fn id(&self) -> &str { &self.id }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // ESC: pop self
        if let Event::Key(KeyEvent { code: KeyCode::Esc, .. }) = event {
            let id = self.id.clone();
            return EventResult::Consumed(Some(Box::new(move |c| {
                c.remove(&id);
            })));
        }
        // Delegate to inner
        self.inner.handle_event(event)
    }

    fn update(&mut self, action: Action) -> Option<Action> {
        self.inner.update(action)
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // Compute centered sub-rect based on inner's preferred size.
        let popup_area = centered_rect(area, /* width, height */);

        // Wipe everything underneath
        Clear.render(popup_area, buf);

        // Bordered, padded, black-bg block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(self.border_color
                .map(|c| Style::default().fg(c))
                .unwrap_or_default())
            .style(Style::default().bg(Color::Black))
            .padding(Padding::uniform(1));
        let inner_area = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render inner content into the padded area
        self.inner.render(inner_area, buf);
    }
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let v = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ]).split(area);
    let h = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(width),
        Constraint::Min(0),
    ]).split(v[1]);
    h[1]
}
```

### B.3 Alternative: use `tui-popup` crate

```rust
use tui_popup::{Popup, SizedWidgetRef};

impl<C: Component + 'static> Component for Dialog<C> {
    // ...
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let popup = Popup::new(self.inner.as_widget_ref())
            .style(Style::default().bg(Color::Black))
            .border_type(BorderType::Rounded);
        popup.render(area, buf);
    }
}
```

The downside is that `tui-popup` expects a `SizedWidgetRef` (a static
size-hint widget), and our inner Component is dynamic. We'd wrap it in a
small adapter. Not a hard problem, but the manual centered-rect version
above is also fine.

**Recommendation:** start with the manual version (above). Adopt
`tui-popup` only if we want drag-to-move (which the Ink Dialog does NOT
support today).

---

## Part C. Concrete Dialog ports

### C.1 ConfirmationDialog

```rust
pub struct ConfirmationDialog {
    title: String,
    message: String,
    confirm_label: String,
    cancel_label: String,
    focused: ConfirmFocus,  // Confirm | Cancel
    result_tx: Option<oneshot::Sender<bool>>,
    id: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfirmFocus { Confirm, Cancel }

impl Component for ConfirmationDialog {
    fn priority(&self) -> Priority { Priority::Critical }
    fn id(&self) -> &str { &self.id }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        let key = match event { Event::Key(k) => k, _ => return EventResult::Ignored(None) };
        match key.code {
            KeyCode::Tab | KeyCode::BackTab |
            KeyCode::Left | KeyCode::Right => {
                self.focused = match self.focused {
                    ConfirmFocus::Confirm => ConfirmFocus::Cancel,
                    ConfirmFocus::Cancel => ConfirmFocus::Confirm,
                };
                EventResult::Consumed(None)
            }
            KeyCode::Enter => {
                let confirmed = self.focused == ConfirmFocus::Confirm;
                if let Some(tx) = self.result_tx.take() { let _ = tx.send(confirmed); }
                let id = self.id.clone();
                EventResult::Consumed(Some(Box::new(move |c| { c.remove(&id); })))
            }
            KeyCode::Esc => {
                if let Some(tx) = self.result_tx.take() { let _ = tx.send(false); }
                let id = self.id.clone();
                EventResult::Consumed(Some(Box::new(move |c| { c.remove(&id); })))
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // ... renders title + message + two buttons ...
    }
}
```

Use:

```rust
let (tx, rx) = oneshot::channel();
compositor.push(Box::new(Dialog::new(
    "confirm-delete",
    ConfirmationDialog::new("Delete?", "Are you sure?", tx),
)));

// Elsewhere, await the result:
tokio::spawn(async move {
    if let Ok(true) = rx.await { /* user confirmed */ }
});
```

### C.2 ThreeButtonDialog

Same pattern, three buttons (`Confirm | Discard | Cancel`), Tab cycles
through them.

### C.3 RoleDialog / CreateSessionDialog

Form-style dialogs. Inner content is a stateful list of fields, each
focusable. Tab/Shift+Tab cycles fields. The dialog's role-listing field
embeds a `VirtualList<RoleId>`.

### C.4 AgentSelector (init-time, standalone)

Currently has a fallback path for when no `<InputProvider>` is present.
In Rust, every binary uses the Compositor, so this fallback disappears.
Just push it onto the Compositor like any other dialog.

### C.5 ConfirmPrompt (inline Y/N)

Currently rendered inline in the composer. In Rust, this is a small
Component that embeds in the layout (not a popup). Its priority is
`Priority::High` (above background, below modals).

### C.6 ThinkingLevelDialog

Custom slider. Implement as a single Component with a small custom
widget (`Bar` from ratatui or hand-rolled). Embed inside a `Dialog`.

### C.7 AttachmentDialog

File-system tree picker. Use `ratatui-explorer` crate as the inner
content; wrap in `Dialog`.

---

## Part D. Test plan

### D.1 Compositor tests (~12 tests)

- Higher priority intercepts first.
- FIFO tiebreak at equal priority.
- `is_active = false` skips handler.
- `Consumed(None)` stops propagation.
- `Consumed(Some(callback))` runs callback.
- `Ignored` propagates to next handler.
- `pop()` removes top layer.
- `remove(id)` removes named layer.
- `update(Action)` fans out across all layers.
- Render order: lowest priority first (so highest paints on top).
- Empty compositor returns `Ignored`.
- All inactive layers return `Ignored`.

### D.2 Dialog base tests (~8 tests)

- ESC pops self.
- Inner component receives non-ESC events.
- Centered rect is correct for given area + dialog size.
- Border color is applied.
- Priority is `Critical`.
- `is_active = false` makes the dialog invisible to events.
- Inner component update is forwarded.
- Modal painting wipes underlying buffer (Clear behaviour).

### D.3 ConfirmationDialog tests (~6 tests)

- Tab cycles focus.
- Enter on Confirm sends `true` via oneshot.
- Enter on Cancel sends `false`.
- ESC sends `false`.
- Initial focus respects builder.
- Render snapshot (TestBackend + insta).

---

## Part E. Migration order

1. **Compositor + Priority + EventResult** (foundation).
2. **`Component` trait** (with default impls).
3. **Tests for the dispatcher.**
4. **Base Dialog wrapper.**
5. **One concrete dialog (ConfirmationDialog) end-to-end with a oneshot
   result channel.**
6. **Each remaining dialog as its own work unit** (parallelizable once
   the base is in place).

This makes `Dialog` and `InputPriority` a single ~5-point work unit
(parts A-B-D) plus one ~3-point work unit per concrete dialog.
