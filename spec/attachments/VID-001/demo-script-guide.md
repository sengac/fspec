# Demo Script Writing Guide

## Overview

This guide explains how to write demo scripts for fspec.videos that showcase terminal interactions and TUI applications. Demo scripts are executable TypeScript files that programmatically control terminal processes to create automated demonstrations.

## The Problem with @microsoft/tui-test

**CRITICAL: Do NOT use `@microsoft/tui-test` for demo scripts!**

The `@microsoft/tui-test` package is a **test framework**, not a library for programmatic terminal control. It provides:
- `test()` - Test definition function
- `expect()` - Assertion functions
- Test runner infrastructure

It does **NOT** provide:
- ❌ `TestController` class
- ❌ Programmatic terminal control API
- ❌ Methods like `spawn()`, `write()`, `waitForText()`

Attempting to import these will result in:
```
SyntaxError: The requested module '@microsoft/tui-test' does not provide an export named 'TestController'
```

## Correct Approach: Using Node.js Built-ins

Demo scripts should use Node.js built-in modules to control terminal processes:

### Required Imports

```typescript
import { spawn, ChildProcess } from 'child_process';
```

### Core Pattern

```typescript
export async function myDemo() {
  console.log('Starting demo...');

  // 1. Spawn a shell process
  const shell = spawn('bash', ['-i'], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      PS1: '$ ',           // Simple prompt
      FORCE_COLOR: '1',    // Enable colors
    },
  });

  // 2. Helper functions
  const write = (text: string) => {
    shell.stdin.write(text);
  };

  const delay = (ms: number) =>
    new Promise(resolve => setTimeout(resolve, ms));

  try {
    // 3. Wait for shell to initialize
    await delay(1000);

    // 4. Execute commands
    console.log('Demo: Running command...');
    write('ls -la\n');
    await delay(2000);

    // 5. Clean exit
    console.log('Demo: Exiting...');
    write('exit\n');
    await delay(1000);

    // 6. Wait for process to exit
    await new Promise<void>((resolve, reject) => {
      shell.on('exit', (code) => {
        if (code === 0) {
          console.log('Demo completed successfully!');
          resolve();
        } else {
          reject(new Error(`Shell exited with code ${code}`));
        }
      });
    });
  } catch (error) {
    console.error('Demo failed:', error);
    shell.kill();
    throw error;
  }
}
```

## Demo Script Requirements

### 1. Module Structure

Every demo script must:

```typescript
// Export the main function
export async function demoName() {
  // Demo logic here
}

// Support direct execution
if (import.meta.url === `file://${process.argv[1]}`) {
  demoName().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}
```

### 2. Console Logging

Use `console.log()` to narrate what the demo is doing:

```typescript
console.log('Starting fspec demo...');
console.log('Demo: Navigating work units...');
console.log('Demo: Opening work unit details...');
console.log('Demo completed successfully!');
```

This helps:
- Debug failures when recording
- Understand demo flow when reading code
- Track progress during long recordings

### 3. Timing and Delays

```typescript
// Wait for shell initialization
await delay(1000);

// Wait after each command for output to appear
write('ls\n');
await delay(2000);

// Wait for visual effects to complete
write('q');  // Quit command
await delay(500);
```

**Guidelines:**
- Shell initialization: 1000ms
- After commands: 1500-2000ms
- After keystrokes: 500-1000ms
- Before exit: 1000ms

### 4. Error Handling

```typescript
try {
  // Demo logic
  await delay(1000);
  write('command\n');

  // Wait for exit
  await new Promise<void>((resolve, reject) => {
    shell.on('exit', (code) => {
      if (code === 0) {
        console.log('Demo completed successfully!');
        resolve();
      } else {
        reject(new Error(`Shell exited with code ${code}`));
      }
    });
  });
} catch (error) {
  console.error('Demo failed:', error);
  shell.kill();  // Clean up
  throw error;
}
```

### 5. Process Cleanup

Always ensure processes are cleaned up:

```typescript
try {
  // Demo logic
} catch (error) {
  console.error('Demo failed:', error);
  shell.kill();  // CRITICAL: Kill shell on error
  throw error;
}
```

## Complete Working Examples

### Example 1: Simple Shell Commands

```typescript
import { spawn } from 'child_process';

export async function simpleCommandsDemo() {
  console.log('Starting simple commands demo...');

  const shell = spawn('bash', ['-i'], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      PS1: '$ ',
      FORCE_COLOR: '1',
    },
  });

  const write = (text: string) => shell.stdin.write(text);
  const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

  try {
    await delay(1000);

    console.log('Demo: Check location...');
    write('pwd\n');
    await delay(2000);

    console.log('Demo: List files...');
    write('ls\n');
    await delay(2000);

    console.log('Demo: Show message...');
    write('echo "Demo complete!"\n');
    await delay(2000);

    console.log('Demo: Exiting...');
    write('exit\n');
    await delay(1000);

    await new Promise<void>((resolve, reject) => {
      shell.on('exit', (code) => {
        if (code === 0) {
          console.log('Demo completed successfully!');
          resolve();
        } else {
          reject(new Error(`Shell exited with code ${code}`));
        }
      });
    });
  } catch (error) {
    console.error('Demo failed:', error);
    shell.kill();
    throw error;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  simpleCommandsDemo().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}
```

### Example 2: Interactive TUI Application

```typescript
import { spawn } from 'child_process';
import { join } from 'path';
import { homedir } from 'os';

export async function tuiAppDemo() {
  console.log('Starting TUI app demo...');

  // Path to the TUI application
  const appPath = join(homedir(), 'projects', 'my-app', 'dist', 'index.js');

  const shell = spawn(process.execPath, [appPath], {
    cwd: join(homedir(), 'projects', 'my-app'),
    env: {
      ...process.env,
      FORCE_COLOR: '1',
    },
  });

  const write = (text: string) => shell.stdin.write(text);
  const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

  try {
    // Wait for TUI to initialize
    await delay(2000);

    console.log('Demo: Navigate down...');
    write('j');  // Vim-style down
    await delay(500);
    write('j');
    await delay(500);

    console.log('Demo: Select item...');
    write('\r');  // Enter key
    await delay(1500);

    console.log('Demo: Show help...');
    write('?');  // Help screen
    await delay(2000);

    console.log('Demo: Close help...');
    write('\x1b');  // ESC key
    await delay(1000);

    console.log('Demo: Quit...');
    write('q');
    await delay(1000);

    await new Promise<void>((resolve, reject) => {
      shell.on('exit', (code) => {
        if (code === 0) {
          console.log('Demo completed successfully!');
          resolve();
        } else {
          reject(new Error(`Shell exited with code ${code}`));
        }
      });
    });
  } catch (error) {
    console.error('Demo failed:', error);
    shell.kill();
    throw error;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  tuiAppDemo().catch(error => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}
```

## Special Characters Reference

```typescript
// Keyboard keys
write('\r');      // Enter
write('\x1b');    // ESC
write('\t');      // Tab
write('\x7f');    // Backspace
write('\x03');    // Ctrl+C

// Arrow keys (ANSI escape sequences)
write('\x1b[A');  // Up arrow
write('\x1b[B');  // Down arrow
write('\x1b[C');  // Right arrow
write('\x1b[D');  // Left arrow

// Vim-style navigation
write('j');       // Down
write('k');       // Up
write('h');       // Left
write('l');       // Right
```

## Testing Demo Scripts

### Running Directly

```bash
# Test the demo script before recording
npx tsx src/demos/my-demo.ts

# Should see console.log output and no errors
```

### Recording with fspec.videos

```bash
# Record the demo
./dist/index.js record -s src/demos/my-demo.ts -o recordings/my-demo.webm

# Check the recording
open recordings/my-demo.webm
```

## Common Issues and Solutions

### Issue: "Module does not provide export"

**Cause:** Trying to import non-existent exports from @microsoft/tui-test

**Solution:** Use `spawn` from `child_process` instead:
```typescript
import { spawn } from 'child_process';  // ✅ Correct
import { TestController } from '@microsoft/tui-test';  // ❌ Wrong
```

### Issue: Demo hangs/doesn't exit

**Cause:** Shell process not terminated properly

**Solution:** Always write `exit\n` and wait for exit event:
```typescript
write('exit\n');
await delay(1000);

await new Promise<void>((resolve, reject) => {
  shell.on('exit', (code) => {
    // Handle exit
  });
});
```

### Issue: Commands execute too fast

**Cause:** Not enough delay between commands

**Solution:** Increase delays:
```typescript
write('command\n');
await delay(2000);  // Increase from 1000 to 2000
```

### Issue: "Demo script exited with code 1"

**Cause:** Error thrown in demo script

**Solution:** Check console.log output, add try/catch, ensure proper cleanup:
```typescript
try {
  // Demo logic
} catch (error) {
  console.error('Demo failed:', error);
  shell.kill();
  throw error;
}
```

## File Naming Conventions

```
src/demos/
├── simple-commands.ts      # Basic shell commands
├── fspec-navigation.ts     # fspec TUI navigation
├── fspec-work-units.ts     # fspec work unit demo
└── full-workflow.ts        # Complete ACDD workflow
```

**Guidelines:**
- Use kebab-case
- Descriptive names (what the demo shows)
- Suffix with `.ts`
- Export function name matches file: `simpleCommands`, `fspecNavigation`

## Summary

✅ **DO:**
- Use `spawn` from `child_process`
- Add console.log statements
- Include proper delays
- Handle errors with try/catch
- Clean up processes with shell.kill()
- Export async function
- Support direct execution with `if (import.meta.url === ...)`

❌ **DON'T:**
- Import from @microsoft/tui-test
- Forget to wait for shell initialization
- Skip error handling
- Forget to exit the shell
- Use synchronous code (always use async/await)

## References

- Node.js child_process: https://nodejs.org/api/child_process.html
- ANSI escape codes: https://en.wikipedia.org/wiki/ANSI_escape_code
