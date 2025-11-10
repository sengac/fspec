# @microsoft/tui-test Usage Guide for Demo Scripts

## What is @microsoft/tui-test?

@microsoft/tui-test is a **test framework** for terminal applications, NOT a library for programmatic terminal control. It provides:

- `test()` - Function to define tests
- `expect()` - Assertion library (from Jest)
- `terminal` fixture - Terminal instance passed to each test
- Test runner - CLI tool that executes test files

## Critical Understanding

**The Terminal class is NOT exported for direct use.**

You cannot do:
```typescript
// ❌ WRONG - TestController doesn't exist
import { TestController } from '@microsoft/tui-test';

// ❌ WRONG - Terminal is not exported for direct instantiation
import { Terminal } from '@microsoft/tui-test';
const terminal = new Terminal(...);
```

**Instead, you must write TEST FILES** that use the framework:
```typescript
// ✅ CORRECT - Write test files
import { test, expect } from '@microsoft/tui-test';

test('my demo', async ({ terminal }) => {
  // terminal is provided as a fixture
  terminal.write('hello');
  await expect(terminal.getByText('hello')).toBeVisible();
});
```

## How Demo Scripts Should Work

Demo scripts for fspec.videos should be **tui-test TEST FILES**, not standalone scripts. The recording workflow becomes:

1. Write demo as a `.test.ts` file using tui-test framework
2. Run the test file with tui-test CLI (which spawns terminal and executes test)
3. Capture the terminal output during test execution
4. Save as video file

## Terminal Fixture API

When you write a test, tui-test provides a `terminal` object with these methods:

### Writing to Terminal

```typescript
// Write text to terminal (no newline)
terminal.write('hello');

// Write text and submit with newline/return
terminal.submit('ls -la');

// Submit empty (just press Enter)
terminal.submit();
```

### Keyboard Input

```typescript
// Arrow keys
terminal.keyUp();        // Up arrow
terminal.keyDown();      // Down arrow
terminal.keyLeft();      // Left arrow
terminal.keyRight();     // Right arrow

// With count
terminal.keyDown(3);     // Press down 3 times

// Special keys
terminal.keyEscape();    // ESC key
terminal.keyBackspace(); // Backspace
terminal.keyDelete();    // Delete
terminal.keyCtrlC();     // Ctrl+C
terminal.keyCtrlD();     // Ctrl+D
```

### Assertions

```typescript
// Wait for text to appear
await expect(terminal.getByText('Welcome')).toBeVisible();

// Check text is NOT visible
await expect(terminal.getByText('Error')).not.toBeVisible();

// Regex matching
await expect(terminal.getByText(/total \d+/)).toBeVisible();

// Color assertions
await expect(terminal.getByText('>')).toHaveFgColor('#FFFFFF');
await expect(terminal.getByText('>')).toHaveBgColor([0, 0, 0]);

// Snapshot testing
await expect(terminal).toMatchSnapshot();
```

### Query Options

```typescript
// Search full buffer (not just visible area)
terminal.getByText('text', { full: true });

// Non-strict mode (allow multiple matches)
terminal.getByText('text', { strict: false });
```

### Terminal Info

```typescript
// Get cursor position
const { x, y, baseY } = terminal.getCursor();

// Get buffer content
const buffer = terminal.getBuffer();          // Full buffer
const visible = terminal.getViewableBuffer(); // Visible portion only

// Resize terminal
terminal.resize(120, 30);

// Kill terminal
terminal.kill();
```

## Test Configuration

### Using `test.use()`

Configure the test environment:

```typescript
import { test, expect, Shell } from '@microsoft/tui-test';

// Use specific shell
test.use({ shell: Shell.Bash });
test.use({ shell: Shell.Powershell });
test.use({ shell: Shell.Zsh });

// Run a specific program instead of shell
test.use({
  program: {
    file: 'git',
    args: ['status']
  }
});

// Run node script
test.use({
  program: {
    file: 'node',
    args: ['/path/to/script.js']
  }
});
```

### Config File

Create `tui-test.config.ts` in project root:

```typescript
import { defineConfig } from '@microsoft/tui-test';

export default defineConfig({
  retries: 3,        // Retry failed tests
  trace: true,       // Enable tracing
  timeout: 30000,    // Test timeout in ms
});
```

## Complete Demo Script Examples

### Example 1: Shell Commands Demo

```typescript
// demos/shell-commands.test.ts
import { test, expect, Shell } from '@microsoft/tui-test';

test.use({ shell: Shell.Bash });

test('demonstrate shell commands', async ({ terminal }) => {
  // Wait for shell prompt
  await expect(terminal.getByText('$', { full: true })).toBeVisible();

  // Show current directory
  terminal.submit('pwd');
  await expect(terminal.getByText('/Users/', { full: true })).toBeVisible();

  // List files
  terminal.submit('ls');
  await expect(terminal.getByText('package.json')).toBeVisible();

  // Exit shell
  terminal.submit('exit');
});
```

### Example 2: TUI Application Demo (fspec)

```typescript
// demos/fspec-navigation.test.ts
import { test, expect } from '@microsoft/tui-test';
import { join } from 'path';
import { homedir } from 'os';

const fspecPath = join(homedir(), 'projects', 'fspec', 'dist', 'index.js');

test.use({
  program: {
    file: 'node',
    args: [fspecPath]
  }
});

test('demonstrate fspec navigation', async ({ terminal }) => {
  // Wait for fspec TUI to load
  await expect(terminal.getByText('fspec', { full: true })).toBeVisible();

  // Navigate down through work units
  terminal.keyDown();
  terminal.keyDown();
  terminal.keyDown();

  // Navigate back up
  terminal.keyUp();

  // Open work unit (press Enter)
  terminal.submit();

  // Wait for details to load
  await expect(terminal.getByText('Description:', { full: true })).toBeVisible();

  // Go back (ESC)
  terminal.keyEscape();

  // Quit (q)
  terminal.write('q');
});
```

### Example 3: Interactive Program Demo

```typescript
// demos/interactive-demo.test.ts
import { test, expect } from '@microsoft/tui-test';

test.use({ program: { file: 'vim', args: [] } });

test('demonstrate vim navigation', async ({ terminal }) => {
  // Wait for vim to load
  await expect(terminal.getByText('VIM', { full: true })).toBeVisible();

  // Enter insert mode
  terminal.write('i');

  // Type some text
  terminal.write('Hello, World!');

  // Exit insert mode
  terminal.keyEscape();

  // Save and quit
  terminal.write(':wq test.txt');
  terminal.submit();
});
```

### Example 4: Multi-Step Workflow Demo

```typescript
// demos/git-workflow.test.ts
import { test, expect, Shell } from '@microsoft/tui-test';

test.use({ shell: Shell.Bash });

test('demonstrate git workflow', async ({ terminal }) => {
  // Check git status
  terminal.submit('git status');
  await expect(terminal.getByText('On branch', { full: true })).toBeVisible();

  // Create a file
  terminal.submit('echo "test" > demo.txt');

  // Add file
  terminal.submit('git add demo.txt');
  await expect(terminal.getByText('$')).toBeVisible();

  // Check status again
  terminal.submit('git status');
  await expect(terminal.getByText('new file:', { full: true })).toBeVisible();

  // Commit
  terminal.submit('git commit -m "Add demo file"');
  await expect(terminal.getByText('1 file changed', { full: true })).toBeVisible();

  // Clean up
  terminal.submit('git reset HEAD~1');
  terminal.submit('rm demo.txt');
  terminal.submit('exit');
});
```

## Running Demo Tests

### Via npx

```bash
# Run all demos
npx @microsoft/tui-test demos/*.test.ts

# Run specific demo
npx @microsoft/tui-test demos/fspec-navigation.test.ts

# With tracing enabled
npx @microsoft/tui-test demos/fspec-navigation.test.ts --trace
```

### Integration with fspec.videos

The recorder should:

1. Accept a demo test file path
2. Spawn tui-test CLI to run the test
3. Capture the terminal output
4. Convert to video

```bash
# Current (broken)
fspec-videos record -s src/demos/basic-usage.ts

# Should become (working)
fspec-videos record -s demos/fspec-navigation.test.ts
```

## Key Differences from Previous Approach

| Aspect | ❌ Old (Broken) | ✅ New (Correct) |
|--------|----------------|-----------------|
| Import | `import { TestController }` | `import { test, expect }` |
| Execution | Standalone script | Test file run by tui-test |
| Terminal access | `new TestController()` | `test('...', async ({ terminal }) => ...)` |
| File extension | `.ts` | `.test.ts` |
| File location | `src/demos/` | `demos/` or `test/demos/` |
| Running | `npx tsx script.ts` | `npx @microsoft/tui-test script.test.ts` |

## Architecture Insights from Source Code

From studying `/tmp/tui-test/src/`:

### Terminal Class (`src/terminal/term.ts`)
- Uses `node-pty` to spawn shell/program
- Uses `@xterm/headless` for terminal emulation
- Manages PTY lifecycle
- Provides write(), submit(), key*() methods
- Handles buffer access and cursor tracking

### Test Framework (`src/test/test.ts`)
- Provides `test()` function to register tests
- Creates Suite and TestCase instances
- Passes `terminal` fixture to test functions
- Integrates with Jest `expect()` library
- Adds custom matchers: toBeVisible, toHaveBgColor, toHaveFgColor, toMatchSnapshot

### Test Runner (`src/runner/`)
- CLI tool that discovers `.test.ts` files
- Spawns terminal for each test
- Executes test function with terminal fixture
- Collects results and generates reports

## Common Patterns

### Waiting for Output

```typescript
// Wait for specific text to appear
await expect(terminal.getByText('Ready')).toBeVisible({ timeout: 5000 });

// Wait by submitting empty command (flush output)
terminal.submit();
await expect(terminal.getByText('$')).toBeVisible();
```

### Timing and Delays

tui-test handles timing automatically with `toBeVisible()`, but you can add explicit waits:

```typescript
// Using toBeVisible with timeout
await expect(terminal.getByText('Loading...'))
  .toBeVisible({ timeout: 10000 });

// Manual delay (not recommended - use toBeVisible instead)
await new Promise(resolve => setTimeout(resolve, 1000));
```

### Error Handling

```typescript
test('handles errors gracefully', async ({ terminal }) => {
  try {
    terminal.submit('invalid-command');

    // Expect error message
    await expect(terminal.getByText('command not found'))
      .toBeVisible({ timeout: 3000 });
  } catch (error) {
    // Test will fail if assertion times out
    throw error;
  }
});
```

## Debugging

### Enable Tracing

```bash
npx @microsoft/tui-test demos/my-demo.test.ts --trace
```

Traces are saved to `tui-traces/` directory.

### View Trace

```bash
npx @microsoft/tui-test show-trace tui-traces/my-demo-trace.json
```

### Manual Debugging

```typescript
test('debug terminal state', async ({ terminal }) => {
  terminal.submit('ls');

  // Print cursor position
  console.log('Cursor:', terminal.getCursor());

  // Print buffer
  console.log('Buffer:', terminal.getBuffer());

  // Take snapshot
  const { view, shifts } = terminal.serialize();
  console.log('Snapshot:', view);
});
```

## Best Practices

1. **Use test.use() for configuration** - Set shell or program at the top
2. **Wait for visibility** - Always use `await expect(...).toBeVisible()`
3. **Use full: true for reliability** - Search entire buffer, not just visible area
4. **Name tests descriptively** - Explain what the demo shows
5. **Keep demos focused** - One workflow per test file
6. **Clean up state** - Exit programs, delete temp files
7. **Use timeouts** - Set reasonable timeouts for slow operations

## Migration Path for Existing Demos

For each existing demo in `src/demos/`:

1. **Rename** `src/demos/basic-usage.ts` → `demos/basic-usage.test.ts`
2. **Replace imports**:
   ```typescript
   // Old
   import { TestController } from '@microsoft/tui-test';

   // New
   import { test, expect } from '@microsoft/tui-test';
   ```
3. **Wrap in test function**:
   ```typescript
   // Old
   export async function basicUsageDemo() {
     const controller = new TestController();
     // ...
   }

   // New
   test('basic usage demo', async ({ terminal }) => {
     // ...
   });
   ```
4. **Replace controller methods**:
   ```typescript
   // Old
   await controller.spawn('bash', ...);
   await controller.write('ls\n');
   await controller.waitForText('$');

   // New
   terminal.submit('ls');
   await expect(terminal.getByText('$')).toBeVisible();
   ```
5. **Remove main execution block** - tui-test runs tests automatically
6. **Test the demo**: `npx @microsoft/tui-test demos/basic-usage.test.ts`

## Summary

✅ **DO:**
- Write demo scripts as `.test.ts` files
- Use `test()` and `expect()` from @microsoft/tui-test
- Access terminal via fixture: `test('...', async ({ terminal }) => ...)`
- Use `test.use()` to configure shell or program
- Use `await expect(terminal.getByText(...)).toBeVisible()` for waiting
- Run with `npx @microsoft/tui-test demos/my-demo.test.ts`

❌ **DON'T:**
- Try to import `TestController` (doesn't exist)
- Try to instantiate `Terminal` directly (not exported)
- Write standalone scripts - must be test files
- Use manual delays - use `toBeVisible()` instead
- Run with `npx tsx` - use tui-test CLI

## References

- GitHub: https://github.com/microsoft/tui-test
- Examples: https://github.com/microsoft/tui-test/tree/main/examples
- Source (studied): `/tmp/tui-test/src/`
