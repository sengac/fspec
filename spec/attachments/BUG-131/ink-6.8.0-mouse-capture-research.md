# Ink 6.8.0 Mouse Capture Breakage — Root Cause Analysis

## Summary

Upgrading from **ink 6.4.0** to **ink 6.8.0** broke all mouse scroll wheel handling across the TUI. The root cause is a new `input-parser.ts` introduced in ink 6.8.0 that splits raw stdin data into individual CSI escape sequences before delivering them to `useInput` handlers. This parser is incompatible with the **X10 mouse protocol** used by our components.

## Background: Mouse Protocol Formats

### X10 Mouse Protocol (`\x1b[?1000h`)

Format: `ESC [ M <button+32> <x+32> <y+32>`

- The 3 trailing bytes (button, x, y) are **raw bytes**, NOT standard CSI parameter encoding
- Button codes: `32`=left click, `33`=middle click, `34`=right click, `35`=release, `96`=scroll up, `97`=scroll down
- Example scroll-up at position (1,1): `\x1b[M\x60\x21\x21` (hex: `1b 5b 4d 60 21 21`)

### SGR Extended Mouse Protocol (`\x1b[?1006h`)

Format: `ESC [ < button ; x ; y M` (press) or `ESC [ < button ; x ; y m` (release)

- All data is encoded as **decimal digits and semicolons** within standard CSI parameter bytes (0x30-0x3F)
- Button codes: `0`=left click, `1`=middle click, `2`=right click, `64`=scroll up, `65`=scroll down
- Example scroll-up at position (10,20): `\x1b[<64;10;20M`

## Root Cause: ink 6.8.0's New Input Parser

### The commit

Ink commit `4848547` — _"Fix dropped keypresses when multiple inputs arrive in one readable tick"_ added `src/input-parser.ts`.

### What changed

**Old flow (ink 6.4.0):**
```
App.tsx handleReadable():
  while ((chunk = stdin.read()) !== null) {
    this.handleInput(chunk);        // ← Raw chunk passed directly
    this.internal_eventEmitter.emit('input', chunk);
  }
```

**New flow (ink 6.8.0):**
```
App.tsx handleReadable():
  while ((chunk = stdin.read()) !== null) {
    const inputEvents = inputParserRef.current.push(chunk);  // ← Split into events!
    for (const input of inputEvents) {
      emitInput(input);
    }
  }
```

### How the input parser splits CSI sequences

The parser (`src/input-parser.ts`) identifies CSI sequences using standard VT terminal grammar:

```typescript
const isCsiParameterByte = (byte: number): boolean => byte >= 0x30 && byte <= 0x3f;
const isCsiIntermediateByte = (byte: number): boolean => byte >= 0x20 && byte <= 0x2f;
const isCsiFinalByte = (byte: number): boolean => byte >= 0x40 && byte <= 0x7e;
```

For X10 mouse: `\x1b[M\x60\x21\x21`:
- `ESC` `[` → CSI start
- `M` (0x4D) → **CSI final byte** (0x40-0x7E range) → sequence terminates here!
- `\x60 \x21 \x21` → split into separate events

Result: `"\x1b[M"` + `"\x60"` + `"\x21"` + `"\x21"` — four separate events instead of one.

### ESC stripping compounds the problem

In `use-input.ts`, after `parseKeypress()`, there's an ESC-stripping step:
```typescript
if (input.startsWith('\u001B')) {
  input = input.slice(1);
}
```

So `"\x1b[M"` → `"[M"` (only 2 characters). The handler code doing `input.charCodeAt(2)` gets `NaN`.

### Trace comparison

**Before (ink 6.4.0):**
```
stdin chunk: \x1b[M\x60\x21\x21 (6 bytes, one event)
→ ESC stripped → "[M`!!" (5 chars)
→ handler: input.startsWith('[M') ✓
→ handler: input.charCodeAt(2) → 96 (scroll up) ✓
```

**After (ink 6.8.0):**
```
stdin chunk: \x1b[M\x60\x21\x21 (6 bytes)
→ input parser splits → "\x1b[M" (3 bytes) + 3 separate byte events
→ Event 1: ESC stripped → "[M" (2 chars)
→ handler: input.startsWith('[M') ✓
→ handler: input.charCodeAt(2) → NaN ✗ BROKEN!
```

### SGR protocol survives the parser

For SGR: `\x1b[<64;10;20M`:
- `ESC` `[` → CSI start
- `<` (0x3C) → CSI parameter byte ✓
- `6` `4` `;` `1` `0` `;` `2` `0` → all CSI parameter bytes ✓
- `M` (0x4D) → CSI final byte → sequence terminates, BUT all data is already captured!

Result: `"\x1b[<64;10;20M"` — **one complete event**.

After ESC strip: `"[<64;10;20M"` — all button/x/y data intact.

## The `key.mouse` Fallback is Dead Code

Several components have a secondary check:
```typescript
if (key.mouse) {
  if (key.mouse.button === 'wheelDown') { ... }
}
```

Ink's `Key` type has **never** had a `mouse` property (confirmed in the `.d.ts` for both 6.4.0 and 6.8.0). TypeScript compilation now correctly reports these as errors:
```
error TS2339: Property 'mouse' does not exist on type 'Key'.
```

This was always dead code — it was probably planned but never implemented.

## Affected Files

### Production code (must change)

| File | Changes needed |
|------|----------------|
| `src/tui/components/VirtualList.tsx` | Switch to SGR protocol. Replace X10 byte parsing with SGR regex parsing. Remove `key.mouse` dead code. |
| `src/tui/components/AgentView.tsx` | Switch enable/disable escape codes. Replace X10 byte parsing with SGR regex parsing. Remove `key.mouse` dead code. |
| `src/tui/components/UnifiedBoardLayout.tsx` | Replace X10 byte parsing with SGR regex parsing. Remove `key.mouse` dead code. |
| `src/tui/components/BoardView.tsx` | Switch enable/disable escape codes to include SGR. |
| `src/tui/components/MultiLineInput.tsx` | Update mouse detection guard. Remove `key.mouse` check. |

### Test files (must update expectations)

| File | Changes needed |
|------|----------------|
| `src/tui/components/__tests__/VirtualList-native-text-selection.test.tsx` | Update `?1000h`/`?1000l` assertions to include `?1006h`/`?1006l`. |

## Implementation Plan

### 1. Create shared mouse protocol constants

Create a small utility module `src/tui/utils/mouseProtocol.ts`:

```typescript
/** Enable SGR extended mouse protocol (button events + SGR encoding) */
export const MOUSE_ENABLE = '\x1b[?1000h\x1b[?1006h';

/** Disable SGR extended mouse protocol */
export const MOUSE_DISABLE = '\x1b[?1006l\x1b[?1000l';

/** SGR mouse event regex: ESC [ < button ; x ; y M/m (after ESC stripping: [<button;x;yM/m) */
export const SGR_MOUSE_RE = /^\[<(\d+);(\d+);(\d+)([Mm])$/;

/** SGR button codes */
export const SGR_BUTTON = {
  LEFT: 0,
  MIDDLE: 1,
  RIGHT: 2,
  SCROLL_UP: 64,
  SCROLL_DOWN: 65,
} as const;

/** Parse an SGR mouse event from input string (after ESC stripping by ink) */
export function parseSgrMouse(input: string): {
  button: number;
  x: number;
  y: number;
  isRelease: boolean;
} | null {
  const match = SGR_MOUSE_RE.exec(input);
  if (!match) return null;
  return {
    button: parseInt(match[1], 10),
    x: parseInt(match[2], 10),
    y: parseInt(match[3], 10),
    isRelease: match[4] === 'm',
  };
}
```

### 2. Update each affected file

Replace all instances of:
- `\x1b[?1000h` → `MOUSE_ENABLE`
- `\x1b[?1000l` → `MOUSE_DISABLE`
- `input.startsWith('[M')` + `charCodeAt(2)` → `parseSgrMouse(input)`
- `key.mouse` → remove entirely (dead code)

### 3. SGR button code reference

| SGR `button` value | Meaning |
|:---:|---|
| 0 | Left click |
| 1 | Middle click |
| 2 | Right click |
| 64 | Scroll up |
| 65 | Scroll down |
| Terminator `M` | Press/motion |
| Terminator `m` | Release |

### 4. Text selection (TUI-078) adaptation

The current text selection code disables mouse tracking on button-down (X10 bytes 32-34) and re-enables on button-release (X10 byte 35). With SGR:
- Button down: `button` 0-2 with terminator `M`
- Button release: `button` 0-2 with terminator `m`

## Verification

After the fix, verify:
1. Scroll wheel works in conversation view (VirtualList scroll mode)
2. Scroll wheel works in Kanban board columns
3. Scroll wheel works in model selector / settings / resume mode (AgentView)
4. Text selection still works (click to disable, release to re-enable mouse tracking)
5. TypeScript compiles with no `key.mouse` errors
