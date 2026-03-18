/**
 * E2E: Board Rendering
 *
 * Verifies the fspec TUI board launches and renders correctly
 * in a real PTY environment using @microsoft/tui-test.
 */

import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

test('fspec board renders column headers', async ({ terminal }) => {
  // The board should render Kanban column headers
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});

test('fspec board renders without crash', async ({ terminal }) => {
  // Verify multiple columns render (not just one)
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

test('fspec board shows ESC hint', async ({ terminal }) => {
  // Board should indicate how to exit
  await expect(terminal.getByText(/ESC/g, { strict: false })).toBeVisible();
});
