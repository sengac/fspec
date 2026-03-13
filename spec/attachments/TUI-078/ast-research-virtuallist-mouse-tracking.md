# AST Research: VirtualList Mouse Tracking

**Work Unit:** TUI-078
**Date:** 2026-03-12
**Purpose:** Analyze the current VirtualList implementation to understand where the mouse tracking fix needs to be implemented.

---

## Current Implementation

### File: `src/tui/components/VirtualList.tsx`

### Mouse Tracking Enable/Disable (Lines 180-186)

```tsx
useEffect(() => {
  if (!isFocused) return;
  process.stdout.write('\x1b[?1000h');
  return () => {
    process.stdout.write('\x1b[?1000l');
  };
}, [isFocused]);
```

**Analysis:**
- Mouse tracking (`?1000h`) is enabled when the component is focused
- Tracking is disabled (`?1000l`) on unmount or when `isFocused` becomes false
- **Gap:** No timer cleanup for pending re-enable operations

### Existing Refs (Lines 193-234)

```tsx
const prevSelectionModeRef = useRef(selectionMode);
const selectedGroupIdRef = useRef<string | number | null>(null);
const containerRef = useRef<DOMElement>(null);
const lastScrollTime = useRef<number>(0);
const scrollVelocity = useRef<number>(1);
const measurementScheduled = useRef(false);
```

**Analysis:**
- Multiple refs already exist for various tracking purposes
- **Required addition:** `reEnableMouseRef` for timer management

### Scroll Input Handler (Lines 487-505)

```tsx
useInputCompat({
  id: `virtual-list-scroll-${instanceId}`,
  priority: InputPriority.BACKGROUND,
  description: 'Virtual list mouse scroll',
  isActive: isFocused,
  handler: (input, key) => {
    if (totalItemCount === 0) return false;
    if (input.startsWith('[M')) {
      const buttonByte = input.charCodeAt(2);
      if (buttonByte === 96) { handleScroll('up'); return true; }
      if (buttonByte === 97) { handleScroll('down'); return true; }
    }
    if (key.mouse) {
      if (key.mouse.button === 'wheelDown') { handleScroll('down'); return true; }
      if (key.mouse.button === 'wheelUp') { handleScroll('up'); return true; }
    }
    return false;
  },
});
```

**Analysis:**
- Scroll wheel events (96/97) are handled correctly
- **Bug:** Button-down events (32-34) fall through with `return false` - this means the event is not consumed, but mouse tracking is not disabled, so terminal never gets the click
- **Required addition:** Button-down handling that disables mouse tracking and sets up re-enable timer

---

## Implementation Plan

### 1. Add Timer Ref (after line 234)

```tsx
const reEnableMouseRef = useRef<ReturnType<typeof setTimeout> | null>(null);
```

### 2. Update Mouse Tracking useEffect (lines 180-186)

```tsx
useEffect(() => {
  if (!isFocused) return;
  process.stdout.write('\x1b[?1000h');
  return () => {
    // Clear any pending re-enable timer
    if (reEnableMouseRef.current !== null) {
      clearTimeout(reEnableMouseRef.current);
      reEnableMouseRef.current = null;
    }
    process.stdout.write('\x1b[?1000l');
  };
}, [isFocused]);
```

### 3. Update Scroll Input Handler (lines 487-505)

Add button-down handling after scroll wheel handling:

```tsx
if (input.startsWith('[M')) {
  const buttonByte = input.charCodeAt(2);
  // Scroll wheel - handle normally
  if (buttonByte === 96) { handleScroll('up'); return true; }
  if (buttonByte === 97) { handleScroll('down'); return true; }
  // Button down - disable mouse tracking for native text selection
  if (buttonByte >= 32 && buttonByte <= 34) {
    // Clear any existing timer and restart
    if (reEnableMouseRef.current !== null) {
      clearTimeout(reEnableMouseRef.current);
    }
    process.stdout.write('\x1b[?1000l');
    reEnableMouseRef.current = setTimeout(() => {
      reEnableMouseRef.current = null;
      process.stdout.write('\x1b[?1000h');
    }, 2500);
    return false; // Don't consume - let terminal handle selection
  }
}
```

---

## X10 Mouse Protocol Reference

| buttonByte (charCodeAt(2)) | Event |
|---|---|
| 32 | Left button **down** |
| 33 | Middle button **down** |
| 34 | Right button **down** |
| 35 | Any button **release** |
| 96 | Scroll wheel **up** |
| 97 | Scroll wheel **down** |
| 98 | Scroll wheel **left** (trackpad) |
| 99 | Scroll wheel **right** (trackpad) |

---

## Scope

This change affects **only** `VirtualList.tsx`. The other mouse tracking implementations in `AgentView.tsx` and `BoardView.tsx` must remain unchanged as they legitimately need click events for modal interaction.

---

## Test Strategy

Tests should verify:
1. Scroll wheel events continue to work (no change)
2. Button-down events trigger `?1000l` output
3. Re-enable timer fires after 2500ms
4. Rapid clicks reset the timer
5. Component unmount clears the timer
