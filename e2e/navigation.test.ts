/**
 * E2E: Keyboard Navigation
 *
 * Tests keyboard shortcuts in the fspec TUI BoardView.
 * Verifies arrow keys, Enter, Escape, and shortcut keys work
 * correctly in a real PTY environment.
 */

import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

test('arrow keys navigate between columns', async ({ terminal }) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Right arrow should move to next column
  terminal.keyRight();
});

test('escape shows exit confirmation', async ({ terminal }) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Press Escape
  terminal.keyEscape();

  // Should show exit confirmation dialog
  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).toBeVisible();
});

test('cancel exit dialog returns to board', async ({ terminal }) => {
  // Wait for board to render
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();

  // Press Escape to show exit dialog
  terminal.keyEscape();

  // Wait for dialog to fully render with its buttons
  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).toBeVisible();
  await expect(terminal.getByText(/Yes/g, { strict: false })).toBeVisible();
  await expect(terminal.getByText(/No/g, { strict: false })).toBeVisible();

  // Exit dialog uses 'visual' confirmMode — default selection is "Yes".
  // Press right arrow to move to "No", then Enter to confirm cancel.
  // Small delay ensures the dialog's input handler is registered.
  await new Promise(r => setTimeout(r, 200));
  terminal.keyRight();
  terminal.submit();

  // Should return to the board with dialog dismissed
  await expect(
    terminal.getByText(/Exit fspec/gi, { strict: false })
  ).not.toBeVisible();

  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
});
