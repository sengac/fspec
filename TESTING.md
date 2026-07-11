# Testing Guide

This document describes how we write and run tests in fspec. It covers our testing
philosophy, the three test runners we use, the fixture/helper infrastructure, and the
concrete patterns every contributor should follow.

---

## Table of Contents

1. [Philosophy](#philosophy)
2. [Test Runners & Configuration](#test-runners--configuration)
3. [Running Tests](#running-tests)
4. [Rust Workspace Tests (codelet/) — Read Before Running](#rust-workspace-tests-codelet--read-before-running)
5. [Test Pyramid](#test-pyramid)
6. [Filesystem Helpers & Fixtures](#filesystem-helpers--fixtures)
7. [TUI Component Testing (ink-testing-library)](#tui-component-testing-ink-testing-library)
8. [Terminal E2E Testing (@microsoft/tui-test)](#terminal-e2e-testing-microsofttui-test)
9. [Integration Tests Without Mocks](#integration-tests-without-mocks)
10. [When Mocks Are Appropriate](#when-mocks-are-appropriate)
11. [Builder / Factory Fixtures](#builder--factory-fixtures)
12. [ACDD Test Structure Convention](#acdd-test-structure-convention)
13. [Animation & Timer Testing](#animation--timer-testing)
14. [Troubleshooting](#troubleshooting)

---

## Philosophy

Our tests follow four guiding principles:

| Principle | In Practice |
|-----------|-------------|
| **Integration over mocks** | Prefer real filesystems, real stores, and real NAPI calls. Mock only at system boundaries (network, native APIs). |
| **SOLID / DRY / Composable** | Every fixture file has a single domain. Base fixtures compose into richer ones. No copy-paste setup code. |
| **Redirect, don't intercept** | Control *inputs* (temp directories, `process.env.HOME`) rather than replacing *code paths* (`vi.mock`). |
| **ACDD compliance** | Every test file links back to a `.feature` file and uses `@step` comments to map Gherkin steps to code. |

> **"Don't mock what you can redirect."**
>
> By controlling filesystem contents and environment variables instead of
> intercepting module internals, our tests exercise the same code paths that run
> in production.

---

## Test Runners & Configuration

We use **three** separate test runners, each with its own configuration.

### 1. Vitest — Unit & Integration Tests

| Setting | Value | Why |
|---------|-------|-----|
| Config | `vitest.config.ts` | Primary config |
| Environment | `jsdom` | Needed for React/Ink component rendering |
| Pool | `forks` (`singleFork: true`) | Single process prevents file conflicts and memory leaks |
| Parallelism | Disabled (`fileParallelism: false`, `maxConcurrency: 1`) | Fully sequential — tests share OS temp space |
| Timeouts | 30 s test, 30 s hook | Generous for NAPI/filesystem work |
| Includes | `src/**/*.test.ts`, `src/**/*.test.tsx`, `bridge/**/*.test.ts`, `extension/**/*.test.ts` | All non-E2E tests |
| Excludes | `**/*e2e*`, `**/*E2E*` | E2E tests run separately |
| Env | `FORCE_COLOR=0` | Consistent non-colorized output for assertions |

### 2. Vitest — E2E Tests (Live API)

| Setting | Value |
|---------|-------|
| Config | `vitest.e2e.config.ts` |
| Timeouts | **240 s** test, **60 s** hook (live LLM calls) |
| Includes | `src/**/*e2e*.test.ts`, `src/**/*E2E*.test.ts` |

### 3. @microsoft/tui-test — Terminal UI Tests

| Setting | Value |
|---------|-------|
| Config | `tui-test.config.ts` |
| Test match | `e2e/**/*.test.ts` |
| Retries | 1 |
| Trace | Enabled (saved to `tui-traces/`) |
| Timeout | 60 s per test, 15 s per assertion |

---

## Running Tests

| Command | What It Does |
|---------|-------------|
| `npm test` | Build + run all unit/integration tests |
| `npm run test:e2e` | Build + run E2E tests (needs API credentials) |
| `npm run test:tui` | Build + run terminal UI tests |
| `npm run test:tui:trace` | Same, with byte-level trace recording |
| `npm run test:watch` | Watch mode (no build step) |
| `npm run test:extension` | Extension-only tests |

> **All commands except `test:watch` run `npm run build` first.** Tests run
> against the compiled bundle, not raw TypeScript.

---

## Rust Workspace Tests (codelet/) — Read Before Running

> ⚠️ **NEVER run `cargo test --workspace` (or a bare `cargo test`) from
> `codelet/`.**
>
> **Incident 2026-07-10:** a plain `cargo test --workspace` compiled all 944
> integration-test binaries in the workspace with full DWARF debug info —
> **1.4–2 GB per binary**, because every test binary statically links the full
> crate graph (arrow, datafusion, lance, tantivy). `target/debug/deps` grew to
> **299 GB** and the machine crashed mid-link.

### Safe invocation patterns

| Goal | Command |
|------|---------|
| One crate's test target (preferred) | `cargo test -p <crate> --test <name>` |
| One whole crate | `cargo test -p <crate>` |
| Several crates | `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p <a> -p <b> -j 12 --no-fail-fast` |
| CI-style bounded run | `cargo test --profile ci-test -p <crate>` |

Rules of thumb:

1. **Always scope with `-p`** — never let cargo expand to the whole workspace.
2. **Drop debug info for broad runs** with
   `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` (both are needed:
   deps build under the `dev` profile, test binaries under `test`).
3. **Tee output to a file** (`cargo test ... 2>&1 | tee /tmp/test-out.txt`)
   and read the file — never re-run an expensive suite just to see a
   different slice of the output.
4. The disk-bloat background, the `ci-test` profile, and
   `incremental = false` on `[profile.test]` are documented in
   `codelet/Cargo.toml` (RPC-043 + the 2026-07-10 incident note).
5. These rules are enforced at runtime by `~/.fspec/blocklist.json`
   (`cargo-test-workspace-block` blocks `--workspace`/`--all`;
   `cargo-test-unscoped-prompt` prompts on a bare `cargo test`).

---

## Test Pyramid

```
┌──────────────────────────────────────────────────┐
│ Terminal E2E  (tui-test)                          │  Real PTY, real process,
│ 4 test files — board, navigation, sessions        │  headless xterm.js
├──────────────────────────────────────────────────┤
│ Integration Tests  (~28 files, ZERO vi.mock)      │  Real FS, real stores,
│ Real NAPI, real IPC, real shell scripts            │  real Ink rendering
├──────────────────────────────────────────────────┤
│ Component Tests  (~45 files)                      │  ink-testing-library render()
│ lastFrame() + stdin.write() + rerender()          │  Store state set directly
├──────────────────────────────────────────────────┤
│ Hook Tests  (~10 files)                           │  TestHarness wrappers,
│ Handler capture via onStateChange callbacks        │  vi.fn() for event sinks only
├──────────────────────────────────────────────────┤
│ Unit Tests  (bulk of 500+ files)                  │  Pure logic, builder fixtures,
│ Fast, isolated, typed                              │  memfs for filesystem logic
├──────────────────────────────────────────────────┤
│ Shared Helpers & Fixtures  (~15 files)            │  Composable, lifecycle-managed,
│ SOLID/DRY/Builder pattern                          │  auto-cleanup
└──────────────────────────────────────────────────┘
```

---

## Filesystem Helpers & Fixtures

All filesystem-based tests use a layered helper system located in `src/test-helpers/`.
The core idea: **create real files in OS temp directories, run production code against
them, then clean up.**

### Composition Hierarchy

```
universal-test-setup.ts         ← Primary entry point for most tests
├── temp-directory.ts           ← OS temp dir creation/removal (with safety guards)
├── test-file-operations.ts     ← JSON/text file read/write primitives
└── work-unit-test-fixtures.ts  ← work-units.json, prefixes.json, epics.json

home-directory-fixture.ts       ← HOME override + ~/.fspec config/credentials CRUD
└── provider-profile-fixtures.ts  ← Composes home-dir + mock server registry

foundation-fixtures.ts          ← In-memory foundation schema builders
└── foundation-with-diagram-fixtures.ts
```

### Temp Directories — `temp-directory.ts`

Creates uniquely-named directories under `os.tmpdir()` to avoid polluting the project.

```typescript
import { createTempTestDir, removeTempTestDir } from '../test-helpers/temp-directory';

let testDir: string;

beforeEach(async () => {
  testDir = await createTempTestDir('my-feature-test');
  // Creates: /tmp/fspec-test-my-feature-test-1710729600000-abc123/
  //          /tmp/fspec-test-my-feature-test-1710729600000-abc123/spec/
});

afterEach(async () => {
  await removeTempTestDir(testDir);
});
```

**Safety guards:** `removeTempTestDir` refuses to delete directories that don't match
`fspec-test-*`, `test-temp-*`, or aren't in the OS temp folder.

### File Operations — `test-file-operations.ts`

Low-level, reusable I/O primitives:

```typescript
import {
  createTestFiles,
  readJsonTestFile,
  writeJsonTestFile,
} from '../test-helpers/test-file-operations';

// Bulk-create multiple files at once
const files = await createTestFiles(testDir, {
  'spec/work-units.json': { data: { workUnits: {}, states: { backlog: [] } } },
  'spec/features/login.feature': { content: 'Feature: Login\n  Scenario: ...' },
});

// Read back and assert
const data = await readJsonTestFile<WorkUnitsData>(files['spec/work-units.json']);
expect(data.workUnits).toEqual({});
```

### Universal Test Setup — `universal-test-setup.ts`

Tiered setup functions with increasing complexity:

| Function | Creates |
|----------|---------|
| `setupTestDirectory(name)` | Temp dir + `spec/` + empty `work-units.json` |
| `setupWorkUnitTest(name)` | Above + `prefixes.json` + `epics.json` + `features/` |
| `setupFoundationTest(name)` | Temp dir + `foundation.json` with full schema |
| `setupFullTest(name)` | Work units + foundation combined |
| `setupGitTest(name)` | Base + `initGit()` that creates a real git repo via NAPI |

Every setup returns a `cleanup()` function:

```typescript
import { setupWorkUnitTest } from '../test-helpers/universal-test-setup';

let env: WorkUnitTestSetup;

beforeEach(async () => {
  env = await setupWorkUnitTest('status-transitions');
});

afterEach(async () => {
  await env.cleanup();
});

it('should create a work unit', async () => {
  await createStory({ prefix: 'AUTH', title: 'Login', cwd: env.testDir });
  const data = await readJsonTestFile(env.workUnitsFile);
  expect(data.workUnits['AUTH-001']).toBeDefined();
});
```

### Home Directory Fixture — `home-directory-fixture.ts`

For testing anything that reads `~/.fspec` (credentials, provider config, profiles).
Overrides `process.env.HOME` to an isolated temp directory.

```typescript
import {
  createHomeDirectoryFixture,
  setupFullProviderEnvironment,
} from '../test-helpers/home-directory-fixture';

let homeFixture: HomeDirectoryFixture;

beforeEach(async () => {
  homeFixture = await createHomeDirectoryFixture({ testName: 'provider-test' });
  await setupFullProviderEnvironment(homeFixture);
  // Now ~/.fspec has Anthropic + Codex credentials and local profiles
});

afterEach(async () => {
  await homeFixture.cleanup(); // Restores original HOME
});

it('should load provider profiles', async () => {
  const profiles = await homeFixture.getProfiles('openai');
  expect(profiles['work-vllm']).toBeDefined();
  expect(profiles['work-vllm'].baseUrl).toBe('http://work:8888');
});
```

**Key design:** The fixture composes into higher-level fixtures. For example,
`provider-profile-fixtures.ts` composes `HomeDirectoryFixture` and adds mock
server registration.

---

## TUI Component Testing (ink-testing-library)

All TUI components are built with React/Ink and tested using `ink-testing-library`.
The `render()` function returns a terminal rendering sandbox.

### Core API

| Member | Purpose |
|--------|---------|
| `lastFrame()` | Most recent terminal text output |
| `frames` | Array of all rendered frames (full history) |
| `stdin` | Writable stream for simulating keyboard input |
| `rerender()` | Re-render with new JSX (prop/state transitions) |
| `unmount()` | Tear down the component tree |

### Pattern 1: Render + Assert on `lastFrame()`

The most common pattern — render a component and assert on its text output:

```typescript
import { render } from 'ink-testing-library';

it('should display role text', () => {
  const { lastFrame } = render(<RoleBanner roleText="security reviewer" />);
  expect(lastFrame()).toContain('Role:');
  expect(lastFrame()).toContain('security reviewer');
});
```

### Pattern 2: Keyboard Interaction via `stdin.write()`

Simulate user input using ANSI escape codes:

```typescript
import { render } from 'ink-testing-library';

it('should navigate with arrow keys', () => {
  const { stdin, lastFrame } = render(<FileList files={files} onExit={vi.fn()} />);

  stdin.write('\x1B[B'); // Down arrow
  stdin.write('\x1B[B'); // Down arrow
  stdin.write('\r');     // Enter

  expect(lastFrame()).toContain('selected: file-3');
});
```

Use the shared keyboard helpers for readability:

```typescript
import { KEY_CODES, pressKey } from './fixtures/keyboardHelpers';

stdin.write(KEY_CODES.down);
stdin.write(KEY_CODES.enter);
stdin.write(KEY_CODES.escape);
```

### Pattern 3: `rerender()` for State Transitions

Test how components respond to prop changes over time:

```typescript
it('should transition from loading to ready', () => {
  const { lastFrame, rerender } = render(
    <InputTransition isLoading={true} />
  );
  expect(lastFrame()).toContain('Thinking...');

  rerender(<InputTransition isLoading={false} />);
  expect(lastFrame()).toContain('Type a message...');
});
```

### Pattern 4: Store-Driven Components

Components backed by Zustand stores are tested by setting store state directly
in `beforeEach`:

```typescript
import { useFspecStore } from '../../store/fspecStore';

beforeEach(() => {
  useFspecStore.setState({
    stagedFiles: [],
    unstagedFiles: [],
    isLoaded: false,
  });
});

it('should show staged files', () => {
  useFspecStore.setState({
    stagedFiles: [{ filepath: 'src/auth.ts', changeType: 'A', staged: true }],
  });
  const { lastFrame } = render(<ChangedFilesViewer onExit={vi.fn()} />);
  expect(lastFrame()).toContain('src/auth.ts');
});
```

### Pattern 5: Hook Testing with TestHarness Wrappers

Since we can't use `renderHook` directly with Ink, hooks are tested by wrapping
them in a minimal component:

```typescript
const TestHarness: React.FC<Props> = ({ onStateChange, ...hookArgs }) => {
  const result = useMyHook(hookArgs);
  React.useEffect(() => { onStateChange?.(result); });
  return React.createElement('ink-text', null, `state: ${result.value}`);
};

it('should update state on input', () => {
  let lastResult: HookResult | null = null;
  render(
    <TestHarness
      onStateChange={(r) => { lastResult = r; }}
      initialValue="hello"
    />
  );
  expect(lastResult?.value).toBe('hello');
});
```

### Stripping ANSI Codes

When color codes interfere with content matching:

```typescript
const clean = lastFrame().replace(/\u001b\[[0-9;]*m/g, '');
expect(clean).toMatch(/>\s+A\s+src\/auth\.ts/);
```

---

## Terminal E2E Testing (@microsoft/tui-test)

For full end-to-end testing of the TUI, we use `@microsoft/tui-test`. This spawns
a **real PTY** with a headless `xterm.js` instance per test — ANSI escapes, cursor
movement, and colors are all fully interpreted.

### Test Structure

```typescript
import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' }, // Launch the compiled binary
  rows: 40,
  columns: 120,
});

test('board renders column headers', async ({ terminal }) => {
  // Wait for content (auto-polls every ~50ms, times out at 15s)
  await expect(terminal.getByText(/backlog/gi, { strict: false })).toBeVisible();
  await expect(terminal.getByText(/specifying/gi, { strict: false })).toBeVisible();
});
```

### Keyboard Interaction

```typescript
test('ESC opens exit dialog', async ({ terminal }) => {
  await expect(terminal.getByText(/backlog/gi, { strict: false })).toBeVisible();

  terminal.keyEscape();
  await expect(terminal.getByText(/Exit fspec/gi, { strict: false })).toBeVisible();

  terminal.keyRight(); // Move to "No"
  terminal.submit();   // Confirm
  await expect(terminal.getByText(/Exit fspec/gi, { strict: false })).not.toBeVisible();
});
```

### Key Points

- **Per-test isolation:** Each test gets a fresh PTY — no state leaks.
- **Program mode:** Tests launch `./dist/index.js` directly, not a shell.
- **Trace recording:** With `trace: true`, every byte is recorded with timestamps to
  `tui-traces/` for post-mortem debugging.
- **Locator-based assertions:** `getByText()` auto-polls the xterm buffer until the
  text appears or the timeout expires. No manual sleeps needed.

---

## Integration Tests Without Mocks

We have **28+ integration test files that use zero `vi.mock()` calls.** The core
philosophy is documented in our integration helper:

> *"NO mocks — tests real behavior and component coordination."*
> *"Dependency Inversion: Uses real implementations, not mocks."*

### Instead-of-Mocking Cheat Sheet

| Instead of Mocking… | We Use… |
|---|---|
| `fs` module | Real filesystem in `/tmp` sandboxes |
| Environment variables | Save/restore `process.env` fields |
| HOME directory | `process.env.HOME = testDir` |
| Credential loading | Real `credentials.json` files |
| Zustand stores | Real stores, reset via `setState()` between tests |
| NAPI/Rust bindings | Real compiled NAPI calls |
| IPC communication | Real Unix domain socket servers |
| Git operations | Real `gitInit`/`gitAdd`/`gitCommit` via libgit2 NAPI |
| Component behavior | Real `ink-testing-library` render + `stdin.write()` |
| Hook scripts | Real shell scripts with `chmod +x` |

### Example: Real NAPI Integration

```typescript
// No mocks — calls real compiled Rust through NAPI-RS
import { callFspecCommand } from '@sengac/codelet-napi';

it('should list work units via Rust FFI', () => {
  const result = callFspecCommand(
    'list-work-units', '{}', testDir,
    (command, argsJson, projectRoot) => {
      // This callback is invoked BY Rust
      const data = JSON.parse(
        fs.readFileSync(join(projectRoot, 'spec', 'work-units.json'), 'utf-8')
      );
      return JSON.stringify({ success: true, data });
    }
  );
  expect(JSON.parse(result).data.workUnits).toHaveLength(3);
});
```

### Example: Real Shell Script Hooks

```typescript
// Create a REAL executable script
await writeFile(
  join(testDir, 'spec/hooks/pre.sh'),
  '#!/bin/bash\necho "PRE-HOOK"',
  { mode: 0o755 }
);

const result = await runCommandWithHooks(
  'update-work-unit-status',
  { workUnitId: 'AUTH-001', newStatus: 'implementing' },
  commandFn,
  testDir
);
expect(result.preHookResults[0].stdout).toContain('PRE-HOOK');
```

### Example: Real IPC Sockets

```typescript
// Real Unix domain socket server
server = createIPCServer(message => { receivedMessages.push(message); });
server.listen(getIPCPath());

await checkpoint({ workUnitId: 'TUI-016', label: 'test', cwd: testDir });

expect(receivedMessages).toContainEqual(
  expect.objectContaining({ type: 'checkpoint-changed' })
);
```

---

## When Mocks Are Appropriate

Mocks are acceptable in **two** specific situations:

### 1. memfs for Command-Layer Filesystem Tests

When testing the full command write→read→modify→write cycle (including file locking),
`memfs` replaces `fs` and `fs/promises` to keep tests fast and hermetic:

```typescript
import { vol } from 'memfs';

vi.mock('fs/promises', async importOriginal => {
  const memfs = await vi.importActual<MemfsModule>('memfs');
  const actual = await importOriginal();
  return { ...actual, ...memfs.fs.promises, default: memfs.fs.promises };
});

vi.mock('fs', async importOriginal => {
  const memfs = await vi.importActual<MemfsModule>('memfs');
  const actual = await importOriginal();
  return { ...actual, ...memfs.fs, default: memfs.fs };
});

// Also mock proper-lockfile to use memfs
vi.mock('proper-lockfile', async () => {
  const actual = await vi.importActual<LockfileModule>('proper-lockfile');
  const memfs = await vi.importActual<MemfsModule>('memfs');
  return {
    lock: (file, opts = {}) => actual.lock(file, { ...opts, fs: memfs.fs, realpath: false }),
    unlock: (file, opts = {}) => actual.unlock(file, { ...opts, fs: memfs.fs, realpath: false }),
    // ... same for lockSync, unlockSync, check, checkSync
  };
});

beforeEach(() => {
  vol.reset(); // Clean slate per test
});
```

> **Important:** `vi.mock()` calls must come **before** imports of the modules
> under test, so Vitest can hoist them properly.

### 2. `vi.fn()` for Callback Event Sinks

Use `vi.fn()` **only** to capture callbacks — never to replace module behavior:

```typescript
const onChange = vi.fn();
const onSubmit = vi.fn();

render(<MultiLineInput value="" onChange={onChange} onSubmit={onSubmit} />);
stdin.write('hello');

expect(onChange).toHaveBeenCalledWith('hello');
expect(onSubmit).not.toHaveBeenCalled();
```

---

## Builder / Factory Fixtures

All fixture data follows the **builder pattern with `Partial<T>` override spreads**:

```typescript
// Base builder — sensible defaults
export function createTestModelInfo(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return {
    id: 'test-model',
    name: 'Test Model',
    contextWindow: 128000,
    ...overrides,
  };
}

// Specialized builder — composes the base
export function createClaudeModel(
  overrides: Partial<NapiModelInfo> = {}
): NapiModelInfo {
  return createTestModelInfo({
    id: 'claude-sonnet-4',
    reasoning: true,
    ...overrides,
  });
}
```

### Static Data Record Fixtures

For data-driven tests, use pre-defined record objects:

```typescript
export const compactionProgressFixtures: Record<string, CompactionProgress> = {
  analyzingContext: { phase: 'Analyzing context', current: 0, total: 1 },
  generatingSummary: { phase: 'Preparing compaction', current: 1, total: 1 },
  emergencyLarge: { phase: 'Emergency compaction', current: 0, total: 1 },
};

export const expectedCompactionTexts = {
  analyzingContext: 'Compacting: Analyzing context...',
  generatingSummary: 'Compacting: Preparing compaction...',
};
```

### Fixture File Index

| File | Domain | Key Exports |
|------|--------|-------------|
| `universal-test-setup.ts` | Test orchestration | `setupTestDirectory`, `setupWorkUnitTest`, `setupFullTest`, `setupGitTest` |
| `temp-directory.ts` | OS temp dirs | `createTempTestDir`, `removeTempTestDir` |
| `test-file-operations.ts` | File I/O | `createTestFiles`, `readJsonTestFile`, `writeJsonTestFile` |
| `work-unit-test-fixtures.ts` | Work units | `createWorkUnitTestEnvironment`, `registerTestPrefix` |
| `foundation-fixtures.ts` | Foundation schema | `createMinimalFoundation`, `createCompleteFoundation` |
| `home-directory-fixture.ts` | HOME override | `createHomeDirectoryFixture`, `setupFullProviderEnvironment` |
| `provider-profile-fixtures.ts` | Profiles | `createProviderProfileFixture`, `createStandardProfiles` |
| `provider-type-fixtures.ts` | UI models | `createTestModelInfo`, `createMultiProviderScenario` |
| `napi-model-fixtures.ts` | NAPI models | `createNapiModelInfo`, `createDefaultCloudProviders` |
| `compaction-fixtures.ts` | Compaction data | `compactionProgressFixtures`, `compactionScenarios` |
| `multiline-input-test-helpers.tsx` | Component rendering | `renderMultiLineInput`, `createCompactionLifecycleTest` |
| `compaction-integration-helpers.tsx` | Integration (no mocks) | `CompactionStateTransitionSimulator`, `createFullIntegrationEnvironment` |
| `source-code-analysis-fixtures.ts` | Architecture tests | `grepTypeScriptFiles`, `verifyTypeDefinition` |

---

## ACDD Test Structure Convention

Every test file **must** follow these conventions:

### 1. Feature File Header Comment

```typescript
/**
 * Feature: spec/features/user-authentication.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios map directly to Gherkin scenarios.
 */
```

### 2. Describe Blocks Mirror Gherkin

```typescript
describe('Feature: User Authentication', () => {
  describe('Scenario: Login with valid credentials', () => {
    it('should redirect to dashboard', async () => {
      // test body
    });
  });
});
```

### 3. `@step` Comments Are Mandatory

Every Gherkin step must have a corresponding `@step` comment:

```typescript
it('should redirect to dashboard', async () => {
  // @step Given I am on the login page
  await page.goto('/login');

  // @step When I enter valid credentials
  await page.fill('#email', 'user@example.com');
  await page.fill('#password', 'password123');
  await page.click('#submit');

  // @step Then I should see the dashboard
  expect(page.url()).toContain('/dashboard');
});
```

**Without `@step` comments, `fspec link-coverage` will block workflow progression.**

---

## Animation & Timer Testing

For animated components (spinners, transitions), use Vitest fake timers:

```typescript
beforeEach(() => { vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); });

it('should complete fade-in animation', async () => {
  const { lastFrame, rerender } = render(
    <InputTransition isLoading={true} />
  );

  rerender(<InputTransition isLoading={false} />);

  // Advance through animation frames
  for (let i = 0; i < 100; i++) {
    vi.advanceTimersByTime(20);
    await vi.runAllTimersAsync();
  }

  expect(lastFrame()).toContain('Type a message...');
});
```

---

## Troubleshooting

### Tests hang or timeout

- Check for floating promises (all promises must be `await`ed)
- Ensure `afterEach` calls `cleanup()` on all fixtures
- Vitest runs sequentially (`maxConcurrency: 1`) — a hanging test blocks everything

### ANSI codes in assertions

Strip them:
```typescript
const clean = output.replace(/\u001b\[[0-9;]*m/g, '');
```

### memfs tests fail with lockfile errors

Ensure `proper-lockfile` is also mocked to use `memfs.fs` with `realpath: false`.

### TUI tests can't find text

- Use `{ strict: false }` on `getByText()` (allows multiple matches)
- Use regex with `/gi` flags for case-insensitive matching
- Check `tui-traces/` for recorded byte streams

### Home directory tests leak state

Always restore `process.env.HOME` in `afterEach`. The `HomeDirectoryFixture.cleanup()`
method does this automatically.
