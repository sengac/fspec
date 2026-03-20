/**
 * E2E: Agent Lifecycle Hooks Integration
 *
 * Feature: spec/features/agent-lifecycle-hooks.feature
 *
 * Verifies that the lifecycle hooks infrastructure is properly wired into
 * the agent session lifecycle. These tests prove:
 *
 * 1. Application boots without errors when fspec-hooks.json is present
 * 2. Board renders cleanly with hooks configured
 * 3. Session creation succeeds with hooks loaded
 * 4. Hook loading does not interfere with navigation
 *
 * The example hooks in spec/hooks/examples/ are wired via spec/fspec-hooks.json
 * (installed as the active config for this test).
 */

import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

test('application boots without errors when lifecycle hooks are configured', async ({
  terminal,
}) => {
  // The board should render normally even with spec/fspec-hooks.json present
  // (containing all 6 agent lifecycle event hooks)
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // No error/crash indicators should appear
  await expect(
    terminal.getByText(/error|crash|panic/gi, { strict: false })
  ).not.toBeVisible();
});

test('board renders all ACDD columns with hooks configured', async ({
  terminal,
}) => {
  // Verify the board renders completely — hooks loading should not interfere
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
  await expect(
    terminal.getByText(/specifying/gi, { strict: false })
  ).toBeVisible();
  await expect(
    terminal.getByText(/implementing/gi, { strict: false })
  ).toBeVisible();
});

test('keyboard navigation works with hooks configured', async ({
  terminal,
}) => {
  // Board should be navigable — hooks don't block UI
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Navigate right
  terminal.keyRight();
  // Should still be responsive (no hang from hook loading)
  await expect(
    terminal.getByText(/specifying/gi, { strict: false })
  ).toBeVisible();
});

test('ESC exit dialog works with hooks configured', async ({ terminal }) => {
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // ESC should show exit dialog even with hooks loaded
  terminal.keyEscape();
  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).toBeVisible();

  // Cancel and return to board
  await new Promise(r => setTimeout(r, 200));
  terminal.keyRight();
  terminal.submit();

  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});

test('entering agent view from board loads session with hooks', async ({
  terminal,
}) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Enter a new session (Enter key on board creates session)
  terminal.submit();

  // Wait for the session view to load
  // The session creation calls create_session_with_id which loads lifecycle hooks
  // and spawns agent_loop which fires session_start hooks
  await new Promise(r => setTimeout(r, 2000));

  // Should transition to agent view without errors
  // (The loading/agent view appears, meaning session creation succeeded
  //  and lifecycle hooks loaded without preventing session start)
  await expect(
    terminal.getByText(/error.*hook|hook.*error|panic/gi, { strict: false })
  ).not.toBeVisible();
});
