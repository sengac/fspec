# RPC-023 — Mouse handling port spec

**Parent:** RPC-002 (Rust ratatui frontend with dual transport)
**Depends on:** RPC-016 (Per-column scroll viewport — provides the BoardStore
viewport methods and `BoardView.last_viewport_height` this card consumes).

**Supersedes the TODO direction in:**
`spec/attachments/RPC-002/10-multilineinput-and-mouse-port-spec.md` Part B —
that file sketched the replacement of `src/tui/utils/mouseProtocol.ts` with
crossterm but never produced a concrete slice. This card is that slice for
the BoardView columns (the highest-value mouse interaction) plus the
foundational hit-test helper + native text-selection toggle that other
slices (AgentView VirtualList scrollback, MultiLineInput) will reuse.

---

## Why this card exists

The TypeScript TUI has fully working mouse-wheel scrolling for kanban columns
(BUG-131, TUI-010), click-to-focus, and a debounced "let the terminal handle
native text selection" toggle (TUI-078). The current Rust port does NOT —
crossterm's `EnableMouseCapture` is already wired in
`codelet/fspec-tui/src/terminal.rs` (lines 18–23, 66–76) and `Event::Mouse`
is therefore delivered into the event stream, but `App::handle_event`
(`codelet/fspec-tui/src/app/events.rs`) matches ONLY `Event::Key` and silently
drops the mouse variant on the floor.

That is the gap this card closes for the BoardView.

### Prior research applied to this slice

The Rust mouse-handling direction was settled across multiple RPC-002
attachments BEFORE this card was scoped. Every architectural decision
below is traceable to one of them — this slice does not re-litigate
those decisions, only applies them. Cross-reference summary:

| Decision in this card                                         | Source                                                                                                |
|---------------------------------------------------------------|-------------------------------------------------------------------------------------------------------|
| crossterm replaces `mouseProtocol.ts` entirely; no SGR regex  | `RPC-002/03-ratatui-ecosystem-survey.md` §A.5; `06-mapping-ink-to-ratatui.md` §Mouse                  |
| Hit-testing is the app's job — components remember last Rect  | `RPC-002/03-ratatui-ecosystem-survey.md` §A.5 (closing paragraph); `RPC-002/10` Part B.3              |
| `MouseTrackingToggle` pattern: tokio timer + Action + Drop    | `RPC-002/06-mapping-ink-to-ratatui.md` lines 236-270; `RPC-002/10` Part B.2; `RPC-002/12` Slice 02    |
| TerminalGuard owns global EnableMouseCapture lifecycle        | `RPC-002/07-recommended-architecture.md` §1 directory layout                                          |
| `Event::Key(_) \| Event::Mouse(_)` both fan through Compositor| `RPC-002/07-recommended-architecture.md` §2 lines 101-112                                             |
| Rolling our own Compositor instead of rat-event/rat-salsa     | `RPC-002/11-open-questions-and-risks.md` Q2 (RESOLVED 2026-05-08)                                     |
| Codex's "let the terminal handle mouse" approach rejected     | `RPC-002/04-codex-architecture-deep-dive.md` §4 lines 148-162                                         |
| Wheel velocity (150ms/cap-5) deferred to VirtualList slice    | `RPC-002/12-suggested-work-unit-breakdown.md` Slice 04; `RPC-002/11` R2                               |
| Native text-selection toggle 5s debounce timing risk          | `RPC-002/11-open-questions-and-risks.md` R3                                                           |
| Draggable scrollbar thumb ~30 LoC DIY (deferred)              | `RPC-002/03-ratatui-ecosystem-survey.md` §A.2 lines 44-49                                             |
| Drag-to-move popups via tui-popup `mouse_down_on`             | `RPC-002/03-ratatui-ecosystem-survey.md` §B.2 (deferred to RPC-022)                                   |

**Key takeaway:** the TypeScript implementation was low-level *because Ink
exposed mouse only as raw escape strings*. Roughly 90% of
`src/tui/utils/mouseProtocol.ts` simply evaporates in the Rust port —
crossterm gives us `Event::Mouse(MouseEvent { kind, column, row, modifiers })`
pre-parsed. The remaining work is exactly the three things crossterm
doesn't have an opinion about: hit-testing, the native-text-selection
toggle, and routing wheel direction to existing Actions.

---

## 1 · TypeScript reference

### 1.1 SGR-1006 parser (`src/tui/utils/mouseProtocol.ts`)

The TS code defines the protocol primitives that the Rust port REPLACES (it
does NOT re-implement them — crossterm's `Event::Mouse` already carries the
parsed values):

```ts
// src/tui/utils/mouseProtocol.ts
export const MOUSE_ENABLE  = '\x1b[?1000h\x1b[?1006h';
export const MOUSE_DISABLE = '\x1b[?1006l\x1b[?1000l';
export const SGR_MOUSE_RE  = /^\[<(\d+);(\d+);(\d+)([Mm])$/;

export const SGR_BUTTON = {
  LEFT: 0, MIDDLE: 1, RIGHT: 2,
  SCROLL_UP: 64, SCROLL_DOWN: 65,
} as const;

export interface SgrMouseEvent {
  button: number;   // 0/1/2/64/65
  x: number;        // 1-based
  y: number;        // 1-based
  isRelease: boolean; // 'm' vs 'M'
}

export function parseSgrMouse(input: string): SgrMouseEvent | null { ... }
```

**Rust equivalent (no port required — crossterm gives it to us for free):**

| TS surface              | Rust equivalent (crossterm)                  |
|-------------------------|----------------------------------------------|
| `MOUSE_ENABLE`          | `EnableMouseCapture` (already wired)         |
| `MOUSE_DISABLE`         | `DisableMouseCapture` (already wired)        |
| `SGR_BUTTON.LEFT`       | `MouseButton::Left`                          |
| `SGR_BUTTON.MIDDLE`     | `MouseButton::Middle`                        |
| `SGR_BUTTON.RIGHT`      | `MouseButton::Right`                         |
| `SGR_BUTTON.SCROLL_UP`  | `MouseEventKind::ScrollUp`                   |
| `SGR_BUTTON.SCROLL_DOWN`| `MouseEventKind::ScrollDown`                 |
| `parseSgrMouse(input)`  | (none — `Event::Mouse(MouseEvent { … })`)    |
| 1-based `x, y`          | 0-based `column, row` on `MouseEvent`        |

### 1.2 BoardView per-column wheel scroll (`src/tui/components/UnifiedBoardLayout.tsx`)

The canonical TS handler is `handleColumnScroll` (lines 236–245) wired through
the `useInputCompat` background priority handler at lines 282–352:

```tsx
// src/tui/components/UnifiedBoardLayout.tsx
import { SGR_BUTTON, parseSgrMouse } from '../utils/mouseProtocol';

// Lines 236-245 — translate wheel direction into selector movement.
const handleColumnScroll = (direction: 'up' | 'down'): void => {
  if (direction === 'down') onWorkUnitChange?.(1);
  else if (direction === 'up') onWorkUnitChange?.(-1);
};

// Lines 282-352 — useInputCompat handler. The mouse branch is BUG-131.
useInputCompat({
  id: 'unified-board-layout',
  priority: InputPriority.BACKGROUND,
  description: 'Board layout keyboard navigation',
  isActive: !isDialogOpen,
  handler: (input, key) => {
    // BUG-131: Mouse scroll handling via SGR protocol
    const mouseEvent = parseSgrMouse(input);
    if (mouseEvent) {
      if (mouseEvent.button === SGR_BUTTON.SCROLL_UP)   { handleColumnScroll('up');   return true; }
      if (mouseEvent.button === SGR_BUTTON.SCROLL_DOWN) { handleColumnScroll('down'); return true; }
    }
    // … PageUp/Down, Home/End, arrows, [, ], Enter …
  },
});
```

**Key behavioural facts to preserve in the port:**

1. The wheel scrolls the **focused column only** — it does NOT inspect the
   mouse coordinates against per-column rectangles. The user controls which
   column is "active" via Left/Right (`h` / `l`) first, then the wheel.
2. Wheel-up / wheel-down delegate to the **same code path** as the Up / Down
   arrow keys (`onWorkUnitChange?.(±1)`), which means they go through
   `BoardStore::move_selection(±1, viewport_height)` — picking up wrap-around
   AND viewport auto-scroll for free.
3. The TS handler returns `true` (consumed) so the keyboard fall-through never
   reinterprets the same escape sequence.

### 1.3 BoardView mount-time MOUSE_ENABLE (`src/tui/components/BoardView.tsx`)

```tsx
// src/tui/components/BoardView.tsx lines 41, 118-126
import { MOUSE_ENABLE, MOUSE_DISABLE } from '../utils/mouseProtocol';

useEffect(() => {
  // Enable mouse tracking for board view (TUI-010)
  // Disabled on unmount and when entering AgentView, model selector, etc.
  // since those views disable mouse tracking in their cleanup
  process.stdout.write(MOUSE_ENABLE); // BUG-131
  return () => {
    process.stdout.write(MOUSE_DISABLE);
  };
}, []);
```

**Rust equivalent:** the Rust port lives a level up. `TerminalGuard::init`
already runs `EnableMouseCapture` once at app start and the matching
`DisableMouseCapture` runs in `Drop` (codelet/fspec-tui/src/terminal.rs:66–90)
plus inside the panic hook. No per-view enable/disable is needed because the
Rust app never swaps in/out a "different" view that needs different mouse
modes — the alt-screen lifecycle owns it.

### 1.4 VirtualList wheel acceleration + TUI-078 toggle (`src/tui/components/VirtualList.tsx`)

VirtualList (the AgentView scrollback substrate) ALSO consumes wheel events,
with two extras the BoardView does not have:

```tsx
// src/tui/components/VirtualList.tsx lines 504-532 — wheel acceleration
const handleScroll = useCallback((direction: 'up' | 'down'): void => {
  const now = Date.now();
  const timeDelta = now - lastScrollTime.current;
  if (timeDelta < 150) {
    scrollVelocity.current = Math.min(scrollVelocity.current + 1, 5);
  } else {
    scrollVelocity.current = 1;
  }
  lastScrollTime.current = now;
  const delta = (direction === 'down' ? 1 : -1) * scrollVelocity.current;
  // … scroll mode vs item mode dispatch …
}, [...]);

// src/tui/components/VirtualList.tsx lines 540-572 — handler dispatch
const mouseEvent = parseSgrMouse(input);
if (mouseEvent) {
  if (mouseEvent.button === SGR_BUTTON.SCROLL_UP)   { handleScroll('up');   return true; }
  if (mouseEvent.button === SGR_BUTTON.SCROLL_DOWN) { handleScroll('down'); return true; }

  // TUI-078: Button DOWN (Left/Middle/Right) — temporarily disable mouse
  // tracking so the terminal handles native text selection. Restarts the
  // 5-second debounce timer.
  if (!mouseEvent.isRelease && (mouseEvent.button === SGR_BUTTON.LEFT
                            || mouseEvent.button === SGR_BUTTON.MIDDLE
                            || mouseEvent.button === SGR_BUTTON.RIGHT)) {
    temporarilyDisableMouseTracking();
    return true;
  }
  // TUI-078: Button RELEASE — immediately re-enable so the wheel works.
  if (mouseEvent.isRelease && ...) {
    if (mouseTrackingTemporarilyDisabledRef.current) {
      clearReEnableTimer();
      process.stdout.write(MOUSE_ENABLE);
      mouseTrackingTemporarilyDisabledRef.current = false;
    }
    return true;
  }
}
```

The acceleration (`scrollVelocity` 1→5 if subsequent wheel events arrive
within 150 ms) is a UX nicety, and the TUI-078 toggle is what lets users
copy/paste text out of the scrollback with their terminal's native selection.

**Scope split:** this card ports BoardView wheel scroll + the foundational
mouse plumbing. Wheel acceleration + TUI-078 toggle for the AgentView
scrollback land in RPC-019 (multi-line input + VirtualList-style scrollback);
this card builds the `MouseTrackingToggle` helper so RPC-019 can compose it
without re-architecting anything.

### 1.5 AgentView resume-mode wheel scroll (`src/tui/components/AgentView.tsx`)

```tsx
// src/tui/components/AgentView.tsx lines 4555-4573 — resume mode
const mouseEvent = parseSgrMouse(input);
if (mouseEvent) {
  if (isResumeMode) {
    if (mouseEvent.button === SGR_BUTTON.SCROLL_UP)   { navigateResumeByDelta(-1); return true; }
    if (mouseEvent.button === SGR_BUTTON.SCROLL_DOWN) { navigateResumeByDelta( 1); return true; }
  }
  // Otherwise let it propagate to VirtualList (BACKGROUND priority).
  return false;
}
```

Same pattern: wheel → ±1 selector movement. This is a downstream slice; the
plumbing this card builds (Action::ScrollFocusedColumn{Up,Down}, hit-test
helper, MouseTrackingToggle) is the substrate.

---

## 2 · Current Rust state (the gap)

### 2.1 Mouse capture IS enabled — but events are dropped

```rust
// codelet/fspec-tui/src/terminal.rs:66-76
fn enable_terminal_modes() -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(
        out,
        EnterAlternateScreen,
        EnableMouseCapture,    // ← already on
        EnableBracketedPaste
    )?;
    Ok(())
}
```

```rust
// codelet/fspec-tui/src/app/events.rs:164-179 — the run loop already
// receives Event::Mouse but never forwards it.
while !self.should_quit {
    tokio::select! {
        Some(event) = events.next() => {
            let event = event?;
            match event {
                Event::Paste(text)    => { let _ = self.handle_paste(&text); }
                Event::Resize(_, _)   => { self.should_render = true; }
                other                 => { let _ = self.handle_event(&other); }
            }
        }
        // …
    }
}
```

```rust
// codelet/fspec-tui/src/app/events.rs:34-71 — handle_event short-circuits
// on Event::Key only. Event::Mouse falls through compositor + navigator,
// both of which also only match Event::Key, so the event is dropped.
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    // … critical-dialog branch …
    if let Event::Key(key) = event { /* app shortcuts */ }
    let result = self.compositor.handle_event(event);   // no-op for Mouse
    // …
    let nav_result = self.navigator.handle_event(event, &self.board_store);
    // …
}
```

```rust
// codelet/fspec-tui/src/views/board.rs:95-163 — BoardView::handle_event
pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult {
    let Event::Key(key) = event else {
        return EventResult::ignored();  // ← every mouse event ends here
    };
    // …
}
```

### 2.2 RPC-016 already provided the receiving end

```rust
// codelet/fspec-tui/src/store/board_viewport.rs:33-47
pub fn move_selection(&mut self, delta: i32, viewport_height: usize) {
    // … wrap-around + auto-scroll …
}
```

```rust
// codelet/fspec-tui/src/views/board.rs:82-84
pub fn last_viewport_height(&self) -> usize {
    self.last_viewport_height.get() as usize
}
```

```rust
// codelet/fspec-tui/src/components/mod.rs:181-191 — Action variants exist
Action::ScrollFocusedColumnUp(usize),
Action::ScrollFocusedColumnDown(usize),
Action::SelectFirstInFocused,
Action::SelectLastInFocused,
```

```rust
// codelet/fspec-tui/src/app/dispatch.rs:146-167 — dispatch already routes
// SelectNext / SelectPrev through move_selection with last_viewport_height.
Action::SelectNext => {
    let vh = self.navigator.board.last_viewport_height();
    self.board_store.move_selection(1, vh);
}
Action::SelectPrev => {
    let vh = self.navigator.board.last_viewport_height();
    self.board_store.move_selection(-1, vh);
}
```

**Implication:** wheel scroll for BoardView is **literally three lines**:
match `Event::Mouse` in `BoardView::handle_event`, switch on
`MouseEventKind::ScrollUp` / `ScrollDown`, emit `Action::SelectPrev` /
`Action::SelectNext`. All the viewport math, wrap-around, and auto-scroll
re-uses RPC-016's plumbing.

The rest of this document is the rigorous version with hit-testing, native
text-selection toggle scaffolding, and the test plan.

---

## 3 · Proposed Rust port

### 3.1 New module: `codelet/fspec-tui/src/mouse/mod.rs`

A dedicated module so future slices (RPC-019 VirtualList scrollback, RPC-020
slash-command popup, RPC-022 modal dialogs) can compose the helpers without
each re-implementing hit-tests or text-selection toggles.

```rust
//! Mouse subsystem — hit-testing helper, button/wheel translation, and
//! the TUI-078 native-text-selection toggle.
//!
//! Feature: spec/features/rpc023-mouse-handling.feature
//!
//! This module replaces the TypeScript src/tui/utils/mouseProtocol.ts
//! SGR parser (crossterm does the parsing for us) and the
//! MOUSE_ENABLE / MOUSE_DISABLE raw-escape writes (the alt-screen
//! lifecycle in terminal.rs owns those globally).

pub mod hit_test;
pub mod toggle;

pub use hit_test::rect_contains;
pub use toggle::MouseTrackingToggle;
```

### 3.2 `mouse/hit_test.rs`

```rust
//! Rectangle hit-testing for mouse events.
//!
//! Components remember their last-rendered Rect in a Cell<Option<Rect>>
//! field and call rect_contains(rect, event.column, event.row) in
//! handle_event before consuming the mouse event.

use ratatui::layout::Rect;

/// True iff (x, y) lies inside rect (half-open on the right/bottom
/// edge — matches ratatui's Rect::intersects convention).
pub fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_interior_point() {
        let r = Rect { x: 5, y: 5, width: 10, height: 10 };
        assert!(rect_contains(r, 5, 5));
        assert!(rect_contains(r, 14, 14));
        assert!(!rect_contains(r, 15, 14));
        assert!(!rect_contains(r, 14, 15));
        assert!(!rect_contains(r, 4, 5));
    }
}
```

### 3.3 `mouse/toggle.rs` — TUI-078 native text-selection toggle

This is **scaffolding** for RPC-019; this card builds it so it's tested and
ready, but the BoardView slice itself does not yet wire any
button-press/release path through it (the BoardView columns intentionally do
not allow native text selection — the kanban cells are inert labels).

```rust
//! Debounced "let the terminal handle native text selection" toggle.
//!
//! TS reference: src/tui/components/VirtualList.tsx lines 180-234,
//! 540-572 (temporarilyDisableMouseTracking + 5-second re-enable
//! timer + immediate re-enable on button-release).

use std::io::stdout;
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use tokio::sync::mpsc::UnboundedSender;
use tokio::task::JoinHandle;

use crate::components::Action;

/// Re-enable mouse capture this many seconds after the last button
/// down event. Matches the TS setTimeout(..., 5000) in VirtualList.
const REENABLE_AFTER: Duration = Duration::from_secs(5);

/// Coordinates the "DisableMouseCapture during text selection,
/// EnableMouseCapture when the user is done" lifecycle.
pub struct MouseTrackingToggle {
    disabled: bool,
    re_enable_handle: Option<JoinHandle<()>>,
    action_tx: UnboundedSender<Action>,
    owner_id: String,
}

impl MouseTrackingToggle {
    pub fn new(owner_id: impl Into<String>, action_tx: UnboundedSender<Action>) -> Self {
        Self {
            disabled: false,
            re_enable_handle: None,
            action_tx,
            owner_id: owner_id.into(),
        }
    }

    pub fn is_disabled(&self) -> bool { self.disabled }

    /// Button-down handler. Disables capture immediately, (re)schedules
    /// the auto-re-enable timer.
    pub fn temporarily_disable(&mut self) {
        self.cancel_pending_reenable();
        if !self.disabled {
            let _ = execute!(stdout(), DisableMouseCapture);
            self.disabled = true;
        }
        let tx = self.action_tx.clone();
        let owner = self.owner_id.clone();
        self.re_enable_handle = Some(tokio::spawn(async move {
            tokio::time::sleep(REENABLE_AFTER).await;
            let _ = tx.send(Action::ReEnableMouseTracking(owner));
        }));
    }

    /// Button-release handler. Re-enables capture immediately so the
    /// wheel works on the very next event.
    pub fn re_enable(&mut self) {
        self.cancel_pending_reenable();
        if self.disabled {
            let _ = execute!(stdout(), EnableMouseCapture);
            self.disabled = false;
        }
    }

    fn cancel_pending_reenable(&mut self) {
        if let Some(handle) = self.re_enable_handle.take() {
            handle.abort();
        }
    }
}

impl Drop for MouseTrackingToggle {
    fn drop(&mut self) {
        self.cancel_pending_reenable();
        if self.disabled {
            let _ = execute!(stdout(), EnableMouseCapture);
        }
    }
}
```

A new Action variant carries the deferred re-enable back to the App:

```rust
// codelet/fspec-tui/src/components/mod.rs — append to the Action enum.

/// RPC-023: the MouseTrackingToggle's 5-second debounce timer has
/// elapsed for the owner String. App::dispatch forwards this to the
/// owning component so it can call MouseTrackingToggle::re_enable().
ReEnableMouseTracking(String),
```

### 3.4 BoardView: wheel → SelectPrev/SelectNext

```rust
// codelet/fspec-tui/src/views/board.rs — extend handle_event.

use crossterm::event::{Event, KeyCode, KeyModifiers, MouseEvent, MouseEventKind};
use crate::mouse::rect_contains;

pub fn handle_event(&self, event: &Event, store: &BoardStore) -> EventResult {
    // --- RPC-023: mouse branch (BEFORE the existing Event::Key match) ---
    if let Event::Mouse(MouseEvent { kind, column, row, .. }) = event {
        let in_content = self
            .last_content_area
            .get()
            .map(|r| rect_contains(r, *column, *row))
            .unwrap_or(false);
        match kind {
            MouseEventKind::ScrollUp if in_content => {
                self.emit(Action::SelectPrev);
                return EventResult::consumed();
            }
            MouseEventKind::ScrollDown if in_content => {
                self.emit(Action::SelectNext);
                return EventResult::consumed();
            }
            _ => return EventResult::ignored(),
        }
    }
    // --- existing Event::Key handling unchanged ---
    let Event::Key(key) = event else {
        return EventResult::ignored();
    };
    // …
}
```

Side note: `Action::SelectPrev` / `SelectNext` route through
`BoardStore::move_selection(±1, last_viewport_height)` per
`codelet/fspec-tui/src/app/dispatch.rs:146-155` — so the wheel inherits
RPC-016's wrap-around AND viewport auto-scroll without any new code.

### 3.5 BoardView: record last_content_area at render time

```rust
// codelet/fspec-tui/src/views/board.rs

pub struct BoardView {
    pub theme: Arc<Theme>,
    pub action_tx: Option<UnboundedSender<Action>>,
    last_viewport_height: Cell<u16>,
    /// RPC-023: the column-content Rect observed by the most recent
    /// render_with_store call. handle_event reads this when hit-testing
    /// wheel events.
    last_content_area: Cell<Option<Rect>>,
}

// inside render_with_store, right after last_viewport_height.set(...):
self.last_viewport_height.set(split[7].height);
self.last_content_area.set(Some(split[7]));   // ← new
paint_content_rows(split[7], buf, widths, store, &self.theme);
```

### 3.6 Click-to-focus (RECOMMENDED — see §6 item 8)

Originally tagged "defer to a follow-up." The cross-reference review
upgrades this to **recommended-include** because:

- `RPC-002/04-codex-architecture-deep-dive.md` §4 lines 159-161
  explicitly identifies click-to-select as a fspec requirement.
- The hit-test infrastructure is being built anyway.
- `BoardStore::set_focused_column` already exists at
  `store/board.rs:128-132`.
- Without it, mouse users have a half-finished story: wheel scrolls
  the focused column, but column focus still requires keyboard `h`/`l`.

Proposed implementation:

```rust
// codelet/fspec-tui/src/views/board.rs — extend BoardView.

pub struct BoardView {
    // … existing fields …
    /// RPC-023: the column-header Rect for each of the 7 canonical
    /// columns, observed by the most recent render_with_store call.
    /// Indexed by COLUMN_ORDER position.
    last_column_header_areas: Cell<Option<[Rect; 7]>>,
    /// RPC-023: the per-column content Rect (split[7] sliced into 7
    /// vertical strips), observed by the most recent render_with_store.
    /// Used by click-on-content-row hit-testing.
    last_column_content_areas: Cell<Option<[Rect; 7]>>,
}

// handle_event extension (inside the Event::Mouse branch):
MouseEventKind::Down(MouseButton::Left) => {
    // Header click → focus that column.
    if let Some(headers) = self.last_column_header_areas.get() {
        for (idx, rect) in headers.iter().enumerate() {
            if rect_contains(*rect, *column, *row) {
                self.emit(Action::SetFocusedColumn(idx));
                return EventResult::consumed();
            }
        }
    }
    // Content-row click → focus column AND select the row under cursor.
    if let Some(contents) = self.last_column_content_areas.get() {
        for (idx, rect) in contents.iter().enumerate() {
            if rect_contains(*rect, *column, *row) {
                self.emit(Action::SetFocusedColumn(idx));
                let row_in_col = (*row).saturating_sub(rect.y) as usize;
                self.emit(Action::SelectIndexInFocused(row_in_col));
                return EventResult::consumed();
            }
        }
    }
    EventResult::ignored()
}
```

This needs one new Action variant
(`Action::SelectIndexInFocused(usize)`) and a matching
`BoardStore::select_index_in_focused(idx, viewport_height)` that
clamps to the column's work-unit count. Both are trivial.

### 3.6a Horizontal wheel (RECOMMENDED — see §6 item 7)

crossterm delivers `MouseEventKind::ScrollLeft` / `ScrollRight` on
terminals that support them. Two extra match arms, zero new plumbing:

```rust
MouseEventKind::ScrollLeft if in_content => {
    self.emit(Action::FocusPrevColumn);
    return EventResult::consumed();
}
MouseEventKind::ScrollRight if in_content => {
    self.emit(Action::FocusNextColumn);
    return EventResult::consumed();
}
```

The TS port could not implement this because `mouseProtocol.ts` only
parsed the SGR codes for vertical wheel (64 / 65).

### 3.7 Source-shape invariants

- `mouse/mod.rs`, `mouse/hit_test.rs`, `mouse/toggle.rs` each stay < 300 LoC.
- `views/board.rs` stays < 300 LoC (currently 274 — the mouse branch adds
  ~15 lines; if it pushes over, extract the new branch into
  `views/board/mouse.rs`).
- No `parseSgrMouse` / `MOUSE_ENABLE` / `MOUSE_DISABLE` string literals
  appear anywhere in `codelet/` — crossterm owns the protocol.
- `TerminalGuard` is the SINGLE site that calls `EnableMouseCapture` /
  `DisableMouseCapture`; `MouseTrackingToggle` is the only allowed exception
  and it is documented as such in the module-level doc-comment.
- `compositor.rs::handle_event` and `app/events.rs::handle_event` stay
  unchanged — mouse routing is the BoardView's concern, not a new
  compositor-level fan-out.
- **Dialog-priority components match `Event::Key` exclusively** (no
  `Event::Mouse` match arms) until RPC-022 introduces the
  drag-to-move popup story. Enforced by a source-shape test that
  scans every component registered at `Priority::Dialog` /
  `Priority::Critical` for `Event::Mouse` pattern occurrences. See §6
  item 5.

### 3.8 MouseTrackingToggle: writer injection for testability

§3.3 sketched the toggle as calling `execute!(stdout(), …)` directly,
which leaves the test plan unable to assert the actual escape was
written. Revised shape:

```rust
pub struct MouseTrackingToggle<W: Write + Send = std::io::Stdout> {
    writer: W,
    disabled: bool,
    re_enable_handle: Option<JoinHandle<()>>,
    action_tx: UnboundedSender<Action>,
    owner_id: String,
}

impl MouseTrackingToggle<std::io::Stdout> {
    /// Production constructor — writes to the real stdout.
    pub fn with_stdout(
        owner_id: impl Into<String>,
        action_tx: UnboundedSender<Action>,
    ) -> Self {
        Self::new(std::io::stdout(), owner_id, action_tx)
    }
}

impl<W: Write + Send> MouseTrackingToggle<W> {
    pub fn new(
        writer: W,
        owner_id: impl Into<String>,
        action_tx: UnboundedSender<Action>,
    ) -> Self { … }

    pub fn temporarily_disable(&mut self) {
        self.cancel_pending_reenable();
        if !self.disabled {
            let _ = execute!(self.writer, DisableMouseCapture);
            self.disabled = true;
        }
        // … schedule re-enable timer (unchanged) …
    }

    pub fn re_enable(&mut self) {
        self.cancel_pending_reenable();
        if self.disabled {
            let _ = execute!(self.writer, EnableMouseCapture);
            self.disabled = false;
        }
    }
}
```

Tests inject `Vec<u8>` (or a custom `Cursor<Vec<u8>>`) and assert the
exact escape byte sequence — `\x1b[?1000l\x1b[?1006l` for disable,
`\x1b[?1000h\x1b[?1006h` for enable. This converts the §4 test plan's
"unit (with `tokio::time::pause`)" entries from "verify Action sent"
to "verify Action sent AND escape written," which is what
`RPC-002/12-suggested-work-unit-breakdown.md` Slice 02 originally
specified as *"unit (mock `execute!`)"*.

---

## 4 · Test plan

| Scenario                                            | Test type | Test file (proposed)                                            |
|-----------------------------------------------------|-----------|-----------------------------------------------------------------|
| `rect_contains` interior + edge + outside           | unit      | inline in `mouse/hit_test.rs`                                   |
| Wheel-down on content area emits SelectNext         | unit      | `tests/board_mouse_rpc023.rs`                                   |
| Wheel-up on content area emits SelectPrev           | unit      | `tests/board_mouse_rpc023.rs`                                   |
| Wheel outside content area returns Ignored          | unit      | `tests/board_mouse_rpc023.rs`                                   |
| Wheel-down at bottom of column wraps via RPC-016    | unit      | `tests/board_mouse_rpc023.rs` (drives `App::dispatch`)          |
| Wheel-up at top wraps via RPC-016                   | unit      | `tests/board_mouse_rpc023.rs`                                   |
| **Wheel-left in content → FocusPrevColumn** (§3.6a) | unit      | `tests/board_mouse_rpc023.rs`                                   |
| **Wheel-right in content → FocusNextColumn** (§3.6a)| unit      | `tests/board_mouse_rpc023.rs`                                   |
| **Click on column header → SetFocusedColumn** (§3.6)| unit      | `tests/board_mouse_rpc023.rs`                                   |
| **Click on content row → focus col + select row** (§3.6)| unit  | `tests/board_mouse_rpc023.rs`                                   |
| Mouse Event::Mouse no longer dropped in run loop    | unit      | `tests/app_mouse_dispatch_rpc023.rs`                            |
| Toggle disables on Down + writes DisableMouseCapture escape | unit | `tests/mouse_toggle_rpc023.rs` (writer=`Vec<u8>`, `tokio::time::pause`) |
| Toggle re-enable immediately writes EnableMouseCapture escape | unit | `tests/mouse_toggle_rpc023.rs`                              |
| Toggle Drop re-enables (writes escape) if still disabled | unit | `tests/mouse_toggle_rpc023.rs`                                |
| Toggle repeated Down restarts the debounce          | unit      | `tests/mouse_toggle_rpc023.rs`                                  |
| Toggle 5s timer fires `Action::ReEnableMouseTracking(owner)` | unit | `tests/mouse_toggle_rpc023.rs`                              |
| Source-shape: `mouse/` directory, < 300 LoC files   | unit      | `tests/source_shape_rpc023.rs`                                  |
| Source-shape: no SGR strings outside `terminal.rs`  | unit      | `tests/source_shape_rpc023.rs`                                  |
| Source-shape: only `terminal.rs` + toggle call mouse capture | unit | `tests/source_shape_rpc023.rs`                              |
| **Source-shape: Dialog/Critical components no `Event::Mouse` arms** (§3.7) | unit | `tests/source_shape_rpc023.rs`                  |

### 4.1 Sample test — wheel emits SelectNext

```rust
// codelet/fspec-tui/tests/board_mouse_rpc023.rs

use codelet_fspec_tui::components::{Action, EventResult};
use codelet_fspec_tui::store::BoardStore;
use codelet_fspec_tui::theme::Theme;
use codelet_fspec_tui::views::BoardView;
use codelet_rpc_types::WorkUnitInfo;
use crossterm::event::{Event, MouseEvent, MouseEventKind, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

fn wu(id: &str) -> WorkUnitInfo {
    WorkUnitInfo {
        id: id.into(), title: id.into(), work_type: "story".into(),
        status: "backlog".into(), description: None, estimate: None,
        epic: None, attachments: Vec::new(), last_state_change_at: None,
    }
}

#[test]
fn wheel_down_inside_content_area_emits_select_next() {
    let (tx, mut rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    let mut store = BoardStore::default();
    store.replace_work_units((0..20).map(|i| wu(&format!("A-{i}"))).collect());

    // Drive a render so last_content_area is populated.
    let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), &store))
        .unwrap();

    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 5, row: 15,   // inside split[7]
        modifiers: KeyModifiers::NONE,
    });
    let result = view.handle_event(&event, &store);
    assert!(matches!(result, EventResult::Consumed(_)));
    let action = rx.try_recv().expect("Action::SelectNext should be emitted");
    assert!(matches!(action, Action::SelectNext));
}

#[test]
fn wheel_outside_content_area_is_ignored() {
    let (tx, mut rx) = unbounded_channel();
    let view = BoardView::new(Arc::new(Theme::default()), tx);
    let store = BoardStore::default();
    let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
    term.draw(|f| view.render_with_store(f.area(), f.buffer_mut(), &store))
        .unwrap();
    // Row 0 is the top border — never inside the content area.
    let event = Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 5, row: 0,
        modifiers: KeyModifiers::NONE,
    });
    assert!(matches!(view.handle_event(&event, &store), EventResult::Ignored(_)));
    assert!(rx.try_recv().is_err());
}
```

### 4.2 Sample test — App run loop forwards Event::Mouse

```rust
// codelet/fspec-tui/tests/app_mouse_dispatch_rpc023.rs

// Construct App with the embedded backend, drive a single
// Event::Mouse(ScrollDown) through App::handle_event, assert the
// BoardStore's selected_index_for("backlog") advanced by 1.
```

### 4.3 Source-shape test

```rust
// codelet/fspec-tui/tests/source_shape_rpc023.rs

#[test]
fn mouse_module_exists() {
    let src = workspace_root().join("fspec-tui").join("src");
    assert!(src.join("mouse").join("mod.rs").exists());
    assert!(src.join("mouse").join("hit_test.rs").exists());
    assert!(src.join("mouse").join("toggle.rs").exists());
}

#[test]
fn no_sgr_strings_outside_terminal_rs() {
    let src = workspace_root().join("fspec-tui").join("src");
    for path in collect_rs_files(&src) {
        if path.ends_with("terminal.rs") { continue; }
        let body = read_to_string_or_panic(&path);
        let code = strip_rust_comments(&body);
        for needle in ["\\x1b[?1000h", "\\x1b[?1006h", "\\x1b[?1006l", "\\x1b[?1000l"] {
            assert!(!code.contains(needle),
                "{} must not embed raw SGR mouse escapes — use crossterm.",
                path.display());
        }
    }
}

#[test]
fn enable_disable_mouse_capture_only_in_terminal_and_toggle() {
    // EnableMouseCapture / DisableMouseCapture allowed ONLY in
    // src/terminal.rs and src/mouse/toggle.rs. Every other module must
    // go through the toggle helper.
}
```

### 4.4 Feature file (proposed)

`spec/features/rpc023-mouse-handling.feature` — one scenario per row of
the test-plan table. Scenarios should reference RPC-016 in their `Background`
to make the dependency relationship explicit:

```gherkin
Feature: Mouse handling for the BoardView
  As a user of the Rust fspec TUI
  I want to scroll kanban columns with my mouse wheel
  So that I do not need to use Page Up / Page Down for every view shift

  Background:
    Given the App run loop has been started under crossterm with
      EnableMouseCapture in effect (codelet/fspec-tui/src/terminal.rs)
    And the BoardStore exposes the RPC-016 viewport methods

  Scenario: Wheel-down inside a column scrolls the focused column down
    Given the BACKLOG column has 20 work units and viewport_height=10
    And the focused column is BACKLOG and selected_index=0
    When the user wheels down with the cursor inside the column-content area
    Then BoardView emits Action::SelectNext
    And BoardStore.selected_index_for("backlog") becomes 1
  …
```

---

## 5 · Out of scope (later cards)

| Item                                                             | Card     |
|------------------------------------------------------------------|----------|
| VirtualList wheel scroll + acceleration (AgentView scrollback)   | RPC-019  |
| TUI-078 native text-selection toggle WIRED into VirtualList      | RPC-019  |
| BoardView opt-in to TUI-078 button-press (kanban cells)          | RPC-019 (deferred per §6 item 9) |
| MultiLineInput bracketed-paste / textarea mouse drag             | RPC-019  |
| Modal-dialog hit-test (tui-popup drag-to-move uses its own path) | RPC-022  |
| Slash-command popup mouse click selection                        | RPC-020  |
| Resume-mode wheel scroll in AgentView                            | RPC-019  |
| Draggable scrollbar thumb (~30 LoC, see RPC-002/03 §A.2)         | RPC-019 (scrollbar slice) |

The `MouseTrackingToggle` + `rect_contains` helpers built by this card are
the shared foundation those cards will consume.

---

## 6 · Open questions for Example Mapping

These convert directly into red cards once Example Mapping starts. Items
1–4 are the original scope questions; items 5–9 are derived from a
cross-reference review against the full RPC-002 mouse research
(`spec/attachments/RPC-002/03-ratatui-ecosystem-survey.md` §A.5,
`06-mapping-ink-to-ratatui.md` §Mouse, `07-recommended-architecture.md` §1
& §2, `10-multilineinput-and-mouse-port-spec.md` Part B,
`11-open-questions-and-risks.md` Q2/R2/R3, `12-suggested-work-unit-breakdown.md`
Slice 02).

### Original scope questions

1. Do we hit-test the wheel against the column-content area only, or also
   the column-header row? (TS: only inside `handleColumnScroll` — no
   coordinate check; this Rust port proposes a hit-test for cleaner
   propagation to overlaid components.)
2. Should we include the click-to-focus interaction in this card, or defer
   it to keep the diff minimal? See item 8 below — the recommendation has
   been **upgraded to "include it"** based on RPC-002/04 §4's explicit
   call-out that fspec must implement click-to-select properly.
3. Should wheel events SKIP the focused-column check and instead scroll the
   column under the cursor? (TS: no — focused column only. Recommendation:
   match TS; cursor-target scroll is a UX experiment for later.)
4. Does the `ReEnableMouseTracking(String)` Action need to fan back through
   the Compositor (so multiple toggle owners can coexist), or is the App's
   direct lookup of a single owner sufficient for the foreseeable slices?

### Cross-reference review questions (added 2026-05-15)

5. **Dialog-component mouse-event invariant.** `App::handle_event` already
   routes `Event::Mouse` through the Compositor (events.rs:56) per the
   RPC-002/07 §2 design (`Event::Key(_) | Event::Mouse(_)` both routed).
   Existing dialog components (DisconnectDialog, HelpDialog) match only
   `Event::Key` and correctly fall through, but nothing enforces that
   invariant. If RPC-022 later adds drag-to-move via tui-popup's
   `PopupState::mouse_down_on(area)`, mouse-event routing through the
   Compositor must remain predictable.
   - **Proposed red card:** "Should the source-shape suite assert that
     every dialog-priority component matches `Event::Key` exclusively
     (no `Event::Mouse` match arms) UNLESS it explicitly opts in via a
     `MouseAware` marker trait?"
   - **Recommendation:** add a source-shape test for the current set
     (DisconnectDialog, HelpDialog); revisit when RPC-022 lands.

6. **MouseTrackingToggle writer injection for testability.** §3.3 calls
   `execute!(stdout(), DisableMouseCapture)` directly, which means the
   §4 test plan items ("Toggle disables on Down + schedules re-enable",
   "Toggle re-enable immediately on Release") can only assert the
   `Action::ReEnableMouseTracking` side of the round-trip — they cannot
   verify the actual escape was written. RPC-002/12 Slice 02 acceptance
   explicitly notes *"unit (mock `execute!`)"* for these tests.
   - **Proposed red card:** "Does `MouseTrackingToggle::new` take a
     `Box<dyn Write + Send>` writer parameter (or generic `W: Write`),
     or is the toggle hard-bound to `stdout()`?"
   - **Recommendation:** generic over `W: Write + Send` with
     `MouseTrackingToggle::with_stdout()` as the production
     constructor; tests inject `Vec<u8>` and assert the exact escape
     byte sequence. This is the same shape `terminal.rs` should
     eventually adopt for symmetry, but that's a follow-up — this card
     only owns the toggle.

7. **Horizontal wheel events.** crossterm delivers
   `MouseEventKind::ScrollLeft` / `ScrollRight` on terminals that
   support them (kitty, iTerm2 with shift-wheel, some Linux mouse
   drivers). The TS port never had access to these because the SGR
   protocol's button codes for horizontal wheel weren't being parsed by
   `mouseProtocol.ts`. This is a Rust-port-only opportunity.
   - **Proposed red card:** "Should horizontal wheel
     (`MouseEventKind::ScrollLeft` / `ScrollRight`) inside the
     column-content area emit `Action::FocusPrevColumn` /
     `Action::FocusNextColumn`?"
   - **Recommendation:** **yes** — it's two extra match arms, zero new
     plumbing (FocusPrev/Next already exist), and it matches the
     kanban metaphor (vertical wheel scrolls within a column,
     horizontal switches columns). Add to the BoardView test plan.

8. **Promote click-to-focus into this card.** §3.6 currently defers
   click-to-focus, but RPC-002/04 §4 lines 159-161 explicitly identifies
   click-to-select as a fspec requirement (*"fspec must implement
   mouse properly because we have a Kanban board with column-to-column
   hit-testing, click-to-select, etc."*). The hit-test infrastructure
   is being built anyway; `BoardStore::set_focused_column` already
   exists at `store/board.rs:128-132`. Additional cost: ~5 lines of
   code + one test. The current scope leaves users with a half-finished
   mouse story: they can scroll with the wheel but still need keyboard
   `h`/`l` to pick which column is focused.
   - **Proposed red card:** "Does this card include
     click-on-column-header → `Action::SetFocusedColumn(idx)`, or is it
     deferred to a follow-up?"
   - **Recommendation:** **include**. Add
     `last_column_header_areas: Cell<Option<[Rect; 7]>>` alongside
     `last_content_area`, and handle
     `MouseEventKind::Down(MouseButton::Left)` inside each header rect.
     A click on a content row should also focus the column AND set
     `selected_index` to the row under the cursor (one more match arm).

9. **BoardView opt-in to TUI-078 button-press for native text selection.**
   The spec scaffolds `MouseTrackingToggle` (§3.3) but the BoardView slice
   intentionally does not wire any button-press path through it (§3.3
   says *"the kanban cells are inert labels"*). If a user wants to
   copy a work-unit title from a column, they currently cannot —
   crossterm's `EnableMouseCapture` intercepts the press before the
   terminal can begin a selection.
   - **Proposed red card:** "Does the BoardView opt into TUI-078
     button-press → DisableMouseCapture on its content rows, or is
     native text selection strictly an AgentView/VirtualList concern
     (RPC-019)?"
   - **Recommendation:** **no, defer to RPC-019**. Reasons: (a) kanban
     cells are short truncated titles — low copy-paste value; (b) the
     5-second debounce would make click-to-focus (item 8) feel
     laggy; (c) the AgentView scrollback is the true high-value
     copy-paste surface and that's where TUI-078 was tuned. Document
     the decision in the module-level doc-comment of
     `mouse/toggle.rs` so RPC-019 has the rationale on hand.

### Decision capture

Whichever way these resolve, the answers should be captured as Example
Mapping rules + examples + assumptions on RPC-023 (`fspec add-rule`,
`fspec add-example`, `fspec add-assumption`). The MouseTrackingToggle
writer-injection question (item 6) is the only one that **changes the
shape of the code being written**; items 5, 7, 8, 9 are scope additions
that fit cleanly inside the existing module structure described in §3.
