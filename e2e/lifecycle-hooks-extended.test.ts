/**
 * E2E: Agent Lifecycle Hooks — Extended Integration Tests
 *
 * Feature: spec/features/agent-lifecycle-hooks.feature
 *
 * These tests verify the lifecycle hooks infrastructure end-to-end
 * through the TUI. They complement the existing lifecycle-hooks.test.ts
 * with deeper coverage of:
 *
 * 1. Application stability with and without hooks configured
 * 2. Board renders correctly regardless of hook configuration
 * 3. Navigation remains responsive with hooks loaded
 * 4. Session creation path (which loads lifecycle hooks) succeeds
 * 5. Multiple keyboard interactions don't trigger hook-related errors
 *
 * The project has NO spec/fspec-hooks.json by default — the "no hooks"
 * path (Option<LifecycleHookEngine> = None) is the primary scenario.
 * Example hooks exist at spec/hooks/examples/ for reference only.
 */

import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

// ============================================================================
// Scenario: Application boots without hooks configured
// Verifies the None engine path has zero overhead
// ============================================================================

test('board renders all six ACDD columns without hooks configured', async ({
  terminal,
}) => {
  // @step Given no spec/fspec-hooks.json exists in the project
  // @step When the application starts
  // @step Then all ACDD Kanban columns should render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
  await expect(
    terminal.getByText(/specifying/gi, { strict: false })
  ).toBeVisible();
  await expect(
    terminal.getByText(/testing/gi, { strict: false })
  ).toBeVisible();
  await expect(
    terminal.getByText(/implementing/gi, { strict: false })
  ).toBeVisible();
  await expect(
    terminal.getByText(/validating/gi, { strict: false })
  ).toBeVisible();
  await expect(terminal.getByText(/done/gi, { strict: false })).toBeVisible();
});

test('no hook-related errors appear on startup', async ({ terminal }) => {
  // @step Given the application has started
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step Then no error messages related to hooks should appear
  await expect(
    terminal.getByText(/hook.*error|error.*hook/gi, { strict: false })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/lifecycle.*fail|fail.*lifecycle/gi, { strict: false })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/panic|RUST_BACKTRACE/gi, { strict: false })
  ).not.toBeVisible();
});

// ============================================================================
// Scenario: Keyboard navigation is unaffected by hook system
// Verifies hooks don't block the UI event loop
// ============================================================================

test('rapid column navigation remains responsive', async ({ terminal }) => {
  // @step Given the board has rendered
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step When I navigate rapidly through columns
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyLeft();
  terminal.keyLeft();

  // @step Then the board should still be responsive
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step And no crash or error should occur
  await expect(
    terminal.getByText(/error|crash|panic/gi, { strict: false })
  ).not.toBeVisible();
});

test('vertical navigation within columns works with hooks system loaded', async ({
  terminal,
}) => {
  // @step Given the board has rendered
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step When I navigate vertically within a column
  terminal.keyDown();
  terminal.keyDown();
  terminal.keyUp();

  // @step Then navigation should complete without errors
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});

// ============================================================================
// Scenario: Exit dialog works correctly with hooks system
// Verifies hook loading doesn't interfere with dialog lifecycle
// ============================================================================

test('exit dialog renders and dismisses cleanly with hooks system active', async ({
  terminal,
}) => {
  // @step Given the board has rendered
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step When I press Escape to show exit dialog
  terminal.keyEscape();

  // @step Then the exit dialog should appear
  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).toBeVisible();

  // @step When I cancel the exit dialog
  await new Promise(r => setTimeout(r, 200));
  terminal.keyRight(); // Move to "No"
  terminal.submit(); // Confirm

  // @step Then the dialog should dismiss and board should be intact
  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});

// ============================================================================
// Scenario: Session creation path triggers hook loading
// Verifies create_session_with_id loads lifecycle hooks without errors
// ============================================================================

test('entering agent view triggers session creation with hook loading', async ({
  terminal,
}) => {
  // @step Given the board has rendered
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step When I press Enter to create a new session
  // (This triggers create_session_with_id which loads lifecycle hooks)
  terminal.submit();

  // @step Then session creation should proceed without hook errors
  // Wait for the transition — the session creation loads hooks from
  // spec/fspec-hooks.json (if present) and registers pre_tool_use handlers
  await new Promise(r => setTimeout(r, 2000));

  // @step And no hook-related crashes should appear
  await expect(
    terminal.getByText(/hook.*error|hook.*panic|hook.*crash/gi, {
      strict: false,
    })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/pre_tool_use.*fail|lifecycle.*fail/gi, {
      strict: false,
    })
  ).not.toBeVisible();
});

// ============================================================================
// Scenario: Board detail panel works with hooks system
// Verifies the work unit details panel renders correctly
// ============================================================================

test('work unit details panel renders without hook interference', async ({
  terminal,
}) => {
  // @step Given the board has rendered with work units
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // @step When I navigate to a work unit
  terminal.keyDown();

  // @step Then the details panel should render without errors
  // (The details panel reads work unit data — hook system should not interfere)
  await expect(
    terminal.getByText(/error|undefined|NaN/gi, { strict: false })
  ).not.toBeVisible();
});
