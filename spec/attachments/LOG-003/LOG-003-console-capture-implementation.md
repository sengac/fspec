# LOG-003: Console Capture Implementation Guide

## Executive Summary

This document provides a comprehensive analysis of implementing console method capture and redirection to the winston logger utility in fspec. The goal is to ensure all console output (log, error, warn, info, debug, trace) is captured in the centralized log file (`~/.fspec/fspec.log`) while preserving normal console output behavior.

## Current State Analysis

### Logger Infrastructure (LOG-001, LOG-002)

The winston logger is already established in `src/utils/logger.ts`:

```typescript
// Singleton logger with lazy initialization
// Logs to ~/.fspec/fspec.log
// Uses Proxy pattern for seamless access
export const logger = new Proxy({} as winston.Logger, { ... });
```

**Key characteristics:**
- Lazy initialization (respects test mocking)
- Logs to `~/.fspec/fspec.log` with append mode
- Log level controlled by `FSPEC_LOG_LEVEL` env var (default: 'info')
- Format: `{timestamp} [{level}]: {message}`

### Console Usage in Codebase

**Quantitative Analysis (non-test files):**
| Method | Count | Primary Usage |
|--------|-------|---------------|
| `console.log` | 1,601 | Help output, command results, user feedback |
| `console.error` | 266 | Error messages, validation failures |
| `console.warn` | 11 | Deprecation warnings, non-critical issues |
| `console.info` | 0 | Not used |
| `console.debug` | 0 | Not used |
| `console.trace` | 0 | Not used |

**High-volume files:**
1. `src/help.ts` - Majority of console.log calls (~1,400+)
2. `src/migrations/index.ts` - Migration status output
3. Various command files - User feedback and results

### Entry Point Analysis (`src/index.ts`)

The application entry point has a specific initialization order:

```typescript
#!/usr/bin/env node

// 1. PERF-001: Performance measure buffer clearing (lines 3-13)
import { performance } from 'perf_hooks';
setInterval(() => { performance.clearMeasures(); }, 30000).unref();

// 2. Commander.js and other imports (lines 15-191)
import { Command } from 'commander';
// ... many imports ...

// 3. Program configuration and command registration (lines 195-372)

// 4. Main function execution (lines 374-455)
```

**Critical insight:** Console capture MUST be established before any other imports that might use console methods. The current PERF-001 block at the top is the ideal location pattern to follow.

### Existing Console Capture Pattern

There's a temporary capture pattern in `src/help.ts`:

```typescript
function captureConsoleOutput(fn: () => void): string {
  const originalLog = console.log;
  const logs: string[] = [];
  console.log = (...args: unknown[]) => {
    logs.push(args.map(String).join(' '));
  };
  try {
    fn();
  } finally {
    console.log = originalLog;
  }
  return stripAnsi(logs.join('\n'));
}
```

This pattern captures output temporarily for processing. Our implementation will be similar but persistent and log to winston.

## Proposed Solution

### Architecture

Create a new utility module `src/utils/console-capture.ts` that:
1. Stores references to original console methods
2. Overrides all console methods to also log through winston
3. Strips ANSI escape codes from log file output
4. Exports a function to initialize capture early in index.ts

### Console Method to Winston Level Mapping

| Console Method | Winston Level | Rationale |
|----------------|---------------|-----------|
| `console.log` | `info` | General information output |
| `console.info` | `info` | Informational messages |
| `console.warn` | `warn` | Warning conditions |
| `console.error` | `error` | Error conditions |
| `console.debug` | `debug` | Debug-level messages |
| `console.trace` | `debug` | Stack trace (debug level + stack) |

### Implementation Details

#### 1. Console Capture Module (`src/utils/console-capture.ts`)

```typescript
/**
 * Console Capture Utility for fspec
 *
 * Intercepts all console methods and redirects output to winston logger
 * while preserving original console behavior.
 *
 * Coverage: LOG-003
 */

import { logger } from './logger';

// Store original console methods
const originalConsole = {
  log: console.log,
  error: console.error,
  warn: console.warn,
  info: console.info,
  debug: console.debug,
  trace: console.trace,
};

// ANSI escape code stripping regex
// eslint-disable-next-line no-control-regex
const ANSI_REGEX = /\x1b\[[0-9;]*m/g;

/**
 * Strip ANSI escape codes (chalk formatting) from a string
 */
function stripAnsi(str: string): string {
  return str.replace(ANSI_REGEX, '');
}

/**
 * Format arguments for logging
 * Handles multiple arguments, objects, and strips ANSI codes
 */
function formatArgs(args: unknown[]): string {
  return stripAnsi(
    args
      .map(arg => {
        if (typeof arg === 'string') return arg;
        if (arg instanceof Error) return arg.stack || arg.message;
        try {
          return JSON.stringify(arg, null, 2);
        } catch {
          return String(arg);
        }
      })
      .join(' ')
  );
}

/**
 * Initialize console capture
 * Call this as early as possible in the application entry point
 */
export function initializeConsoleCapture(): void {
  // Override console.log -> logger.info
  console.log = (...args: unknown[]) => {
    originalConsole.log(...args);
    logger.info(formatArgs(args));
  };

  // Override console.info -> logger.info
  console.info = (...args: unknown[]) => {
    originalConsole.info(...args);
    logger.info(formatArgs(args));
  };

  // Override console.warn -> logger.warn
  console.warn = (...args: unknown[]) => {
    originalConsole.warn(...args);
    logger.warn(formatArgs(args));
  };

  // Override console.error -> logger.error
  console.error = (...args: unknown[]) => {
    originalConsole.error(...args);
    logger.error(formatArgs(args));
  };

  // Override console.debug -> logger.debug
  console.debug = (...args: unknown[]) => {
    originalConsole.debug(...args);
    logger.debug(formatArgs(args));
  };

  // Override console.trace -> logger.debug with stack trace
  console.trace = (...args: unknown[]) => {
    originalConsole.trace(...args);
    const stack = new Error().stack || '';
    logger.debug(`${formatArgs(args)}\n${stack}`);
  };
}

/**
 * Restore original console methods
 * Primarily for testing purposes
 */
export function restoreConsole(): void {
  console.log = originalConsole.log;
  console.error = originalConsole.error;
  console.warn = originalConsole.warn;
  console.info = originalConsole.info;
  console.debug = originalConsole.debug;
  console.trace = originalConsole.trace;
}

/**
 * Get original console methods
 * For cases where direct console access is needed (e.g., test output)
 */
export const originalConsoleMethods = originalConsole;
```

#### 2. Index.ts Modification

Add the console capture initialization immediately after the PERF-001 block:

```typescript
#!/usr/bin/env node

// PERF-001: Clear React 19's performance measure buffer periodically
// ... existing code ...

// LOG-003: Capture all console methods and redirect to winston logger
// This MUST run before any other imports that might use console
import { initializeConsoleCapture } from './utils/console-capture';
initializeConsoleCapture();

import { Command } from 'commander';
// ... rest of imports ...
```

**CRITICAL:** The import and initialization MUST be at the very top, before any other imports except the performance hook. This ensures all subsequent console calls are captured.

#### 3. Test File (`src/utils/__tests__/console-capture.test.ts`)

```typescript
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { initializeConsoleCapture, restoreConsole } from '../console-capture';
import { logger } from '../logger';

// Mock the logger
vi.mock('../logger', () => ({
  logger: {
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  },
}));

describe('console-capture', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    initializeConsoleCapture();
  });

  afterEach(() => {
    restoreConsole();
  });

  it('should capture console.log and log to info level', () => {
    console.log('test message');
    expect(logger.info).toHaveBeenCalledWith('test message');
  });

  it('should capture console.error and log to error level', () => {
    console.error('error message');
    expect(logger.error).toHaveBeenCalledWith('error message');
  });

  it('should capture console.warn and log to warn level', () => {
    console.warn('warning message');
    expect(logger.warn).toHaveBeenCalledWith('warning message');
  });

  it('should strip ANSI escape codes', () => {
    console.log('\x1b[31mred text\x1b[0m');
    expect(logger.info).toHaveBeenCalledWith('red text');
  });

  it('should handle multiple arguments', () => {
    console.log('hello', 'world', 123);
    expect(logger.info).toHaveBeenCalledWith('hello world 123');
  });

  it('should handle objects', () => {
    console.log({ foo: 'bar' });
    expect(logger.info).toHaveBeenCalledWith('{\n  "foo": "bar"\n}');
  });

  it('should handle Error objects', () => {
    const error = new Error('test error');
    console.error(error);
    expect(logger.error).toHaveBeenCalledWith(expect.stringContaining('test error'));
  });

  it('should restore original console methods', () => {
    const originalLog = console.log;
    restoreConsole();
    initializeConsoleCapture();
    restoreConsole();
    // After restore, console.log should be different from captured version
    expect(console.log).not.toBe(originalLog);
  });
});
```

## Edge Cases and Considerations

### 1. Circular Dependencies

The console-capture module imports from logger.ts. Ensure logger.ts doesn't directly use console methods at module load time (it doesn't currently).

### 2. Test Environment

Tests may need to:
- Mock the logger to avoid file I/O
- Call `restoreConsole()` in afterEach to prevent test pollution
- Access `originalConsoleMethods` for direct test output

### 3. Performance

Each console call now has additional overhead:
- ANSI stripping regex
- JSON serialization for objects
- Winston logging (async file I/O)

For typical CLI usage, this overhead is negligible. The winston file transport is already async/buffered.

### 4. Log File Size

With all console output captured, the log file will grow faster. Consider:
- Adding log rotation (winston-daily-rotate-file)
- Documenting log file location and cleanup
- Making capture optional via env var (e.g., `FSPEC_CAPTURE_CONSOLE=false`)

### 5. Ink/React TUI

The interactive TUI uses Ink which may have special console handling. Test thoroughly with:
- `fspec` (no args - launches TUI)
- Keyboard navigation
- Screen updates

### 6. help.ts captureConsoleOutput

The existing `captureConsoleOutput` function in help.ts will still work because:
- It stores the current console.log (which is our wrapped version)
- It replaces console.log temporarily
- It restores our wrapped version when done

This means captured help output will NOT be double-logged, which is the desired behavior.

## Implementation Checklist

- [ ] Create `src/utils/console-capture.ts` with:
  - [ ] `initializeConsoleCapture()` function
  - [ ] `restoreConsole()` function
  - [ ] `originalConsoleMethods` export
  - [ ] ANSI stripping utility
  - [ ] Argument formatting utility

- [ ] Modify `src/index.ts`:
  - [ ] Add import at top (after PERF-001 block)
  - [ ] Call `initializeConsoleCapture()` before other imports

- [ ] Create test file `src/utils/__tests__/console-capture.test.ts`:
  - [ ] Test all console method captures
  - [ ] Test ANSI stripping
  - [ ] Test multiple arguments
  - [ ] Test object serialization
  - [ ] Test Error handling
  - [ ] Test restore functionality

- [ ] Manual testing:
  - [ ] Run various fspec commands
  - [ ] Verify output appears in console AND log file
  - [ ] Verify ANSI codes stripped from log file
  - [ ] Test interactive TUI (fspec with no args)
  - [ ] Test help commands (fspec help, fspec --help)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaks existing console output | Low | High | Preserve original call, test thoroughly |
| Performance degradation | Low | Medium | Async winston transport handles this |
| Test pollution | Medium | Medium | Provide restoreConsole() utility |
| TUI compatibility | Low | High | Test interactive mode thoroughly |
| Circular dependency | Low | High | Logger lazy initialization prevents this |

## Success Criteria

1. All console methods (log, error, warn, info, debug, trace) are captured
2. Original console output is preserved (appears in terminal)
3. All output is logged to `~/.fspec/fspec.log`
4. ANSI escape codes are stripped from log file
5. Multiple arguments are properly formatted
6. Objects and Errors are serialized appropriately
7. All existing tests pass
8. New tests cover the capture functionality
9. Interactive TUI continues to work correctly
10. Help system continues to work correctly

## References

- LOG-001: Add winston universal logger for fspec
- LOG-002: Wire LockedFileManager errors to winston logger
- `src/utils/logger.ts`: Winston logger implementation
- `src/help.ts:1737-1752`: Existing console capture pattern
- `src/index.ts`: Application entry point
