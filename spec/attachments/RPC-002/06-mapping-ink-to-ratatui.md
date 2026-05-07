# 06 — Direct Mapping: Ink/React → ratatui

This is a construct-by-construct translation guide. Whenever a developer
hits an Ink/React idiom in the existing codebase and wonders "what is the
ratatui equivalent?", look here.

---

## React reconciliation -> immediate-mode redraw

Ink/React render is *retained-mode*: the framework owns a virtual DOM and
diffs it against the previous tree. Each component returns JSX; React
patches the terminal to match.

ratatui is *immediate-mode*: every frame, the app calls
`terminal.draw(|frame| { ... })` and re-renders the entire UI from
scratch. Widgets are values, not retained nodes.

**Practical implication:** All caching, memoisation, `useMemo`,
`useCallback`, and React-key tricks **disappear**. You re-render
everything every frame. ratatui is fast enough that this is fine for
most TUIs.

**Exception:** when an item is expensive to compute (syntect highlight,
layout-heavy paragraph), cache the **rendered `Vec<Line>`** keyed by
content hash. See oatmeal's `BubbleCacheEntry` in `05-prior-art-...md`.

---

## Component / Hook -> Component trait + struct

```typescript
// Ink
function MyComponent({ items }: { items: Item[] }) {
  const [selected, setSelected] = useState(0);
  useInputCompat({
    id: 'my-comp',
    priority: InputPriority.MEDIUM,
    handler: (input, key) => {
      if (key.upArrow) { setSelected(s => Math.max(0, s - 1)); return true; }
      return false;
    },
  });
  return <Box>...</Box>;
}
```

```rust
// ratatui (using our Compositor pattern)
pub struct MyComponent {
    items: Vec<Item>,
    selected: usize,
}

impl Component for MyComponent {
    fn priority(&self) -> Priority { Priority::Medium }

    fn handle_event(&mut self, event: &Event, ctx: &mut Context) -> EventResult {
        match event {
            Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {
                self.selected = self.selected.saturating_sub(1);
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored(None),
        }
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        // immediate-mode draw based on self.items / self.selected
    }
}
```

---

## Hook reference table

| Ink/React hook | ratatui equivalent | Notes |
|---|---|---|
| `useState<T>(initial)` | `T` field on the component struct | Mutate via `&mut self`. |
| `useRef<T>(initial)` | `T` field on the component struct | Same as above. No DOM refs in ratatui. |
| `useEffect(fn, [deps])` | Manual: in `update(action)`, dispatch on action types | The "deps changed" semantics aren't free; you derive them. |
| `useLayoutEffect(fn, [deps])` | Same as `useEffect` - no render-phase distinction in immediate mode | The race the layout-effect version solves doesn't exist. |
| `useMemo(fn, [deps])` | Cache field + manual invalidation, or call the function each frame | Often unnecessary - re-compute. |
| `useCallback(fn, [deps])` | Just a regular method | No closure-equality concern. |
| `useId()` | Not needed - struct identity replaces DOM IDs | |
| `useReducer(reducer, initial)` | `update(action) -> Option<Action>` method on Component | Essentially the same pattern. |
| `useContext(Context)` | Pass a `&Context` parameter to `handle_event` / `update` / `render` | Or store an `Arc<RwLock<...>>` on the component. |

---

## Layout

| Ink JSX | ratatui |
|---|---|
| `<Box flexDirection="row">{children}</Box>` | `Layout::horizontal([...]).split(area)` -> render each child into its sub-rect |
| `<Box flexDirection="column">` | `Layout::vertical([...]).split(area)` |
| `<Box flexGrow={1}>` | `Constraint::Min(0)` or `Constraint::Fill(1)` |
| `<Box flexGrow={2} flexShrink={0}>` | `Constraint::Fill(2)` |
| `<Box width={50}>` | `Constraint::Length(50)` |
| `<Box width="50%">` | `Constraint::Percentage(50)` |
| `<Box height={3}>` (in a column) | `Constraint::Length(3)` |
| `<Box padding={1}>` | `Block::default().padding(Padding::uniform(1))` then render child into the inner area |
| `<Box marginLeft={2}>` | Manual: split with `Constraint::Length(2)` for spacer + `Min(0)` for content |
| `<Box borderStyle="round">` | `Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)` |
| `<Box borderColor="red">` | `Block::default().borders(...).border_style(Style::default().fg(Color::Red))` |
| `<Box backgroundColor="black">` | Render `Clear` or a styled `Block` over the rect first |
| `<Box position="absolute" width="100%" height="100%" justifyContent="center" alignItems="center">` | Compute centered sub-rect with `Layout::vertical([Min(0), Length(h), Min(0)])` then `Layout::horizontal(...)`; render `Clear` then content |
| `<Text>{value}</Text>` | `Paragraph::new(value)` or `Span::raw(value)` inside a `Line` |
| `<Text bold color="red">` | `Span::styled(value, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))` |
| `<Text dimColor>` | `Style::default().add_modifier(Modifier::DIM)` |

### Centered modal helper

The Ink `<Dialog>` does:

```jsx
<Box position="absolute" width="100%" height="100%"
     justifyContent="center" alignItems="center" flexDirection="column">
  <Box flexDirection="column" borderStyle="round" padding={1} backgroundColor="black">
    {children}
  </Box>
</Box>
```

ratatui equivalent (~10 LoC):

```rust
fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let h = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(height),
        Constraint::Min(0),
    ]).split(area);
    let v = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(width),
        Constraint::Min(0),
    ]).split(h[1]);
    v[1]
}

// In render:
let popup_area = centered_rect(frame.area(), 60, 12);
frame.render_widget(Clear, popup_area);                        // wipe under
frame.render_widget(
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(Color::Black)),
    popup_area,
);
let inner = popup_area.inner(Margin::new(1, 1));               // padding=1
// render dialog content into `inner`
```

(Or just use `tui-popup`, which wraps this pattern.)

---

## Styling

| Ink prop | ratatui equivalent |
|---|---|
| `color="red"` | `Style::default().fg(Color::Red)` |
| `backgroundColor="black"` | `Style::default().bg(Color::Black)` |
| `bold` | `Modifier::BOLD` |
| `italic` | `Modifier::ITALIC` |
| `underline` | `Modifier::UNDERLINED` |
| `dimColor` | `Modifier::DIM` |
| `inverse` | `Modifier::REVERSED` |

---

## Input handling

| Ink/React | ratatui |
|---|---|
| `useInput((input, key) => { ... })` | `Component::handle_event(&mut self, event: &Event)` returning `EventResult` |
| `useInputCompat({ id, priority, isActive, handler })` | Same, plus `Component::priority()` and `Component::is_active()` |
| `key.upArrow` | `KeyEvent { code: KeyCode::Up, .. }` |
| `key.return` | `KeyCode::Enter` |
| `key.escape` | `KeyCode::Esc` |
| `key.tab` / `key.shift && key.tab` | `KeyCode::Tab` / `KeyCode::BackTab` (or `Tab` with `KeyModifiers::SHIFT`) |
| `key.ctrl && input === 'c'` | `KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }` |
| Returning `true` from handler (stop propagation) | Returning `EventResult::Consumed(None)` |
| Returning `false` (let other handlers run) | Returning `EventResult::Ignored(None)` |

---

## Mouse

| Ink (parsed via `mouseProtocol.ts`) | ratatui (via crossterm) |
|---|---|
| `parseSgrMouse(input)` | crossterm parses internally; you receive `Event::Mouse(MouseEvent { kind, row, column, modifiers })` |
| `SGR_BUTTON.LEFT` (= 0) press | `MouseEventKind::Down(MouseButton::Left)` |
| `SGR_BUTTON.LEFT` release | `MouseEventKind::Up(MouseButton::Left)` |
| `SGR_BUTTON.MIDDLE` (= 1) | `MouseButton::Middle` |
| `SGR_BUTTON.RIGHT` (= 2) | `MouseButton::Right` |
| `SGR_BUTTON.SCROLL_UP` (= 64) | `MouseEventKind::ScrollUp` |
| `SGR_BUTTON.SCROLL_DOWN` (= 65) | `MouseEventKind::ScrollDown` |
| `MOUSE_ENABLE` raw write | `crossterm::execute!(stdout, EnableMouseCapture)` |
| `MOUSE_DISABLE` raw write | `crossterm::execute!(stdout, DisableMouseCapture)` |
| Coords 1-based | Coords 0-based |

**The entire `src/tui/utils/mouseProtocol.ts` file (and its tests)
disappears.**

---

## Lifecycle

| Ink/React | ratatui |
|---|---|
| Component mount | Component struct constructed |
| Component unmount | `Drop` impl runs (rare; usually we just stop rendering it) |
| `useEffect(() => () => cleanup(), [])` cleanup | `Drop::drop` |
| Effect re-run on deps change | Manual re-fire in `update(action)` |

---

## Async work

| Ink/React | ratatui + tokio |
|---|---|
| `setTimeout(fn, ms)` | `tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(ms)).await; ... })` |
| `setInterval(fn, ms)` | `tokio::spawn(async move { let mut iv = tokio::time::interval(Duration::from_millis(ms)); loop { iv.tick().await; ... } })` |
| `clearTimeout(handle)` | Drop the `JoinHandle`, or use a `CancellationToken` |
| `Promise<T>` | `Future<Output = T>` |
| `await fetch(...)` | `await reqwest::get(...)` |

For one-shot deferred work (like the `setTimeout(0)` after Yoga layout):
ratatui has no equivalent need. The render is synchronous; do the work
inline.

For the **5-second mouse-tracking re-enable timer** (TUI-078):

```rust
struct VirtualList {
    mouse_disabled: bool,
    re_enable_timer: Option<tokio::task::JoinHandle<()>>,
    action_tx: mpsc::Sender<Action>,
}

impl VirtualList {
    fn temporarily_disable_mouse(&mut self) {
        // cancel existing timer
        if let Some(h) = self.re_enable_timer.take() { h.abort(); }
        execute!(stdout(), DisableMouseCapture).ok();
        self.mouse_disabled = true;
        // schedule re-enable
        let tx = self.action_tx.clone();
        self.re_enable_timer = Some(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            tx.send(Action::ReEnableMouse).await.ok();
        }));
    }
}

// In `update(Action::ReEnableMouse)`:
fn update(&mut self, action: Action) -> Option<Action> {
    if let Action::ReEnableMouse = action {
        if self.mouse_disabled {
            execute!(stdout(), EnableMouseCapture).ok();
            self.mouse_disabled = false;
        }
    }
    None
}
```

---

## Refs to imperative children

| Ink/React | ratatui |
|---|---|
| `useRef<{ selectedIndex: number }>` passed to child as `selectionRef` | `Arc<RwLock<SelectionState>>` shared between parent and child, OR a `mpsc::Sender<Action>` callback |

The Rust idiom is to lift state into the parent and pass `&mut` access
down on each render call - but for an event-driven action bus, sending an
`Action::SelectionChanged(index)` is cleaner.

---

## Provider / Context

| Ink/React | ratatui |
|---|---|
| `<InputProvider>` wrapping app | `App` owns a `Compositor` field |
| `<ThemeProvider>` | `App` owns a `Theme` struct, passed by `&Theme` to render calls |
| `useContext(MyContext)` | Pass `&Context` to `handle_event` / `update` / `render` |

---

## Error boundaries

Ink doesn't really have these (you'd wrap children in try/catch).
ratatui doesn't either - errors propagate via `Result` up to the main
loop, which can choose to log + redraw or exit.

---

## Test mapping

| Ink testing | Rust testing |
|---|---|
| `ink-testing-library`'s `render(<App />)` | `ratatui::backend::TestBackend` + `Terminal::new` |
| `lastFrame()` | `terminal.backend().buffer()` snapshot |
| `stdin.write(...)` | Send `Event::Key(...)` directly to the Compositor in unit tests |
| Snapshot tests | `insta` crate for buffer snapshots |
