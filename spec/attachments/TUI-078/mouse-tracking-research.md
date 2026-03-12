# Mouse Tracking vs Text Selection — Research Notes

**Work Unit:** TUI-078  
**Date:** 2026-03-12  
**Question:** How do other coding agents/TUI frameworks handle the conflict between mouse scroll wheel and native terminal text selection?

---

## The Fundamental Problem

When application mouse tracking (`?1000h` or stronger) is active, the terminal hands **all** mouse button events to the app via stdin — and stops performing native text selection. There is **no protocol mode that delivers only scroll wheel events while leaving click-and-drag to the terminal's native selector**. This is a hard protocol limitation.

fspec's current implementation enables `?1000h` (X10 button event tracking) in three places:

| Component | File | When Enabled |
|---|---|---|
| `VirtualList` | `src/tui/components/VirtualList.tsx` L182 | When `isFocused` prop is `true` (default = always) |
| `AgentView` | `src/tui/components/AgentView.tsx` L1583 | When `showModelSelector`, `showSettingsTab`, or `isResumeMode` is true |
| `BoardView` | `src/tui/components/BoardView.tsx` L122 | When `viewMode === 'board'` |

Since `VirtualList` has `isFocused = true` as its default and wraps the entire conversation area, mouse tracking is active essentially 100% of the time in the main view — which is exactly where users most want to copy AI output.

---

## Projects Investigated

### 1. pi-mono (`badlogic/pi-mono`)

**Strategy: No mouse tracking at all.**

The `ProcessTerminal.start()` function never writes any mouse tracking escape sequence. The only sequences sent at startup are:

| Sequence | Purpose |
|---|---|
| `\x1b[?2004h` | Bracketed paste mode |
| `\x1b[?u` | Query Kitty keyboard protocol support |
| `\x1b[>7u` | Enable Kitty protocol (flags 1+2+4) |
| `\x1b[>4;2m` | Fallback: xterm `modifyOtherKeys` mode 2 |
| `\x1b[?2026h/l` | Synchronized output (flicker-free rendering) |
| `\x1b[?25l/h` | Hide/show cursor |

No `?1000h`, `?1002h`, `?1006h` — nothing. The terminal retains full native mouse ownership.

**How scrolling works:** Keyboard only. Arrow keys, Page Up/Down. `SelectList` tracks `selectedIndex` and computes a visible window. A `(3/10)` indicator shows scroll position.

**The `StdinBuffer` still parses mouse sequences defensively** — in case a parent process (like tmux with mouse mode on) forwards them — but the app itself never requests them.

**Verdict:** Native text selection works perfectly. Trade-off: no scroll wheel at all.

---

### 2. opencode (`anomalyco/opencode`) — via opentui

**Strategy: Go *harder* on mouse tracking, then implement your own selection system entirely.**

opencode is built on **opentui** — a native TUI core written in Zig with TypeScript bindings. opentui enables `?1003h` (all-motion tracking, even without button press) + `?1006h` (SGR extended coordinates), which is far more aggressive than `?1000h`.

Because the terminal forwards every single mouse event, opentui:

1. **Tracks all mouse drag coordinates** in the Zig native core using a hit grid system
2. **Renders its own selection highlight overlay** — the app itself draws the visual selection highlight, not the terminal
3. **Writes selected text to clipboard** via **OSC 52** (`\x1b]52;c;<base64-text>\x07`)
4. Exposes this via `selectable` JSX prop on text components and `useSelectionHandler` hook

```typescript
// opencode/opentui API
import { useSelectionHandler } from "@opentui/solid"
const App = () => {
  useSelectionHandler((selection) => {
    console.log("Selected:", selection)
  })
  return <text selectable>Select me!</text>
}
```

**The catch:** This completely replaces the terminal's native selection with a home-grown one. The open bug tracker tells the story:
- Issue #574: "currently we can't highlight the text generated in the TUI" (took a long time to ship)
- Issue #1210: "Mouse text selection has unexpected behavior" (can't scroll while selecting, wrong coordinates, copies immediately on release)
- Issue #15760: "Mouse selection is very unreliable and drives me nuts (vscode terminal)"
- Issue #16778: "Escape key doesn't dismiss dialogs after opening via click on selectable text"

This required building an entire native Zig rendering core to implement. It is still buggy.

**Verdict:** Technically achieves both scroll and selection simultaneously, but at enormous complexity cost, and it's still unreliable.

---

### 3. vtcode (`vinhnx/vtcode`)

The shallow clone contained zero source files. No usable research.

---

## Industry Consensus (bubbletea, ratatui, GDB TUI)

### bubbletea (Go TUI framework, 40k+ stars)
[Issue #162](https://github.com/charmbracelet/bubbletea/issues/162) — "Allow both native text selection and mouse wheel scrolling" — is an open issue. No solution has been found at the protocol level. The framework makes you choose: mouse events (breaks selection) or no mouse events (selection works, no scroll wheel).

### GDB TUI (authoritative reference implementation)
[GDB's official docs](https://sourceware.org/gdb/current/onlinedocs/gdb.html/TUI-Mouse-Support.html) say explicitly:

> *"The TUI itself does not directly support copying/pasting with the mouse. However, on Unix terminals, you can typically **press and hold the SHIFT key** on your keyboard to temporarily bypass GDB's TUI and access the terminal's native mouse copy/paste functionality. Alternatively, to disable mouse support in the TUI entirely and give the terminal control over mouse clicks, turn off the `tui mouse-events` setting."*

GDB added `set tui mouse-events off` as a toggle specifically for users who want native selection over TUI scroll. This was added as a patch in Feb 2023 after years of complaints.

---

## X10 Mouse Protocol Button Byte Reference

In `?1000h` (X10 mode), the mouse event encoding in the 3rd byte of `ESC [ M <btn> <x> <y>` is:

| `buttonByte` (charCodeAt(2)) | Event |
|---|---|
| `32` | Left button **down** |
| `33` | Middle button **down** |
| `34` | Right button **down** |
| `35` | Any button **release** |
| `96` | Scroll wheel **up** |
| `97` | Scroll wheel **down** |
| `98` | Scroll wheel **left** (trackpad horizontal) |
| `99` | Scroll wheel **right** (trackpad horizontal) |

Scroll events (96–99) and click events (32–34) are **distinguishable at the byte level**.

---

## The Solution

Since scroll bytes (96–99) and button-down bytes (32–34) are distinguishable, we can:

1. Keep `?1000h` active → scroll wheel events keep arriving → `VirtualList` keeps scrolling ✅
2. When a **button-down** (32–34) arrives — user wants to click/drag to select — immediately write `?1000l`
3. The terminal takes over and handles the entire drag natively ✅
4. Re-enable `?1000h` after a timeout (e.g. 2–3s), covering any reasonable selection drag

```typescript
if (input.startsWith('[M')) {
  const buttonByte = input.charCodeAt(2);

  // Scroll wheel — handle normally, always
  if (buttonByte === 96) { handleScroll('up'); return true; }
  if (buttonByte === 97) { handleScroll('down'); return true; }

  // Button down — hand control to the terminal for native text selection
  if (buttonByte >= 32 && buttonByte <= 34) {
    process.stdout.write('\x1b[?1000l'); // release mouse to terminal
    setTimeout(() => process.stdout.write('\x1b[?1000h'), 2500); // re-enable after selection
    return false; // don't consume — let terminal have it
  }
}
```

### Trade-offs

- **Scroll wheel still works** during normal reading ✅
- **Native text selection works** when you click-and-drag ✅
- **~2.5s window after a click** where scroll wheel doesn't work — acceptable, user is selecting/copying not scrolling
- **Long drags (>2.5s)** would re-enable tracking mid-drag — can increase timeout or detect Ctrl+C/Cmd+C to re-enable instead

### Scope

This logic belongs **only in the main conversation `VirtualList`** handler, where click events currently go unconsumed anyway. The `AgentView` and `BoardView` handlers register at higher priority and legitimately need click events for modal navigation — those should remain unchanged.

---

## Options Comparison

| Approach | Scroll Wheel | Text Selection | Complexity |
|---|---|---|---|
| **Current fspec** (`?1000h` always-on) | ✅ | ❌ broken | Medium |
| **pi-mono style** (no tracking) | ❌ keyboard only | ✅ native | Low |
| **Disable on button-down** ← **recommended** | ✅ | ✅ native | Low |
| **opentui style** (own selection system) | ✅ | ✅ but buggy | Very High |
| **GDB `mouse-events off` toggle** | ❌ | ✅ | Low (config) |

---

## Codebase Intelligence (from DeepSearch)

### How `?1000h` is enabled per-component

#### VirtualList.tsx — conditioned on `isFocused`

```tsx
// Lines ~180–186
useEffect(() => {
  if (!isFocused) return;
  process.stdout.write('\x1b[?1000h');
  return () => {
    process.stdout.write('\x1b[?1000l');
  };
}, [isFocused]);
```

#### AgentView.tsx — conditioned on overlay screens being open

```tsx
// Lines ~1579–1589
useEffect(() => {
  if (showModelSelector || showSettingsTab || isResumeMode) {
    process.stdout.write('\x1b[?1000h');
    return () => {
      process.stdout.write('\x1b[?1000l');
    };
  }
}, [showModelSelector, showSettingsTab, isResumeMode]);
```

#### BoardView.tsx — conditioned on `viewMode === 'board'`

```tsx
// Lines ~117–127
useEffect(() => {
  if (viewMode === 'board') {
    process.stdout.write('\x1b[?1000h');
    return () => {
      process.stdout.write('\x1b[?1000l');
    };
  }
}, [viewMode]);
```

### Input handler priority chain (mouse-relevant handlers)

```
Terminal stdin → Ink useInput → InputManager → InputHandlerRegistry (sorted by priority desc)

  Priority 800 (HIGH)       agent-view-pause        returns false for all mouse
  Priority 200 (LOW)        agent-view-main          handles 96/97 in isResumeMode only; else returns false
  Priority 100 (BACKGROUND) virtual-list-scroll-*   handles 96/97; button-down 32–34 currently ignored
  Priority 100 (BACKGROUND) virtual-list-nav-*       guard: returns false for all [M input immediately
  Priority 100 (BACKGROUND) unified-board-layout     handles 96/97 → column scroll (board mode only)
```

When the user is in the main conversation view (not a modal overlay, not board mode):
- `agent-view-main` sees the button-down first, does nothing, returns `false`
- `virtual-list-scroll-*` receives it — **this is where the fix goes**
- Currently returns `false` without disabling tracking → terminal never gets the click → no native selection

### Why AgentView and BoardView must NOT be changed

Both `AgentView` (model selector, settings tab, resume list) and `BoardView` (column navigation) legitimately need their `?1000h` enabled for modal interaction. Their mouse tracking is scoped to separate `useEffect` instances. The VirtualList `?1000h` is only active for the main conversation view. Changing AgentView or BoardView would break click navigation in those overlays.

### Exact change site

**File:** `src/tui/components/VirtualList.tsx`  
**Handler id:** `virtual-list-scroll-${instanceId}`  
**Handler priority:** `InputPriority.BACKGROUND` (100)

Current scroll handler (simplified):

```tsx
useInputCompat({
  id: `virtual-list-scroll-${instanceId}`,
  priority: InputPriority.BACKGROUND,
  isActive: isFocused,
  handler: (input, key) => {
    if (totalItemCount === 0) return false;
    if (input.startsWith('[M')) {
      const buttonByte = input.charCodeAt(2);
      if (buttonByte === 96) { handleScroll('up');   return true; }
      if (buttonByte === 97) { handleScroll('down'); return true; }
      // ← button-down 32–34 falls through here with return false; BUG
    }
    if (key.mouse) { /* Ink parsed path — scroll only */ }
    return false;
  },
});
```

Required addition after the 96/97 cases:

```tsx
// Button down — hand control to the terminal for native text selection
if (buttonByte >= 32 && buttonByte <= 34) {
  if (reEnableMouseRef.current !== null) {
    clearTimeout(reEnableMouseRef.current);
  }
  process.stdout.write('\x1b[?1000l');
  reEnableMouseRef.current = setTimeout(() => {
    reEnableMouseRef.current = null;
    process.stdout.write('\x1b[?1000h');
  }, 2500);
  return false; // do NOT consume — terminal needs to see the click for native selection
}
```

### Required ref for timer management

```tsx
const reEnableMouseRef = useRef<ReturnType<typeof setTimeout> | null>(null);
```

This must be defined at the top of the component (alongside existing refs like `lastScrollTime` and `scrollVelocity`).

The existing `useEffect` cleanup that writes `?1000l` on unmount/blur should also clear this timer:

```tsx
useEffect(() => {
  if (!isFocused) return;
  process.stdout.write('\x1b[?1000h');
  return () => {
    if (reEnableMouseRef.current !== null) {
      clearTimeout(reEnableMouseRef.current);
      reEnableMouseRef.current = null;
    }
    process.stdout.write('\x1b[?1000l');
  };
}, [isFocused]);
```

### No shared mouse parsing utility exists

The `buttonByte` pattern (`input.charCodeAt(2)`) is copy-pasted verbatim in three files:

| File | Context |
|---|---|
| `VirtualList.tsx` | Scroll handler |
| `AgentView.tsx` | Resume list scroll |
| `UnifiedBoardLayout.tsx` | Column scroll |

The fix is fully self-contained within VirtualList. No shared utility needs to be extracted for this change.

### Dual parse paths (raw vs Ink)

Every scroll handler checks **both** `input.startsWith('[M')` (raw escape sequence path) and `key.mouse` (Ink's parsed mouse object). The `?1000l`/`?1000h` disable/re-enable only needs to be in the raw path — `key.mouse` with a button value of `left`/`middle`/`right` (not `wheelUp`/`wheelDown`) would be the Ink-parsed equivalent for completeness, but in practice Ink may not always populate `key.mouse` for button-down events, so the raw path is the reliable one.

---

## References

- [bubbletea issue #162 — Allow both native text selection and mouse wheel scrolling](https://github.com/charmbracelet/bubbletea/issues/162)
- [GDB TUI Mouse Support docs](https://sourceware.org/gdb/current/onlinedocs/gdb.html/TUI-Mouse-Support.html)
- [GDB patch: add `set tui mouse-events off`](https://sourceware.org/pipermail/gdb-patches/2023-February/196491.html)
- [opencode issue #1210 — Mouse text selection unexpected behavior](https://github.com/anomalyco/opencode/issues/1210)
- [opencode issue #7926 — Add option to disable mouse capture](https://github.com/anomalyco/opencode/issues/7926)
- [opencode issue #15760 — Mouse selection unreliable](https://github.com/anomalyco/opencode/issues/15760)
- [opentui docs — useSelectionHandler](https://opentui.com/docs/bindings/solid/)
- [opentui DeepWiki — Mouse Events](https://deepwiki.com/anomalyco/opentui/6.2-mouse-events)
- [xterm.js issue #4903 — ?1003h vs ?1006h selection clearing behaviour](https://github.com/xtermjs/xterm.js/issues/4903)
- [terminalguide — Mouse Click and Dragging Tracking ?1002](https://terminalguide.namepad.de/mode/p1002/)
- [llxprt-code issue #861 — Ink UI: in-app selection/copy mode with mouse tracking enabled](https://github.com/vybestack/llxprt-code/issues/861)
- [xterm control sequences reference](https://www.invisible-island.net/xterm/ctlseqs/ctlseqs.html)
