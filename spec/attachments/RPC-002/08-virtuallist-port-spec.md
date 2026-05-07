# 08 — VirtualList Port Specification

This document is the detailed port plan for `src/tui/components/VirtualList.tsx`
(689 LoC) to Rust. It maps each behaviour to a Rust implementation strategy
and references the source crates.

---

## 1. Public API

```rust
// components/virtual_list.rs

pub struct VirtualList<T: ListItem> {
    /// Source-of-truth items (standard mode)
    items: Vec<T>,

    /// Lazy-mode count (overrides items.len() if provided)
    item_count: Option<usize>,

    /// Lazy-mode accessor (mutually exclusive with items)
    get_items: Option<Box<dyn Fn(usize, usize) -> Vec<T> + Send + Sync>>,

    /// Selection
    selected_index: usize,
    scroll_offset: usize,
    selection_mode: SelectionMode,

    /// Group selection
    group_by: Option<Box<dyn Fn(&T) -> GroupId + Send + Sync>>,
    group_by_index: Option<Box<dyn Fn(usize) -> GroupId + Send + Sync>>,
    selected_group_id: Option<GroupId>,
    group_padding_before: u16,

    /// Layout
    fixed_height: Option<u16>,
    reserved_lines: u16,
    height_adjustment: i16,

    /// Behaviour
    enable_wrap_around: bool,
    scroll_to_end: bool,
    user_scrolled_away: bool,
    show_scrollbar: bool,

    /// Mouse-wheel velocity (TUI scroll acceleration)
    last_scroll_time: Option<Instant>,
    scroll_velocity: u32,

    /// Native text-selection mode toggle (TUI-078)
    mouse_disabled: bool,
    re_enable_timer: Option<JoinHandle<()>>,

    /// Identity
    id: String,

    /// Action bus
    action_tx: mpsc::UnboundedSender<Action>,

    /// Callbacks
    on_select: Option<Box<dyn Fn(&T, usize) -> Action + Send + Sync>>,
    on_focus: Option<Box<dyn Fn(&T, usize) -> Action + Send + Sync>>,

    /// Phantom for T
    _marker: PhantomData<T>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode { Item, Scroll }

pub type GroupId = u64; // we hash arbitrary keys to u64

pub trait ListItem: Send + Sync {
    fn render(&self, area: Rect, buf: &mut Buffer, selected: bool, ctx: &RenderContext);
    fn height_hint(&self, width: u16) -> u16 { 1 }
}
```

---

## 2. Builder pattern

```rust
let list = VirtualList::<WorkUnit>::builder("board-todo-column")
    .items(work_units)
    .show_scrollbar(true)
    .selection_mode(SelectionMode::Item)
    .reserved_lines(4)
    .on_select(|item, _idx| Action::OpenWorkUnit(item.id.clone()))
    .build(action_tx);
```

A builder keeps the constructor sane given the ~16 optional fields.

---

## 3. Behaviour-by-behaviour port

### 3.1 Item virtualization

```rust
fn visible_items(&self, area: Rect) -> Vec<&T> /* or owned, lazy mode */ {
    let visible_height = self.visible_height(area);
    let start = self.scroll_offset;
    let end = (start + visible_height as usize).min(self.total_count());

    if let Some(get) = &self.get_items {
        // lazy mode - returns owned Vec
        get(start, end)
    } else {
        self.items[start..end].iter().collect()
    }
}

fn total_count(&self) -> usize {
    self.item_count.unwrap_or(self.items.len())
}

fn is_lazy_mode(&self) -> bool {
    self.get_items.is_some() && self.item_count.is_some()
}
```

### 3.2 Dynamic height measurement (ELIMINATED)

Replace `Yoga measureElement` + `setTimeout(0)` with constraint-based
layout:

```rust
fn visible_height(&self, area: Rect) -> u16 {
    if let Some(h) = self.fixed_height { return h; }
    let h = area.height as i32 + self.height_adjustment as i32;
    h.max(1) as u16
}
```

`reserved_lines` and `height_adjustment` are kept as parameters but
`area.height` already accounts for parent layout. In most cases consumers
will pass `Constraint::Min(0)` and `area.height` will be exactly right -
no adjustment needed. We keep the field for backwards-compat during
migration; expect to delete it after the port is complete.

### 3.3 Selection modes

```rust
fn handle_navigate(&mut self, dir: Direction, key: NavKey) {
    match self.selection_mode {
        SelectionMode::Scroll => self.scroll_navigation(dir, key),
        SelectionMode::Item => {
            if self.has_grouping() {
                self.navigate_to_group(dir);
            } else {
                self.navigate_to(self.selected_index as i64 + dir as i64);
            }
        }
    }
}
```

### 3.4 Group-based selection

```rust
fn group_id(&self, index: usize) -> Option<GroupId> {
    if let Some(f) = &self.group_by_index {
        return Some(f(index));
    }
    if let Some(f) = &self.group_by {
        return self.items.get(index).map(|item| f(item));
    }
    None
}

fn navigate_to_group(&mut self, dir: Direction) {
    let total = self.total_count();
    if total == 0 { return; }

    let current = self.group_id(self.selected_index);

    match dir {
        Direction::Up => {
            // find first item of previous group
            for i in (0..self.selected_index).rev() {
                if self.group_id(i) != current {
                    let prev = self.group_id(i);
                    // find first item in that group
                    for j in (0..=i).rev() {
                        if self.group_id(j) != prev {
                            self.set_selected(j + 1);
                            return;
                        }
                    }
                    self.set_selected(0);
                    return;
                }
            }
        }
        Direction::Down => {
            for i in self.selected_index + 1..total {
                if self.group_id(i) != current {
                    self.set_selected(i);
                    return;
                }
            }
        }
    }
}

/// On items mutation, restore selection to first item with same group ID
fn preserve_group_selection(&mut self) {
    if self.is_lazy_mode() { return; } // parent handles
    let Some(target) = self.selected_group_id else { return };
    let total = self.total_count();
    if total == 0 { return; }
    for i in 0..total {
        if self.group_id(i) == Some(target) {
            self.set_selected(i);
            return;
        }
    }
}
```

### 3.5 scrollToEnd + user-scrolled-away

```rust
fn maybe_auto_scroll_to_end(&mut self, visible_height: u16) {
    if !self.scroll_to_end { return; }
    if self.user_scrolled_away { return; }
    if self.selection_mode == SelectionMode::Item { return; }
    let total = self.total_count();
    if total == 0 { return; }
    let new_offset = (total as i64 - visible_height as i64 + 1).max(0) as usize;
    self.scroll_offset = new_offset;
}

fn detect_user_scrolled_away(&mut self, dir: Direction, max_offset: usize) {
    let at_bottom = self.scroll_offset >= max_offset.saturating_sub(1);
    if dir == Direction::Up && !at_bottom {
        self.user_scrolled_away = true;
    } else if at_bottom {
        self.user_scrolled_away = false;
    }
}
```

### 3.6 Mouse-wheel velocity acceleration

```rust
fn handle_scroll(&mut self, dir: Direction, area: Rect) {
    let now = Instant::now();
    let delta = self.last_scroll_time
        .map(|t| now.duration_since(t).as_millis())
        .unwrap_or(u128::MAX);

    if delta < 150 {
        self.scroll_velocity = (self.scroll_velocity + 1).min(5);
    } else {
        self.scroll_velocity = 1;
    }
    self.last_scroll_time = Some(now);

    let amount = self.scroll_velocity as i64;
    let signed = if dir == Direction::Down { amount } else { -amount };

    // route depending on selection mode
    match self.selection_mode {
        SelectionMode::Scroll => {
            self.set_scroll_offset(self.scroll_offset as i64 + signed);
        }
        SelectionMode::Item => {
            if self.has_grouping() { self.navigate_to_group(dir); }
            else { self.navigate_to(self.selected_index as i64 + signed); }
        }
    }
}
```

### 3.7 Scrollbar (use ratatui core widget)

```rust
fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
    if !self.show_scrollbar { return; }
    let total = self.total_count();
    let visible = self.visible_height(area) as usize;
    if total <= visible { return; }

    let mut state = ScrollbarState::new(total)
        .position(self.scroll_offset)
        .viewport_content_length(visible);

    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .symbols(scrollbar::VERTICAL);

    StatefulWidget::render(scrollbar, area, buf, &mut state);
}
```

The custom `getScrollbarString` cache, the `■`/`│` glyphs, the
`scrollbarCache` map - **all gone**.

### 3.8 Keyboard navigation

```rust
fn handle_key(&mut self, key: &KeyEvent, area: Rect) -> EventResult {
    if self.total_count() == 0 { return EventResult::Ignored(None); }
    if key.modifiers.contains(KeyModifiers::SHIFT) &&
       matches!(key.code, KeyCode::Up | KeyCode::Down) {
        // text selection in terminal - don't capture
        return EventResult::Ignored(None);
    }

    match self.selection_mode {
        SelectionMode::Scroll => self.handle_scroll_navigation(key, area),
        SelectionMode::Item => self.handle_item_navigation(key, area),
    }
}

fn handle_item_navigation(&mut self, key: &KeyEvent, area: Rect) -> EventResult {
    let visible = self.visible_height(area) as usize;
    let total = self.total_count();
    match key.code {
        KeyCode::Up => {
            if self.has_grouping() { self.navigate_to_group(Direction::Up); }
            else { self.navigate_to(self.selected_index as i64 - 1); }
            EventResult::Consumed(None)
        }
        KeyCode::Down => {
            if self.has_grouping() { self.navigate_to_group(Direction::Down); }
            else { self.navigate_to(self.selected_index as i64 + 1); }
            EventResult::Consumed(None)
        }
        KeyCode::PageUp => {
            self.navigate_to(self.selected_index as i64 - visible as i64);
            EventResult::Consumed(None)
        }
        KeyCode::PageDown => {
            self.navigate_to(self.selected_index as i64 + visible as i64);
            EventResult::Consumed(None)
        }
        KeyCode::Home => { self.navigate_to(0); EventResult::Consumed(None) }
        KeyCode::End => { self.navigate_to(total as i64 - 1); EventResult::Consumed(None) }
        KeyCode::Enter => {
            if let Some(cb) = &self.on_select {
                if let Some(item) = self.items.get(self.selected_index) {
                    let action = cb(item, self.selected_index);
                    self.action_tx.send(action).ok();
                }
            }
            EventResult::Consumed(None)
        }
        _ => EventResult::Ignored(None),
    }
}
```

### 3.9 Native text-selection toggle (TUI-078)

```rust
fn handle_mouse(&mut self, ev: &MouseEvent, area: Rect) -> EventResult {
    match ev.kind {
        MouseEventKind::ScrollUp => {
            self.handle_scroll(Direction::Up, area);
            EventResult::Consumed(None)
        }
        MouseEventKind::ScrollDown => {
            self.handle_scroll(Direction::Down, area);
            EventResult::Consumed(None)
        }
        MouseEventKind::Down(_) => {
            self.temporarily_disable_mouse();
            EventResult::Consumed(None)
        }
        MouseEventKind::Up(_) => {
            self.re_enable_mouse();
            EventResult::Consumed(None)
        }
        _ => EventResult::Ignored(None),
    }
}

fn temporarily_disable_mouse(&mut self) {
    if let Some(h) = self.re_enable_timer.take() { h.abort(); }
    let _ = execute!(stdout(), DisableMouseCapture);
    self.mouse_disabled = true;
    let tx = self.action_tx.clone();
    let id = self.id.clone();
    self.re_enable_timer = Some(tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        tx.send(Action::ReEnableMouse(id)).ok();
    }));
}

fn re_enable_mouse(&mut self) {
    if let Some(h) = self.re_enable_timer.take() { h.abort(); }
    if self.mouse_disabled {
        let _ = execute!(stdout(), EnableMouseCapture);
        self.mouse_disabled = false;
    }
}
```

### 3.10 Scroll adjustment to keep selection visible

```rust
fn ensure_selection_visible(&mut self, visible_height: u16) {
    if self.selection_mode != SelectionMode::Item { return; }
    let (range_start, range_end) = self.visible_range_for(self.selected_index);

    if range_start < self.scroll_offset {
        self.scroll_offset = range_start;
    } else if range_end >= self.scroll_offset + visible_height as usize {
        let range_size = range_end - range_start + 1;
        if range_size <= visible_height as usize {
            self.scroll_offset = range_end - visible_height as usize + 1;
        } else {
            self.scroll_offset = range_start;
        }
    }
}

fn visible_range_for(&self, index: usize) -> (usize, usize) {
    if !self.has_grouping() { return (index, index); }
    let Some(group) = self.group_id(index) else { return (index, index); };

    let mut start = index;
    let mut end = index;

    let mut i = index as i64 - 1;
    while i >= 0 {
        if self.group_id(i as usize) == Some(group) {
            start = i as usize;
            i -= 1;
        } else { break; }
    }

    for j in index + 1..self.total_count() {
        if self.group_id(j) == Some(group) { end = j; }
        else { break; }
    }

    let start = start.saturating_sub(self.group_padding_before as usize);
    (start, end)
}
```

---

## 4. Component impl

```rust
impl<T: ListItem + 'static> Component for VirtualList<T> {
    fn priority(&self) -> Priority { Priority::Background }
    fn id(&self) -> &str { &self.id }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        // Note: area passed via Context or remembered from last render
        match event {
            Event::Key(k) => self.handle_key(k, self.last_area),
            Event::Mouse(m) => self.handle_mouse(m, self.last_area),
            _ => EventResult::Ignored(None),
        }
    }

    fn update(&mut self, action: Action) -> Option<Action> {
        if let Action::ReEnableMouse(id) = &action {
            if id == &self.id {
                self.re_enable_mouse();
            }
        }
        None
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.last_area = area;
        let visible_height = self.visible_height(area);

        // Clamp / adjust before render
        if self.total_count() > 0 && self.selected_index >= self.total_count() {
            self.selected_index = self.total_count() - 1;
        }
        self.maybe_auto_scroll_to_end(visible_height);
        self.ensure_selection_visible(visible_height);

        // Empty state
        if self.total_count() == 0 {
            Paragraph::new("No items")
                .style(Style::default().add_modifier(Modifier::DIM))
                .render(area, buf);
            return;
        }

        // Layout: list area + scrollbar area
        let chunks = if self.show_scrollbar {
            Layout::horizontal([
                Constraint::Min(0),
                Constraint::Length(1),  // 1-cell scrollbar gutter
            ]).split(area)
        } else {
            std::rc::Rc::new([area, Rect::ZERO][..]).into()
        };
        let list_area = chunks[0];

        // Render visible items
        let mut y = list_area.y;
        for (visible_idx, item) in self.visible_items(list_area).iter().enumerate() {
            let actual_idx = self.scroll_offset + visible_idx;
            let h = item.height_hint(list_area.width);
            if y + h > list_area.y + list_area.height { break; }
            let item_area = Rect { x: list_area.x, y, width: list_area.width, height: h };
            let selected = self.is_item_selected(actual_idx);
            item.render(item_area, buf, selected, &self.render_ctx());
            y += h;
        }

        // Scrollbar
        if self.show_scrollbar {
            self.render_scrollbar(chunks[1], buf);
        }
    }
}
```

---

## 5. Integration with `tui-widget-list` (alternative path)

If we choose to delegate to `tui-widget-list` rather than rolling the
visible-window logic:

- Implement `ListableWidget` for our `T`.
- Use `ListBuilder` for lazy generation.
- Keep our wrapper layer for: group selection, scroll vs item modes,
  scrollToEnd, mouse-wheel velocity, native text-selection toggle.

Trade-off:
- **Pro:** less code, tested upstream variable-height logic.
- **Con:** another dependency; `ListableWidget` API may not give us
  enough control for group selection (we'd shadow its `selected` state).

Recommendation: **try `tui-widget-list` first.** If group selection
doesn't compose cleanly, fall back to rolling our own (the spec above).

---

## 6. Test plan

| Scenario | Test type | Notes |
|---|---|---|
| Up/Down navigates | unit | construct VirtualList, send `KeyCode::Up`, assert `selected_index` |
| Group navigation skips within-group items | unit | items with shared group ids; assert |
| Selection preserved across mutation by group | unit | mutate `items`; assert `selected_index` updates to first matching group |
| Scroll auto-sticks at end | unit | `scroll_to_end = true`; push items; assert offset |
| User scrolling away detaches stick | unit | scroll up; push items; assert offset frozen |
| Mouse wheel velocity acceleration | unit | send 5 wheel events within 150ms; assert offset moved by 1+2+3+4+5 |
| Native text-selection: press disables, release re-enables | unit | mock execute! and assert sequence |
| Scrollbar renders only when content > viewport | snapshot | TestBackend |
| PageUp/PageDown jumps by visible_height | unit | |
| Wrap-around at boundaries | unit | |
| Lazy mode `getItems` called for visible window only | unit | mock accessor; assert call args |
| Empty state renders message | snapshot | |
| Bordered container `height_adjustment` works | snapshot | |

Total estimate: **~30 test cases, ~600 LoC tests**.
