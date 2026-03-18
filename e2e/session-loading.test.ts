/**
 * E2E: Session Loading Performance
 *
 * Tests the "Loading..." state in SessionHeader when transitioning
 * from BoardView to AgentView. Uses tui-test traces to capture
 * exact timing of what renders and when.
 *
 * Two scenarios:
 * 1. New session (no prior session) — measures model init delay
 * 2. Resume session (existing attached session) — measures blob rehydration delay
 */

import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

test('board to agent view transition via Enter', async ({ terminal }) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Press Enter on the selected work unit
  terminal.submit();

  // Should show the create session dialog OR navigate to attached session
  // Either way, trace recording captures the exact render timeline
  // Give it time to react — we're observing, not asserting speed yet
  await expect(
    terminal.getByText(/Start New Agent|Loading|Model/gi, { strict: false })
  ).toBeVisible();
});

test('board to agent view transition via slash key', async ({ terminal }) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Press / to navigate (same as Shift+Right)
  terminal.write('/');

  // Should trigger session navigation or create dialog
  await expect(
    terminal.getByText(/Start New Agent|Loading|Model/gi, { strict: false })
  ).toBeVisible();
});
