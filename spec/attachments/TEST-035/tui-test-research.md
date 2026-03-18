# tui-test Research: E2E Terminal Testing for fspec

## Overview

[`@microsoft/tui-test`](https://github.com/microsoft/tui-test) (v0.0.3) is an end-to-end terminal testing framework by Microsoft. It's essentially **Playwright for terminals** — it spawns real PTY processes, pipes output through a headless xterm.js emulator, and provides auto-wait locator assertions to query screen state.

## Architecture

### Core Pipeline

```
Real PTY process (node-pty)
    → PTY.onData() 
    → xterm.Terminal.write(data)   [headless @xterm/headless]
    → In-memory screen buffer
    → Locator queries (getByText, getBuffer, serialize)
```

Every byte of output from the real process is piped into a headless xterm emulator that fully interprets ANSI escape sequences, cursor movements, colors, scrolling — exactly as a real terminal would.

### PTY Layer

- **Node.js**: Uses `node-pty` package. Wraps handle with safe error-swallowing wrappers for `write`, `resize`, `kill` (PTY may close between check and call).
- **Bun**: Uses native `Bun.Terminal` and `Bun.spawn` APIs.
- Interface: `IPtyBackend` with five methods: `onData`, `onExit`, `write`, `resize`, `kill`.

### Two Modes

1. **Shell mode**: Launches a shell (bash, zsh, fish, cmd, powershell, xonsh) with shell integration scripts
2. **Program mode**: Launches a specific program directly via `which` resolution — **this is what we need for fspec**

### Locator System (Auto-Wait)

`terminal.getByText(text | regex, options?)` returns a `Locator` — a lazy, poll-based element finder:

- Flattens the terminal buffer into a single string and searches for text/regex
- **Polling**: Checks every **50ms** until timeout (configurable per-assertion)
- **Strict mode** (default): Throws if multiple matches found
- On timeout, dumps the full terminal contents in the error message for debugging

```typescript
// Poll implementation (src/utils/poll.ts)
async function _poll(callback, startTime, delay, timeout, isNot) {
  const result = await Promise.resolve(callback());
  if (!isNot && result) return true;
  if (isNot && !result) return false;
  if (startTime + timeout < Date.now()) return isNot;
  return new Promise(resolve =>
    setTimeout(() => resolve(_poll(callback, startTime, delay, timeout, isNot)), delay)
  );
}
```

### Assertion API

```typescript
// Text appears on screen (auto-waits)
await expect(terminal.getByText("Loading...")).toBeVisible();

// Text does NOT appear (auto-waits for disappearance)
await expect(terminal.getByText("Loading...")).not.toBeVisible();

// Regex matching (must use global flag)
await expect(terminal.getByText(/backlog/gi, { strict: false })).toBeVisible();

// Full buffer search (not just visible viewport)
await expect(terminal.getByText("usage: git", { full: true })).toBeVisible();

// Snapshot testing (captures colors, styles, full screen layout)
await expect(terminal).toMatchSnapshot();
```

### Terminal Interaction API

```typescript
terminal.write("foo");           // Type characters (no Enter)
terminal.submit("echo hello");   // Type + press Enter
terminal.submit();               // Just press Enter

// Navigation keys
terminal.keyUp(count?)
terminal.keyDown(count?)
terminal.keyLeft(count?)
terminal.keyRight(count?)
terminal.keyEscape(count?)
terminal.keyBackspace(count?)
terminal.keyCtrlC(count?)
terminal.keyCtrlD(count?)

// Arbitrary key combos with modifiers
terminal.keyPress("a", { ctrl: true, alt: false, shift: false });
terminal.keyPress(Key.F5);
terminal.keyPress(Key.Home, { shift: true });

// Mouse
terminal.mousePress(x, y);      // Click
terminal.mouseTo(x, y);         // Move

// Terminal management
terminal.resize(columns, rows);
terminal.getCursor();            // { x, y, baseY }
terminal.getBuffer();            // Full buffer as string[][]
terminal.getViewableBuffer();    // Visible viewport only
terminal.serialize();            // For snapshot comparison
```

### Trace System

When `trace: true`, every byte received by the terminal is recorded with a timestamp. Traces are stored in `tui-traces/` and can be replayed via `show-trace` command. This is invaluable for debugging timing issues like our "Loading..." problem.

### Test Lifecycle

1. **Per-test isolation**: Each test gets a fresh PTY + xterm.js instance (no state leakage)
2. **Parallel execution**: Tests run in a worker pool (configurable)
3. **Retry support**: Configurable retries per test
4. **Cleanup**: PTY processes are killed after each test

## Relevance to fspec "Loading..." Investigation

### What We're Debugging

The fspec TUI shows "Loading..." in the SessionHeader when transitioning from BoardView to AgentView. Two distinct scenarios:

1. **New session** (no prior session attached): "Loading..." appears while `initializeModels()` runs (NAPI calls to Rust for model/provider discovery)
2. **Resuming existing session** (work unit has attached session with many messages): "Loading..." appears while `persistenceGetSessionMessageEnvelopes()` rehydrates blobs (~2.7s for 3069 messages)

### Why tui-test Helps

| Problem | tui-test Solution |
|---------|-------------------|
| Can't measure how long "Loading..." is visible | Trace recording with millisecond timestamps |
| Can't distinguish new vs resume timing | Two separate test cases, same assertions |
| Guessing which React effects fire when | Black-box observation of what actually renders |
| No regression detection for UI timing | Snapshot + timing assertions |
| Can't reproduce exact user navigation flow | `keyDown()`, `submit()`, `write("/")` scripting |

### fspec Integration Points

- **Entry point**: `./dist/index.js` (Vite-bundled, `#!/usr/bin/env node` shebang)
- **Launch**: `fspec` with no arguments enters TUI mode (requires `process.stdin.isTTY`)
- **Program mode works**: tui-test's PTY provides a real TTY, so fspec's TTY check passes
- **Current test infra**: Vitest with jsdom, no existing E2E/PTY tests

## Proposed Setup

### Installation

```bash
npm i -D @microsoft/tui-test
```

### Configuration

`tui-test.config.ts`:
```typescript
import { defineConfig } from "@microsoft/tui-test";

export default defineConfig({
  retries: 1,
  trace: true,
});
```

### Package.json Script

```json
{
  "scripts": {
    "test:e2e": "npx @microsoft/tui-test",
    "test:e2e:trace": "npx @microsoft/tui-test --trace"
  }
}
```

### Example Test: Board Renders

```typescript
import { test, expect } from "@microsoft/tui-test";

test.use({
  program: { file: "./dist/index.js" },
  rows: 40,
  columns: 120,
});

test("fspec board renders work units", async ({ terminal }) => {
  // Board should render column headers
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});
```

### Example Test: New Session Loading Time

```typescript
import { test, expect } from "@microsoft/tui-test";

test.use({
  program: { file: "./dist/index.js" },
  rows: 40,
  columns: 120,
});

test("new session shows model name quickly", async ({ terminal }) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Press Enter on the first work unit
  terminal.submit();

  // Create session dialog should appear
  await expect(
    terminal.getByText(/Start New Agent/gi, { strict: false })
  ).toBeVisible();

  // Confirm with Y (or Enter depending on dialog)
  terminal.write("y");

  // "Loading..." should disappear within a reasonable time
  // If this times out, we have a performance bug
  await expect(
    terminal.getByText("Loading...", { strict: false })
  ).not.toBeVisible();
});
```

### Example Test: Navigation Flow

```typescript
import { test, expect } from "@microsoft/tui-test";

test.use({
  program: { file: "./dist/index.js" },
  rows: 40,
  columns: 120,
});

test("slash key navigates to agent view", async ({ terminal }) => {
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Press / to navigate (same as Shift+Right)
  terminal.write("/");

  // Should either show create dialog or navigate to existing session
  // Trace will show exact timing of what renders
});

test("escape returns to board from agent view", async ({ terminal }) => {
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  terminal.submit(); // Enter agent view
  
  // Wait for agent view to appear
  await expect(
    terminal.getByText(/Start New Agent/gi, { strict: false })
  ).toBeVisible();

  terminal.keyEscape(); // Cancel dialog
  terminal.keyEscape(); // Back to board

  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});
```

### Debugging with Traces

After running with `--trace`, inspect `tui-traces/` for recorded sessions. Each trace contains every byte the terminal received with timestamps, allowing you to:

1. See exactly when "Loading..." first appears in the terminal buffer
2. See exactly when it gets replaced by the actual model name
3. Calculate the precise delay
4. Compare new session vs resume session timings

## File Structure

```
fspec/
├── tui-test.config.ts          # tui-test configuration
├── e2e/                         # E2E test directory
│   ├── board.test.ts            # Board rendering tests
│   ├── navigation.test.ts       # Board ↔ Agent navigation tests
│   ├── session-loading.test.ts  # Loading/timing regression tests
│   └── keyboard.test.ts         # Keyboard shortcut tests
├── tui-traces/                  # Auto-generated trace recordings
│   └── ...
└── tui-snapshots/               # Auto-generated snapshot files
    └── ...
```

## Key Considerations

1. **Build first**: Tests need `./dist/index.js` to exist — run `npm run build` before E2E tests
2. **node-pty dependency**: tui-test requires `node-pty` (native addon, needs build tools)
3. **CI compatibility**: fspec's TTY check (`process.stdin.isTTY`) — tui-test's PTY satisfies this, but CI environments may need special handling
4. **Test isolation**: Each test gets a fresh PTY, so no state leaks between tests
5. **Existing vitest tests unaffected**: tui-test has its own test runner, separate from vitest
6. **gitignore**: Add `tui-traces/` and `tui-snapshots/` to `.gitignore` (or keep snapshots for regression testing)
