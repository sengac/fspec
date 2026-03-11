# BUG-103: ANSI Escape Codes and TUI Content Leaking into Fspec Tool Call Results

## Summary

Commands executed via the NAPI FspecTool bridge return ANSI escape codes (chalk formatting), TUI escape sequences, and double-captured output in their results. The root cause is an incomplete migration to the `output.ts` abstraction layer combined with missing ANSI stripping in the capture pipeline.

---

## Architecture: Three-Layer Output Capture System

When the Rust agent calls an fspec command via NAPI, `src/utils/fspec-callback.ts` sets up three capture layers:

### Layer 1: `output.log` / `output.error` / `output.warn` (the correct path)

- **File:** `src/utils/output.ts`
- Commands are *supposed* to use the `output` abstraction instead of `console.log` directly.
- `createCaptureContext()` (line 74-104) creates a capture context with two string arrays: `stdout[]` and `stderr[]`.
- In capture mode, messages are pushed as plain strings — **no chalk coloring applied** in the capture context (unlike the default CLI context which wraps `error` in `chalk.red` and `warn` in `chalk.yellow`).
- Activated via `setOutputContext(captureContext)` just before `program.parseAsync()`.

### Layer 2: Commander.js `configureOutput`

- **File:** `src/utils/fspec-callback.ts`, lines 643-653
- Captures Commander.js's own help text and error messages (e.g., `--help` output, unknown option errors).
- These bypass the `output` abstraction entirely.

```typescript
program.configureOutput({
  writeOut: (str) => { commanderOutput += str; },
  writeErr: (str) => { commanderError += str; },
  outputError: (str) => { commanderError += str; },
});
```

### Layer 3: Raw `process.stdout.write` / `process.stderr.write` monkey-patch

- **File:** `src/utils/fspec-callback.ts`, lines 581-620
- The **most aggressive** layer — catches absolutely everything.
- `console.log()` → `process.stdout.write` (overridden) → captured into `processStdoutCapture`.
- **All output is silently swallowed** — the original `write` is never called during capture.

### Combined Output

All three layers are merged into the final result:

```typescript
const capturedOutput =
  capturedStdout.join('\n') +                              // Layer 1
  (commanderOutput ? '\n' + commanderOutput : '') +        // Layer 2
  (processStdoutCapture ? '\n' + processStdoutCapture : '');  // Layer 3
```

---

## Problem 1: Commands Using `chalk` Inside `output.log()` (~10+ commands)

The `output.ts` capture context **does NOT strip ANSI codes**. When commands pass chalk-formatted strings through `output.log`, the ANSI escape codes are captured raw into the result returned to the LLM.

### Affected Files

| File | What leaks |
|------|------------|
| `src/commands/list-tags.ts:78` | `output.log(chalk.green(tag.tag) + ...)` — green-colored tag names |
| `src/commands/show-work-unit.ts:413,432,435` | Yellow event names, bold feature files, gray scenario references |
| `src/commands/list-scenario-tags.ts:166` | Cyan tags, gray categories |
| `src/commands/link-coverage.ts:216` | Yellow warnings |
| `src/commands/list-feature-tags.ts:143` | Cyan tags, gray categories |
| `src/commands/query-orphans.ts:123` | Bold "Suggested actions" |
| `src/commands/list-checkpoints.ts:45` | Bold checkpoint names |
| `src/commands/audit-scenarios.ts:172` | Cyan total count |
| `src/commands/discover-event-storm.ts:35` | `chalk.red(...)` passed to `output.error` |

---

## Problem 2: `console.log` Used Directly (Bypasses Layer 1)

### `help-formatter.ts:192`

`displayHelpAndExit()` calls `console.log(formatCommandHelp(config))` with heavily chalk-formatted text, then `process.exit(0)`. This is caught by Layer 3 but arrives as raw chalk-colored help text in the result.

---

## Problem 3: Double-Capture via `console-capture.ts` Interaction

`initializeConsoleCapture()` (called at startup in `src/index.ts` line 18) wraps every `console.*` method to also log to winston. When Layer 3 is active simultaneously:

```
console.log("hello")
  → wrapped console.log (console-capture.ts)
    → originalConsole.log("hello")
      → Node.js internal console.log
        → process.stdout.write (overridden by Layer 3!) → CAPTURED
    → winston logger (stripped of ANSI) → log file transport
      → if winston writes to stdout → POTENTIALLY CAPTURED AGAIN
```

This means some output could be **captured twice** — once through the normal path and once through winston's transport if it writes to stdout.

---

## Problem 4: `isInCaptureMode()` Exists but Is NEVER USED

There is a function `isInCaptureMode()` at line 65 of `output.ts` designed to let commands check if they're running in capture mode — but **no command ever calls it**. This was clearly intended to let commands conditionally skip chalk formatting when captured, but the conversion was never completed.

---

## Problem 5: Exit Cascade Artifacts

Many commands call `process.exit()` directly. The callback overrides `process.exit` (line 625-627) to throw `__FSPEC_EXIT_OVERRIDE__:N`. This causes cascading catches:

1. Command succeeds → `process.exit(0)` → throws `__FSPEC_EXIT_OVERRIDE__:0`
2. Command's own `catch` block catches this → treats as error → `process.exit(1)` → throws `__FSPEC_EXIT_OVERRIDE__:1`
3. Callback detects this and tries to clean up with **fragile regex patterns** (lines 797-817) that match chalk-colored `Error:` and `✗ Error:` prefixes

The cleanup regex depends on exact chalk formatting (e.g., `\x1b[31m`), which is inherently fragile.

---

## Problem 6: TUI Escape Sequences

TUI components write raw terminal escape sequences directly to `process.stdout.write`:

| File | Escape sequence |
|------|----------------|
| `src/tui/components/BoardView.tsx:122` | `\x1b[?1000h` (mouse tracking enable) |
| `src/tui/components/VirtualList.tsx:182` | `\x1b[?1000h` (mouse tracking enable) |
| `src/tui/components/AgentView.tsx:1583` | `\x1b[?1000h` (mouse tracking enable) |

These would be captured by Layer 3 if somehow active during fspec-callback execution. Currently mitigated by the `--format json` auto-injection in `fspec-callback.ts` (lines 680-686), but this is fragile — any command that doesn't respect `--format json` could trigger TUI rendering.

---

## Problem 7: No ANSI Stripping in Capture Context

The capture context in `output.ts` does **NOT** strip ANSI codes from pushed strings — unlike `console-capture.ts`'s winston path which does call `stripAnsi()`. This means any chalk that enters Layer 1 goes directly into the results.

---

## Problem 8: JSON Extraction and Output Contamination

At lines 736-743 of `fspec-callback.ts`, the callback tries to extract JSON from captured output:

```typescript
const jsonMatch = trimmedOutput.match(/(\{[\s\S]*\}|\[[\s\S]*\])$/);
```

This regex looks for the **last** JSON object/array in the output. If chalk ANSI codes appear **inside** JSON values (e.g., a command builds a JSON string using chalk), the JSON parse would fail and fall back to raw text output.

---

## Recommended Fixes

### Fix 1: Strip ANSI in Capture Context (Catch-All) — HIGH PRIORITY

Add `stripAnsi()` in `createCaptureContext()` in `output.ts` so that the `stdout[]` and `stderr[]` arrays always receive clean text:

```typescript
// In createCaptureContext()
const captureContext = {
  log: (msg: string) => { stdout.push(stripAnsi(msg)); },
  error: (msg: string) => { stderr.push(stripAnsi(msg)); },
  warn: (msg: string) => { stderr.push(stripAnsi(msg)); },
  // ...
};
```

### Fix 2: Strip ANSI in Layer 3 Capture — HIGH PRIORITY

Add `stripAnsi()` to the `processStdoutCapture` and `processStderrCapture` strings in `fspec-callback.ts`:

```typescript
const overriddenStdoutWrite = (...args) => {
  const str = typeof args[0] === 'string' ? args[0] : args[0]?.toString();
  if (str) { processStdoutCapture += stripAnsi(str); }
  return true;
};
```

### Fix 3: Convert Remaining `console.log` to `output.log` — MEDIUM PRIORITY

Audit and convert all commands that use `console.log` directly (especially `help-formatter.ts`) to use the `output` abstraction.

### Fix 4: Remove Chalk from `output.log()` Calls — MEDIUM PRIORITY

In the ~10 affected commands, remove chalk wrapping when calling `output.log()`:

```typescript
// Before
output.log(`  ${chalk.green(tag.tag)} - ${tag.description}`);

// After
output.log(`  ${tag.tag} - ${tag.description}`);
```

Or conditionally apply chalk only when not in capture mode:

```typescript
import { isInCaptureMode } from '../utils/output';
const tagName = isInCaptureMode() ? tag.tag : chalk.green(tag.tag);
output.log(`  ${tagName} - ${tag.description}`);
```

### Fix 5: Suppress Winston Stdout Transport During Capture — LOW PRIORITY

If winston has a stdout transport active, temporarily disable it during fspec-callback capture to prevent double-capture.

---

## Priority Matrix

| Fix | Severity | Effort | Impact |
|-----|----------|--------|--------|
| Strip ANSI in capture context (`output.ts`) | High | Low | Catches all Layer 1 chalk leaks |
| Strip ANSI in Layer 3 (`fspec-callback.ts`) | High | Low | Catches all `console.log` + chalk bypasses |
| Convert `console.log` → `output.log` in commands | Medium | Medium | Prevents Layer 1 bypass, improves architecture |
| Remove chalk from `output.log()` calls | Medium | Medium | Clean separation of CLI vs capture output |
| Use `isInCaptureMode()` in commands | Low | High | Most work, least immediate impact |
| Suppress winston stdout during capture | Low | Low | Edge case prevention |

---

## Files to Modify (Minimum Viable Fix)

For the fastest resolution that eliminates most issues, only two files need changes:

1. **`src/utils/output.ts`** — Add `stripAnsi()` to `createCaptureContext()` push calls
2. **`src/utils/fspec-callback.ts`** — Add `stripAnsi()` to Layer 3 `process.stdout.write` / `process.stderr.write` overrides

This two-file fix would act as a catch-all safety net, stripping ANSI at the capture boundary regardless of what commands do upstream.
