# 02 — fspec Ink/React TUI Capability Inventory

This document enumerates every capability of the existing Ink/React TUI
that a Rust ratatui port must preserve. Capabilities are grouped by
subsystem with source references, behaviour summaries, replicate-difficulty
ratings, and dependency notes.

> Source: parallel investigation by agent `d2d49672-e846-4f90-9be8-e1ec2bc41119`,
> using the `DeepSearch` tool against `src/tui/` and `src/components/`.

---

## 1. Input Priority Manager (foundational — port FIRST)

### 1.1 Centralized Input Dispatcher

- **Source:**
  `src/tui/input/InputManager.tsx`,
  `src/tui/input/InputHandlerRegistry.ts`,
  `src/tui/input/InputContext.ts`,
  `src/tui/input/types.ts`,
  `src/tui/components/InputProvider.tsx`
- **Key functions:**
  - `createInputHandlerRegistry()`
  - `register/unregister/getOrderedHandlers`
  - `<InputManager>` (single owner of Ink `useInput`)
- **Behaviour:** A single `useInput` hook owns all stdin events for the
  whole app. Every component registers a handler via the registry; on
  each keystroke the registry returns handlers sorted by descending
  `priority` (with FIFO tiebreak via `registeredAt` counter), and the
  manager walks them, calling each `isActive()` then `handler(input, key)`.
  Returning `true` stops propagation (early return).
- **Difficulty:** **Hard.** The priority/propagation contract must be
  reimplemented faithfully, but ratatui has no input system at all (you
  read crossterm events directly), so the whole pattern is built from
  scratch in Rust as a `Vec<Box<dyn Handler>>` + sorted dispatch
  (Helix-style `Compositor`).
- **Dependencies:** Pure JS data structure + Ink `useInput`. **No React
  reconciliation dependency** in the registry itself.

### 1.2 InputPriority Enum (5 levels)

- **Source:** `src/tui/input/types.ts` lines 27-41
- **Levels:**
  - `CRITICAL = 1000`
  - `HIGH = 800`
  - `MEDIUM = 500`
  - `LOW = 200`
  - `BACKGROUND = 100`
- **Convention:**
  - Modals -> CRITICAL
  - Overlays / HITL -> HIGH
  - Primary text input -> MEDIUM
  - Mode / global shortcuts -> LOW
  - Passive scroll / nav -> BACKGROUND
  - Numbers are conventional anchors - arbitrary numeric priority is allowed.
- **Difficulty:** **Easy.** Direct port to a Rust enum or `pub const`s.
- **Dependencies:** None.

### 1.3 useInputCompat with useLayoutEffect Race-Fix (PROV-095)

- **Source:** `src/tui/input/useInputCompat.ts` (~lines 92-129)
- **Behaviour:** The compat hook registers via `useLayoutEffect` (not
  `useEffect`) so handlers are live **before** the next stdin tick -
  closing a race where a modal opening would drop a keystroke between the
  parent deactivating and the modal registering. Falls back to plain Ink
  `useInput` when no `InputManager` is present (used by standalone
  `AgentSelector` / `ConfirmPrompt` during `fspec init`).
- **Difficulty:** **Moderate.** In Rust, registration is synchronous by
  definition, so the race goes away. The fallback path doesn't apply.
  But the *behaviour contract* (atomic mount -> immediate live registration
  before next event) must be preserved.
- **Dependencies:** Depends on React commit phase. **Goes away in Rust.**

### 1.4 isActive Dynamic Gating

- **Source:** `useInputHandler.ts`, `useInputCompat.ts`
- **Behaviour:** `isActive()` is a **closure called per event**, not at
  registration time - lets parents stay registered while modals open
  (parent uses `isActive: () => !modalOpen`). Cheap gating without
  re-registration.
- **Difficulty:** **Easy.** Rust closure stored on handler.
- **Dependencies:** None.

---

## 2. Mouse Protocol (`src/tui/utils/mouseProtocol.ts`)

### 2.1 SGR Mode 1006 Parser

- **Source:** `src/tui/utils/mouseProtocol.ts` (69 LOC), tests in
  `__tests__/mouseProtocol.test.ts`
- **Key:** `parseSgrMouse(input)` - regex
  `/^\[<(\d+);(\d+);(\d+)([Mm])$/` (post-ESC-strip form)
- **Constants:** `SGR_BUTTON.LEFT = 0`, `MIDDLE = 1`, `RIGHT = 2`,
  `SCROLL_UP = 64`, `SCROLL_DOWN = 65`
- **Returns:** `{ button, x, y, isRelease }` - `M` = press,
  `m` = release; coords 1-based.
- **Difficulty:** **Easy -> eliminated.** Direct regex/parser port; *or*
  use crossterm in Rust which delivers parsed `MouseEvent` directly.
- **Dependencies:** None.

### 2.2 Enable / Disable Sequences (BUG-131)

- **Source:** Same file
- **Constants:**
  - `MOUSE_ENABLE = '\x1b[?1000h\x1b[?1006h'`
  - `MOUSE_DISABLE = '\x1b[?1006l\x1b[?1000l'` (reverse order, asserted in tests)
- **Behaviour:** Components write these directly to `process.stdout` on
  focus / blur. Only mode 1000 (button-event) + 1006 (SGR encoding); **NOT**
  ?1003 motion or ?1002.
- **Difficulty:** **Easy.** crossterm's `EnableMouseCapture` /
  `DisableMouseCapture` covers this idiomatically.
- **Dependencies:** Direct stdout write.

### 2.3 Native Text-Selection Passthrough (TUI-078)

- **Source:** `VirtualList.tsx` lines 181-215, 553-567 - helpers
  `temporarilyDisableMouseTracking`, `scheduleMouseTrackingReEnable`,
  `clearReEnableTimer`
- **Behaviour:** On L/M/R press -> write `MOUSE_DISABLE` and start a
  **5-second** debounce timer for re-enable; subsequent presses reset the
  timer; release re-enables instantly; unmount/blur clears timer.
- **Difficulty:** **Moderate.** Port the timer logic with Tokio. Requires
  platform-correct disable / enable to allow native terminal text
  selection while keeping wheel-scroll on release.
- **Dependencies:** Direct stdout writes + `setTimeout` / `clearTimeout`.

---

## 3. VirtualList (`src/tui/components/VirtualList.tsx`, 689 LoC)

A generic, virtualized list with mouse + keyboard support, group-aware
selection, and dynamic height measurement.

### 3.1 Item Virtualization

- Renders only items in `[scrollOffset, scrollOffset + visibleHeight)`.
- Two data-source modes:
  - **Standard:** `items: T[]` prop, sliced via `items.slice(start, end)`.
  - **Lazy (PERF-004):** `itemCount: number` + `getItems(start, end): T[]`
    - viewport-aware lazy computation for very large lists.
- **Difficulty:** **Moderate.** `tui-widget-list` covers standard mode;
  lazy mode requires a custom `ListItems` trait wrapper in Rust.

### 3.2 Dynamic Height Measurement (Yoga)

- Uses Ink's `measureElement(containerRef.current)` inside a
  `useLayoutEffect` with `setTimeout(0)` to wait for Yoga layout.
- `measurementScheduled.current` guard prevents simultaneous measurements.
- `heightAdjustment: -1 | -2` compensates for Yoga over-measurement when
  inside bordered containers (e.g., `CheckpointViewer`).
- `reservedLines: number = 4` fallback when measurement is unavailable.
- **Difficulty:** **Hard -> eliminated.** ratatui uses constraint-based
  layout (`Layout::vertical([Constraint::Length(n), Constraint::Min(0)])`)
  - no measurement, no `setTimeout`, no Yoga workaround needed.
- **Dependencies:** Yoga (via Ink), React reconciliation phase.

### 3.3 Selection Modes (`'item'` vs `'scroll'`)

- **Item mode:** Up/Down moves selection; viewport auto-scrolls to keep
  selection visible.
- **Scroll mode:** Up/Down moves viewport without changing selection;
  used for read-only browsing of long content.
- Mode transitions trigger different behaviours (e.g., entering item mode
  with `scrollToEnd` selects the last item).
- **Difficulty:** **Easy.** Direct enum + match.

### 3.4 Group-Based Selection (TUI-042/043/044)

- **`groupBy?: (item: T) => string | number`** or
  **`groupByIndex?: (index: number) => string | number`** (lazy mode).
- Navigation moves between groups (up/down jumps to first item of
  prev/next group).
- Selection highlights ALL items in a group (entire turn highlighted).
- **Selection preservation:** when items mutate, selection re-anchors to
  the first item with the same group ID.
- `groupPaddingBefore: number` extends visible range upward (separator
  bars).
- **Difficulty:** **Moderate.** No prior art - direct port (~30 LoC).
- **Dependencies:** None inherent.

### 3.5 scrollToEnd + user-scrolled-away Detection

- **`scrollToEnd: boolean`** auto-sticks viewport to last item.
- `userScrolledAway: boolean` state - when user scrolls up while at
  bottom, we detach; when user reaches bottom again, we re-attach.
- Different rules for scroll mode vs item mode.
- **Difficulty:** **Easy.** Confirmed pattern (tenere uses an `AtomicBool`).

### 3.6 Mouse-Wheel Velocity Acceleration

- `lastScrollTime`, `scrollVelocity` (cap 5).
- Successive wheel events within 150 ms increment velocity; gap > 150 ms
  resets to 1.
- **Difficulty:** **Easy.** Direct port (~20 LoC).

### 3.7 Scrollbar Rendering

- Custom scrollbar component using Unicode square + line characters.
- `scrollbarCache: Map<string, string>` memoises scrollbar strings
  (`${height}-${thumbPos}-${thumbHeight}` key); evicts oldest on cache
  size > 1000.
- Thumb height = `max(1, floor((visibleHeight / itemCount) * scrollbarHeight))`.
- Thumb position = `floor((scrollOffset / itemCount) * scrollbarHeight)`.
- **Difficulty:** **Easy -> eliminated.** ratatui core `Scrollbar` widget
  handles all of this; no caching needed (allocation-free render).

### 3.8 Keyboard Navigation

| Key | Action |
|---|---|
| Up / Down | navigate one item / group |
| PageUp / PageDown | navigate by `visibleHeight` |
| Home | go to index 0 |
| End | go to last index |
| Enter | invoke `onSelect(item, index)` |
| `enableWrapAround` | wrap around at boundaries |

- **Difficulty:** **Easy.** Direct port.

### 3.9 selectionRef Escape Hatch

- **`selectionRef?: MutableRefObject<{ selectedIndex: number }>`** lets
  parent components read the current selection imperatively (used for
  `/expand` slash command in lazy mode where `onSelect` is suppressed).
- **Difficulty:** **Easy.** Replace with a callback or shared `Arc<Mutex>`.

### 3.10 onFocus / onSelect Callbacks

- `onFocus(item, index)` fires every time selection changes (item mode,
  non-lazy mode).
- `onSelect(item, index)` fires on Enter (non-lazy mode).
- Lazy mode delegates both to the parent via `selectionRef` and the
  parent's own input handler.
- **Difficulty:** **Easy.**

### 3.11 useId Instance Keying

- Each VirtualList registers two input handlers
  (`virtual-list-scroll-${instanceId}` and
  `virtual-list-nav-${instanceId}`) so multiple instances coexist.
- **Difficulty:** **Easy -> eliminated.** Each Rust component owns its own
  state; no shared registry collision risk.

---

## 4. Dialog System

### 4.1 Base Dialog (`src/components/Dialog.tsx`, ~75 LoC)

- **Responsibilities:** Centred modal overlay, bordered, padded, black
  background, ESC-to-close, input capture with `InputPriority.CRITICAL`.
- **Composition pattern:** Accepts `children` for content; does NOT
  implement business logic.
- **Difficulty:** **Easy.** `tui-popup` covers rendering;
  `InputPriority::CRITICAL` is enforced by the Compositor.

### 4.2 Concrete Dialog Variants

| File | Purpose | Notable behaviours |
|---|---|---|
| `StatusDialog.tsx` | Status / info panel | Auto-close on action |
| `ConfirmationDialog.tsx` | Yes / No confirm | Default-button focus, ESC = Cancel |
| `ThreeButtonDialog.tsx` | Three-action prompt (e.g., Save / Discard / Cancel) | Focus cycling Tab/Shift+Tab |
| `RoleDialog.tsx` | Role selector for agent | List + free-text input |
| `CreateSessionDialog.tsx` | Session creation form | Multi-field form |
| `AgentSelector.tsx` | Agent picker (used during `fspec init` standalone) | Falls back to plain Ink `useInput` |
| `ConfirmPrompt.tsx` | Y/N inline prompt | Standalone (no InputManager) |
| `ThinkingLevelDialog.tsx` | Slider for thinking effort | Custom slider widget |
| `AttachmentDialog.tsx` | Attachment file picker | File-system tree |

- **Pattern:** All use `<Dialog>` + custom content + their own input handler
  at HIGH or CRITICAL priority.
- **Difficulty:** **Moderate.** Each dialog is a thin Component over
  `tui-popup` with its own state struct.

### 4.3 ESC Capture

- Base `Dialog` registers `dialog-esc` handler at CRITICAL priority.
- Returns `true` for `key.escape`, otherwise `false` (lets nested dialog
  content handle other keys).
- **Difficulty:** **Easy.** Standard Compositor pattern.

---

## 5. MultiLineInput (`src/tui/components/MultiLineInput.tsx`)

A terminal-UI rich text editor.

### 5.1 Core Editing

- Cursor: row + column, with column-clamp on row change.
- Insert / delete / backspace / delete-word (Ctrl+W).
- Enter inserts newline (or submits, depending on mode).
- Shift+Enter always inserts newline.
- Word-wrap based on terminal width.

### 5.2 History (Up / Down at top / bottom of input)

- Input history stack persisted across sessions.
- Up at top of input -> previous history entry.
- Down at bottom of input -> next history entry (or clear).

### 5.3 Slash Command Palette

- `/` at column 0 opens `SlashCommandPalette` (popup over input).
- Filtered by typed text; arrow keys navigate; Enter selects.

### 5.4 File Mention Popup

- `@` opens `FileSearchPopup` with fuzzy file search.
- Closes on Esc; selecting inserts `@filename`.

### 5.5 Bracketed Paste

- Detects bracketed-paste sequences.
- Treats paste atomically (one history entry, one undo step).

### 5.6 Multi-line Compaction (UX-002)

- Input area auto-grows up to a max height; beyond that, scrolls
  internally.
- VirtualList re-measures when input grows / shrinks.

- **Difficulty:** **Hard.** `tui-textarea` covers core editing + undo +
  word-wrap. Slash palette / file mention / history / bracketed paste are
  ~200 LoC of custom glue on top.

---

## 6. Layout & Measurement

### 6.1 useTerminalSize Hook

- Subscribes to `process.stdout`'s `'resize'` event.
- Returns `{ width, height }`, updates on SIGWINCH.
- **Difficulty:** **Easy -> eliminated.** ratatui's draw frame provides
  `Frame::area()` directly.

### 6.2 Yoga Flex Layout (Ink-native)

- Every Ink `<Box>` is a Yoga node.
- `flexDirection`, `flexGrow`, `flexShrink`, `width`, `height`,
  `padding`, `margin`, `borderStyle` etc.
- `position: absolute` for modal overlays (Dialog uses this).
- **Difficulty:** **Hard -> mapped, not ported.** ratatui has no Yoga;
  layout uses `Layout::vertical/horizontal([Constraint::Length(n) |
  Percentage(p) | Min(n) | Max(n) | Ratio(a, b) | Fill(weight)])`.
  Most fspec layouts translate directly:
  - `flexGrow: 1` -> `Constraint::Min(0)` or `Constraint::Fill(1)`
  - `width: 50` -> `Constraint::Length(50)`
  - `width: '50%'` -> `Constraint::Percentage(50)`
  - `position: absolute` -> `Clear` widget over a centred sub-rect.

### 6.3 measureElement (only used in VirtualList)

- See section 3.2.
- **Difficulty:** Eliminated.

---

## 7. Other Complex Stateful Components

| Component | What makes it non-trivial | Difficulty |
|---|---|---|
| `BoardView.tsx` | Multi-column Kanban with focus traversal across columns; each column uses VirtualList; supports drag-to-reorder via keyboard. | Hard |
| `AgentView.tsx` | Real-time streaming chat with tool-call cards; uses VirtualList in lazy mode + scrollToEnd + group selection. | Hard |
| `CheckpointViewer.tsx` | Diff-style viewer with `heightAdjustment: -2` for bordered VirtualList. | Moderate |
| `FileSearchPopup.tsx` | Fuzzy search popup; popup-over-input modal pattern. | Moderate |
| `SlashCommandPalette.tsx` | Filtered command list above input. | Easy |
| `UnifiedBoardLayout.tsx` | Complex nested layout combining board + side pane + footer. | Hard |
| `ConversationInputArea.tsx` | Composer + status + agent badges + thinking indicator. | Hard |
| `ChangedFilesViewer.tsx` | Filesystem watcher integration with live updates. | Moderate |
| `ProviderSettingsScreen.tsx` | Multi-step form with provider-specific fields. | Moderate |
| `CustomModelFormView.tsx` | Inline editable form. | Moderate |
| `BlocklistListView.tsx` | List with item-level actions. | Easy |
| `WorkUnitMetadata.tsx` / `WorkUnitAttachments.tsx` | Read-only renderers. | Easy |

---

## 8. Cross-Cutting

### 8.1 InputProvider Tree Wrapping

- `<InputProvider>` wraps the entire app, owning the registry context.
- Children use `useInputCompat()` to register handlers.
- **Difficulty:** Replaced by app-level `Compositor`.

### 8.2 Scrollbar Cache (memoization)

- Module-level `Map<string, string>` with FIFO eviction.
- **Difficulty:** Eliminated.

### 8.3 Keybinding Shortcuts UI

- `KeybindingShortcuts.tsx` renders a footer hint bar.
- **Difficulty:** Easy. Pure render.

---

## Summary Difficulty Distribution

| Difficulty | Count |
|---|---|
| Easy | 19 |
| Moderate | 13 |
| Hard | 7 |
| Eliminated by ratatui / crossterm | 8 |

The **port-critical hard items** are:
1. Input priority Compositor (foundation - port first).
2. VirtualList total composition (the largest single component).
3. MultiLineInput on `tui-textarea` (heaviest editor).
4. BoardView, AgentView, UnifiedBoardLayout, CheckpointViewer
   (large consumers of VirtualList).
