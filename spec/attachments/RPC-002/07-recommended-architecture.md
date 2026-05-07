# 07 — Recommended Architecture

This document proposes the foundational architecture for the Rust ratatui
port. It is a synthesis of:

- The ratatui-org `templates/component` skeleton
- Helix's `Compositor` + `EventResult` pattern
- Codex's `tokio::select!` event loop and `mpsc<AppEvent>` bus
- lazygit's `ContextKind` enum (mapped to our `InputPriority`)

The goal is to keep dependencies thin and stay close to bare ratatui.
We add ~50 LoC of glue (the Compositor) instead of adopting a heavier
framework like `tui-realm` or `rat-salsa`.

---

## 1. Crate Layout

The ratatui-side of fspec will be a separate Rust crate (or workspace
of crates) consumed by the existing TypeScript host via tarpc over the
embedded transport (see `rpc-002-feasibility.md`).

```
codelet/
  fspec-tui/                    # the new ratatui crate
    src/
      main.rs                   # bin: fspec-tui (debug standalone)
      lib.rs                    # exports for embedding
      app/
        mod.rs                  # App struct, run loop
        compositor.rs           # input dispatcher (priority stack)
        event.rs                # AppEvent enum
        action.rs               # Action enum (mpsc bus)
      tui/
        mod.rs                  # terminal init/teardown
        backend.rs              # CrosstermBackend wrapper
      components/
        mod.rs                  # `trait Component`
        virtual_list.rs         # port of VirtualList.tsx
        dialog.rs               # base Dialog wrapper over tui-popup
        multi_line_input.rs     # port of MultiLineInput.tsx
        scrollbar.rs            # ratatui Scrollbar + drag thumb
        ...
      views/
        board.rs                # BoardView equivalent
        agent.rs                # AgentView equivalent
        ...
      input/
        priority.rs             # Priority enum
        keybindings.rs          # key map definitions
      mouse/
        mod.rs                  # mouse-capture toggle, hit-test helpers
      theme.rs                  # color / style constants
      transport/
        embedded.rs             # tarpc embedded handle
        websocket.rs            # tarpc WS handle
```

---

## 2. Top-level App

```rust
// app/mod.rs
pub struct App {
    compositor: Compositor,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
    should_quit: bool,
    should_render: bool,
}

impl App {
    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = ratatui::init();
        let mut event_stream = crossterm::event::EventStream::new();
        let mut render_interval = tokio::time::interval(
            Duration::from_millis(16) // ~60fps cap
        );

        loop {
            tokio::select! {
                Some(Ok(event)) = event_stream.next() => {
                    self.handle_terminal_event(event).await?;
                }
                Some(action) = self.action_rx.recv() => {
                    self.handle_action(action).await?;
                }
                _ = render_interval.tick(), if self.should_render => {
                    terminal.draw(|frame| self.render(frame))?;
                    self.should_render = false;
                }
            }
            if self.should_quit { break; }
        }

        ratatui::restore();
        Ok(())
    }

    fn handle_terminal_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(_) | Event::Mouse(_) => {
                let result = self.compositor.handle_event(&event);
                if let EventResult::Consumed(Some(callback)) = result {
                    callback(&mut self.compositor);
                }
                self.should_render = true;
            }
            Event::Resize(_, _) => self.should_render = true,
            Event::Paste(text) => self.compositor.handle_paste(&text),
            _ => {}
        }
        Ok(())
    }

    fn handle_action(&mut self, action: Action) -> Result<()> {
        if let Action::Quit = action { self.should_quit = true; return Ok(()); }
        // Fan out to compositor / individual components
        if let Some(follow_up) = self.compositor.update(action) {
            self.action_tx.send(follow_up).ok();
        }
        self.should_render = true;
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        self.compositor.render(frame.area(), frame.buffer_mut());
    }
}
```

---

## 3. The Compositor (priority dispatcher)

```rust
// app/compositor.rs

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background = 100,
    Low = 200,
    Medium = 500,
    High = 800,
    Critical = 1000,
}

pub enum EventResult {
    Ignored(Option<Box<dyn FnOnce(&mut Compositor) + Send>>),
    Consumed(Option<Box<dyn FnOnce(&mut Compositor) + Send>>),
}

pub trait Component: Send {
    fn priority(&self) -> Priority { Priority::Medium }
    fn is_active(&self) -> bool { true }
    fn id(&self) -> &str { "anonymous" }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        EventResult::Ignored(None)
    }

    fn update(&mut self, action: Action) -> Option<Action> { None }

    fn render(&mut self, area: Rect, buf: &mut Buffer);
}

pub struct Compositor {
    layers: Vec<Box<dyn Component>>,
    /// FIFO tiebreak counter (mirrors Ink's `registeredAt`).
    registration_order: u64,
}

impl Compositor {
    pub fn push(&mut self, component: Box<dyn Component>) {
        self.layers.push(component);
        self.registration_order += 1;
        // re-sort by (priority desc, registration_order asc)
        self.sort();
    }

    pub fn pop(&mut self) -> Option<Box<dyn Component>> { self.layers.pop() }

    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        for layer in self.layers.iter_mut().rev() {
            if !layer.is_active() { continue; }
            let result = layer.handle_event(event);
            if matches!(result, EventResult::Consumed(_)) {
                return result;
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
        for layer in self.layers.iter_mut() {
            // Each layer renders in priority order (bottom layer first).
            // Modals layer painted last = on top.
            layer.render(area, buf);
        }
    }

    fn sort(&mut self) {
        // Sort low-to-high priority so iter_mut().rev() yields highest first.
        // Among equal priorities, FIFO (later-registered first)
        self.layers.sort_by(|a, b| {
            a.priority().cmp(&b.priority())
        });
    }
}
```

### Why this matches Ink's InputPriority manager

- **5 levels via enum:** `CRITICAL`, `HIGH`, `MEDIUM`, `LOW`, `BACKGROUND`.
  Same anchors.
- **Sort-by-priority-then-FIFO:** `sort_by` + `registration_order`
  counter mirrors `registeredAt`.
- **Stop-propagation-on-true:** `EventResult::Consumed` short-circuits.
- **isActive dynamic gating:** `is_active(&self)` is called per event.
- **Modal capture:** push a modal at `Priority::Critical`; it sits at
  the top after sort and gets first dibs.

### Why we don't need `useLayoutEffect`

Push to the Compositor is synchronous. The next event sees the new
layer immediately. The race the `useLayoutEffect` fix solved is gone.

---

## 4. Action Bus

```rust
// app/action.rs

#[derive(Clone, Debug)]
pub enum Action {
    Quit,
    Resize(u16, u16),

    // Mouse subsystem
    ReEnableMouse,

    // VirtualList actions
    ListSelectionChanged(String /* component id */, usize /* index */),
    ListItemSelected(String, usize),

    // Dialog actions
    DialogClose(String /* dialog id */),
    DialogConfirm(String, ConfirmValue),

    // App-level
    LoadWorkUnits,
    WorkUnitsLoaded(Vec<WorkUnit>),

    // ... many more
}
```

The `Action` enum is the Redux-action equivalent. Cloneable, debuggable,
all behaviour-changing events flow through it. Components can return
`Some(Action::...)` from `update` to chain effects.

---

## 5. Component Composition Idioms

### Modal opening

```rust
// In a parent component's handle_event:
if matches!(event, key("/")) {
    let palette = SlashCommandPalette::new(self.action_tx.clone());
    return EventResult::Consumed(Some(Box::new(move |c| {
        c.push(Box::new(palette));
    })));
}
```

The optional callback in `EventResult` defers the mutation to after
event handling completes (avoids borrow-checker fights with iterating
`self.layers`).

### Modal self-dismiss (ESC)

```rust
// In Dialog::handle_event:
if matches!(event, key(Esc)) {
    return EventResult::Consumed(Some(Box::new(|c| {
        c.pop();  // remove self
    })));
}
```

### Modal returning a value

```rust
// Dialog stores an Option<Sender<T>> and on submit:
if let Some(tx) = self.result_tx.take() {
    tx.send(Confirmed::Yes).ok();
}
return EventResult::Consumed(Some(Box::new(|c| { c.pop(); })));
```

---

## 6. Theme

```rust
// theme.rs
pub struct Theme {
    pub fg: Color,
    pub bg: Color,
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub dim: Style,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
    // ...
}

impl Theme {
    pub fn dark() -> Self { /* ... */ }
    pub fn light() -> Self { /* ... */ }
}
```

Pass `&Theme` by reference everywhere (or store an `Arc<Theme>` on the
App). Theme is read-only at runtime.

---

## 7. Transport boundary

The ratatui frontend talks to fspec via tarpc (per
`rpc-002-feasibility.md`). The transport layer is decoupled from the UI:

```rust
// transport/mod.rs
#[async_trait]
pub trait FspecBackend: Send + Sync {
    async fn list_work_units(&self) -> Result<Vec<WorkUnit>>;
    async fn create_session(&self, role: &str) -> Result<SessionId>;
    async fn send_message(&self, id: SessionId, msg: String) -> Result<()>;
    // ... mirror of the JS API surface
}

// Two implementations:
pub struct EmbeddedBackend(/* tarpc in-process handle */);
pub struct WebSocketBackend(/* tarpc WS client */);
```

App holds an `Arc<dyn FspecBackend>`; components call methods on it via
`tokio::spawn` and post `Action::*Loaded(...)` back through the action
bus.

---

## 8. Test strategy

| Layer | Backend | Library |
|---|---|---|
| Compositor unit tests | none | plain `#[test]` |
| Component unit tests | `TestBackend` | `ratatui::backend::TestBackend` + `insta` snapshots |
| Integration | full app + mock `FspecBackend` | `ratatui::TestTerminal` + `insta` |
| E2E | actual `tarpc` + actual terminal | `microsoft/tui-test` (via PTY) - same harness used for the existing Ink tests |

Snapshot the buffer (`Vec<String>` of cells) for visual regression
testing. `insta` makes diffs ergonomic.

---

## 9. Comparison to alternatives we considered

### vs `tui-realm`

`tui-realm`'s focus-based event routing fights our priority model.
Adopting it means hacking subscriptions to approximate priority - more
work than a 30-LoC Compositor, with a heavier dep.

### vs `rat-salsa`

`rat-event`'s `Dialog` qualifier is a great direct mapping. But adopting
the whole `rat-salsa` family (`rat-event`, `rat-widget`, `rat-focus`,
`rat-popup`, `rat-dialog`, `rat-text`, `rat-window`, `rat-theme`,
...) is essentially adopting another framework. It is a viable choice -
just not what we recommend by default.

### vs raw ratatui without any Compositor

Possible but messy. Every component would need to know about every other
component to decide whether to handle an event. The Compositor pays for
itself the moment we have 3+ overlapping handlers (which we do already
in fspec).

### vs porting the Ink design verbatim

Doesn't work. React lifecycle and ratatui's immediate mode are too
different. Trying to mimic `useEffect` in Rust leads to combinatorial
state machines.

---

## 10. Recommended starting commit

A first slice that proves the architecture:

1. `cargo new fspec-tui` from the `templates/component` skeleton.
2. Implement `Compositor` + `Priority` + `EventResult`.
3. Implement a tiny `App` with two components: a "Hello" centered text
   at `Priority::Background`, and an ESC-to-dismiss `Dialog` at
   `Priority::Critical`.
4. Verify priority routing + action bus + render loop work.
5. Snapshot tests via `insta`.

That should fit in a single 5-point work unit and establishes the
foundation for everything else.
