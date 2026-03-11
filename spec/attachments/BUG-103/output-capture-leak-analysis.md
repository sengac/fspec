# BUG-103: ANSI Escape Codes and TUI Content Leaking into Fspec Tool Call Results

## Status: RESOLVED

---

## Root Cause

**Layer 3 (`process.stdout.write` override) was the primary bug, not a missing ANSI strip.**

When `fspecCallback` monkey-patched `process.stdout.write`, it created a global interception that captured **everything written to stdout** — including Ink's TUI renderer which writes screen redraws concurrently via `process.stdout.write` during async command execution.

The ThinkingIndicator spinner fires a `setInterval` every 80ms, triggering React state changes → Ink re-renders → `stream.write(buffer)` → `process.stdout.write` (now intercepted). Each render cycle dumped a full TUI frame (spinner characters, tool call output, conversation content, UI chrome) into `processStdoutCapture`.

**Why some commands appeared clean:** Fast commands (e.g., `list-work-units`) completed within a single event loop tick before Ink's next render timer fired. Slow async commands (e.g., `link-coverage` with file validation, step consistency checks) gave the event loop time to service multiple Ink render timers, accumulating frames of TUI content in the capture.

**Secondary issue:** ANSI stripping was also missing from all capture layers, so chalk-formatted text from `output.log()` calls and Commander output arrived with raw escape codes. But even with stripping, the **text content** of captured TUI frames remained — stripping `\x1b[33m` from `\x1b[33mThinking...\x1b[39m` still leaves `Thinking...` in the result.

**Tertiary issue:** `configureOutput` on the root Commander program didn't propagate to subcommands. Commander's `copyInheritedSettings` copies the parent's `_outputConfiguration` by reference during `addCommand()`, but `configureOutput()` creates a **new object** via spread — breaking the shared reference. All subcommands retained the old config that wrote to `process.stdout.write`. This was why Layer 3 was originally added: Layer 2 only captured root-program output, not subcommand help/errors.

---

## Architecture Before Fix: Three-Layer Output Capture

When the Rust agent called an fspec command via NAPI, `fspec-callback.ts` set up three capture layers:

### Layer 1: `output.log` / `output.error` / `output.warn`

- **File:** `src/utils/output.ts`
- Commands use the `output` abstraction instead of `console.log`.
- `createCaptureContext()` creates arrays: `stdout[]` and `stderr[]`.
- Activated via `setOutputContext(captureContext)` before `program.parseAsync()`.
- **Status:** Working correctly, but was missing ANSI stripping.

### Layer 2: Commander.js `configureOutput`

- **File:** `src/utils/fspec-callback.ts`
- Captures Commander.js help text and error messages.
- **Bug:** Only applied to root program, not subcommands (due to `configureOutput` timing vs `copyInheritedSettings`).

### Layer 3: Raw `process.stdout.write` monkey-patch (THE BUG)

- **File:** `src/utils/fspec-callback.ts`
- Overrode `process.stdout.write` / `process.stderr.write` to capture everything.
- **Intended purpose:** Catch subcommand help/errors that Layer 2 missed.
- **Actual effect:** Also captured all concurrent Ink TUI renders, contaminating tool results with spinner text, conversation content, and UI chrome.

### Combined Output (Before Fix)

```typescript
const capturedOutput =
  capturedStdout.join('\n') +                              // Layer 1
  (commanderOutput ? '\n' + commanderOutput : '') +        // Layer 2 (root only!)
  (processStdoutCapture ? '\n' + processStdoutCapture : '');  // Layer 3 (TUI junk!)
```

---

## The Ink Render Race Condition (Detailed)

```
1. Rust agent emits FspecCommandRequest {command: "link-coverage", ...}
2. GlobalSessionStreamManager.handleFspecCommandRequest() calls fspecCallback()
3. fspecCallback():
   a. Overrides process.stdout.write ← THE TRAP IS SET
   b. Calls program.parseAsync(argv) ← STARTS ASYNC EXECUTION
   c. During the await, event loop processes:
      - ThinkingIndicator setInterval fires (every 80ms)
      - React setState → Ink onRender() → throttledLog()
      - stream.write(buffer) → process.stdout.write ← INTERCEPTED
      - Full TUI frame captured into processStdoutCapture
      - Repeats every ~80ms for duration of command
   d. Command completes
   e. processStdoutCapture now contains N TUI frames + actual command output
   f. JSON extraction regex may or may not find actual output amid TUI noise
```

**Ink's rendering pipeline:** Ink receives `stdout: process.stdout` at construction. Its `log-update` instance closes over this as `stream` and calls `stream.write(buffer.join(''))` on each render. Since `stream.write` resolves to `process.stdout.write` **at call time** (not at closure time), fspecCallback's override intercepts every render.

**There is no Ink pause API.** The `Ink` class has no `pause()` or `suspend()` method — the only way to stop rendering is `unmount()`. The `patchConsole: false` config and `incrementalRendering: true` config don't help — the problem is that fspecCallback intercepts Ink's output, not the other way around.

---

## Fix Applied

### 1. Removed Layer 3 entirely

Deleted the `process.stdout.write` and `process.stderr.write` overrides from `fspec-callback.ts`. The TUI's Ink renderer now writes to the terminal normally during async command execution, uncontaminated.

### 2. Fixed Layer 2: Propagated `configureOutput` to all subcommands

```typescript
const commanderOutputConfig = {
  writeOut: (str: string) => { commanderOutput += stripAnsi(str); },
  writeErr: (str: string) => { commanderError += stripAnsi(str); },
  outputError: (str: string) => { commanderError += stripAnsi(str); },
};
program.configureOutput(commanderOutputConfig);
for (const cmd of program.commands) {
  cmd.configureOutput(commanderOutputConfig);
}
```

This ensures subcommand help output and error messages are captured by Layer 2 — the reason Layer 3 was originally added.

### 3. Added ANSI stripping to Layer 1 (`createCaptureContext`)

Added shared `stripAnsi()` to `output.ts` with a comprehensive regex handling CSI sequences (colors, cursor, erase, mouse tracking), OSC sequences, character set designations, and other escape sequences. Applied in `createCaptureContext()` so all `output.log/error/warn` calls are stripped at the capture boundary.

### 4. DRY: Consolidated `stripAnsi` to single definition

Replaced duplicate weak `stripAnsi` implementations (SGR-only regex `/\x1b\[[0-9;]*m/g`) in `src/help.ts` and `src/utils/console-capture.ts` with imports from `src/utils/output.ts`. The shared implementation handles all ANSI sequence types.

### 5. Removed dead ANSI regex in `cleanExitOverrideArtifacts`

Since all capture layers now strip ANSI before storage, the colored `\x1b[31mError:\x1b[39m __FSPEC_EXIT_OVERRIDE__` regex pattern was dead code. Removed it and updated the comment.

---

## Architecture After Fix: Two-Layer Output Capture

```typescript
const capturedOutput =
  capturedStdout.join('\n') +                         // Layer 1: output.log (all commands)
  (commanderOutput ? '\n' + commanderOutput : '');    // Layer 2: Commander help/errors (all subcommands)
```

| Layer | What it captures | ANSI stripped? |
|-------|-----------------|----------------|
| **Layer 1** | `output.log/error/warn` from commands | ✅ in `createCaptureContext()` |
| **Layer 2** | Commander subcommand help/errors | ✅ in `configureOutput` callbacks |
| ~~Layer 3~~ | ~~process.stdout.write~~ | **REMOVED** |

**TUI Ink renders:** Pass through to terminal normally. No global stdout interception.

---

## Files Modified

| File | Change |
|------|--------|
| `src/utils/fspec-callback.ts` | Removed Layer 3 (process.stdout/stderr.write overrides). Propagated `configureOutput` to all subcommands. Removed dead ANSI regex. |
| `src/utils/output.ts` | Added exported `stripAnsi()` with comprehensive CSI/OSC/SGR regex. Applied in `createCaptureContext()`. |
| `src/utils/console-capture.ts` | Replaced local weak `stripAnsi` with import from `output.ts`. |
| `src/help.ts` | Replaced local weak `stripAnsi` with import from `output.ts`. |

---

## Original Problem Inventory (from pre-fix analysis)

| Problem | Status | Resolution |
|---------|--------|------------|
| P1: Commands using chalk inside `output.log()` | ✅ Fixed | `stripAnsi()` in `createCaptureContext()` strips at boundary |
| P2: `console.log` used directly in help-formatter | ✅ Mitigated | Layer 2 now captures subcommand help; no Layer 3 to contaminate |
| P3: Double-capture via console-capture.ts | ✅ Fixed | Layer 3 removed — no global stdout interception to double-capture |
| P4: `isInCaptureMode()` never used | ⚪ Not addressed | Low priority — stripping at boundary makes this unnecessary |
| P5: Exit cascade artifacts with ANSI regexes | ✅ Fixed | Dead ANSI regex removed; remaining patterns are plain text |
| P6: TUI escape sequences captured | ✅ Fixed | **Root cause** — Layer 3 removed entirely |
| P7: No ANSI stripping in capture context | ✅ Fixed | `stripAnsi()` applied in all capture paths |
| P8: JSON extraction contaminated by ANSI | ✅ Fixed | All captured text is now ANSI-free before JSON extraction |
