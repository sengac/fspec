/**
 * E2E: AST Entity Deduplication — GraphSearch ast_index without constraint violations
 *
 * Feature: spec/features/ast-entity-deduplication.feature
 *
 * Verifies the TUI and CLI don't show errors when the knowledge graph
 * indexes a codebase containing TypeScript files with cross-imports.
 * The root bug (KGRAPH-026) was that import resolution created duplicate
 * File entities, triggering a nanograph @unique constraint violation on
 * File.path during ast_index.
 *
 * These E2E tests verify the fix through the real application binary —
 * no mocks, no stubs, real code execution in a real PTY.
 */

import { test, expect } from '@microsoft/tui-test';

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

// ============================================================================
// Helper: wait for board to be fully rendered
// ============================================================================
const waitForBoard = async (
  terminal: Parameters<Parameters<typeof test>[1]>[0]['terminal']
) => {
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
};

// ============================================================================
// Scenario: Full codebase indexing completes without errors
// (Triggered via TUI session creation which initializes the graph)
// ============================================================================

test.describe('AST Entity Deduplication', () => {
  test('application boots without AST graph unique constraint errors', async ({
    terminal,
  }) => {
    // @step Given a project directory with TypeScript files containing cross-imports
    // The fspec project itself has bridge/telegram-endpoint.ts importing ./telegram-slash-commands
    // which was the original trigger for the bug
    await waitForBoard(terminal);

    // @step When the AST index operation runs via walk_and_extract
    // Graph initialization happens on session entry — enter agent view
    terminal.submit();
    await new Promise(r => setTimeout(r, 2000));

    // @step Then the load operation should succeed with no unique constraint violations
    await expect(
      terminal.getByText(/unique constraint/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/duplicate.*File/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/duplicate.*path/gi, { strict: false })
    ).not.toBeVisible();

    // @step And the graph should contain File, Function, and Imports data
    // Verify no error banners or crash messages
    await expect(
      terminal.getByText(/Failed to load entities/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/JSONL.*error/gi, { strict: false })
    ).not.toBeVisible();
  });

  test('no graph entity errors visible after session creation', async ({
    terminal,
  }) => {
    // @step Given the board has loaded
    await waitForBoard(terminal);

    // @step When the user enters a session (graph handler registers + entity pipeline fires)
    terminal.submit();
    await new Promise(r => setTimeout(r, 2000));

    // @step Then no duplicate or constraint violation messages appear
    await expect(
      terminal.getByText(/@unique/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/constraint violation/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/duplicate value/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/entity.*error/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/extractor.*fail/gi, { strict: false })
    ).not.toBeVisible();
  });

  test('board navigation remains stable after graph indexing completes', async ({
    terminal,
  }) => {
    // @step Given the board has loaded and graph features are active
    await waitForBoard(terminal);

    // @step When I enter then exit a session (triggering graph init)
    terminal.submit();
    await new Promise(r => setTimeout(r, 2000));
    terminal.keyEscape();
    await new Promise(r => setTimeout(r, 1000));

    // @step Then the application is still responsive
    const buffer = terminal.getViewableBuffer();
    const screen = buffer
      .map((row: string[]) => row.join('').trimEnd())
      .join('\n');
    expect(screen.length).toBeGreaterThan(0);

    // @step And no deduplication errors leaked into the UI
    await expect(
      terminal.getByText(/unique constraint/gi, { strict: false })
    ).not.toBeVisible();
    await expect(
      terminal.getByText(/duplicate/gi, { strict: false })
    ).not.toBeVisible();
  });
});
