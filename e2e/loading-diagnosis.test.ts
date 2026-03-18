/**
 * E2E: "Loading..." Duration Diagnosis
 *
 * Measures exactly how long "Loading..." is visible in the SessionHeader
 * when entering AgentView from the board. Writes timing data to
 * /tmp/fspec-loading-diag.log for inspection.
 *
 * Run with: npx @microsoft/tui-test --trace e2e/loading-diagnosis.test.ts
 * Then: cat /tmp/fspec-loading-diag.log
 */

import { test, expect } from '@microsoft/tui-test';
import fs from 'node:fs';

const LOG = '/tmp/fspec-loading-diag.log';
const log = (msg: string) => {
  const line = `[${new Date().toISOString()}] ${msg}\n`;
  fs.appendFileSync(LOG, line);
};

test.use({
  program: { file: './dist/index.js' },
  rows: 40,
  columns: 120,
});

test('measure Loading... duration on new session via Enter', async ({
  terminal,
}) => {
  fs.writeFileSync(LOG, '=== Loading... Diagnosis: Enter key ===\n');

  // Wait for board
  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
  log('Board rendered');

  // Press Enter on a work unit
  terminal.submit();
  log('Pressed Enter');

  // Check what appears — create dialog or agent view
  try {
    await expect(
      terminal.getByText(/Start New Agent/gi, { strict: false })
    ).toBeVisible();
    log('Create session dialog appeared');
    terminal.submit(); // Confirm
    log('Confirmed dialog');
  } catch {
    log('No create dialog — either auto-resuming or navigated directly');
  }

  // Now measure Loading...
  const t0 = Date.now();

  try {
    await expect(
      terminal.getByText('Loading...', { strict: false })
    ).toBeVisible();
    const appearedMs = Date.now() - t0;
    log(`"Loading..." APPEARED after ${appearedMs}ms`);

    // Wait for it to disappear (up to 30s)
    await expect(
      terminal.getByText('Loading...', { strict: false })
    ).not.toBeVisible();
    const disappearedMs = Date.now() - t0;
    log(`"Loading..." DISAPPEARED after ${disappearedMs}ms`);
    log(`VISIBLE DURATION: ${disappearedMs - appearedMs}ms`);
  } catch {
    const elapsed = Date.now() - t0;
    log(`"Loading..." never appeared OR never disappeared within ${elapsed}ms`);

    // Dump screen
    const buffer = terminal.getViewableBuffer();
    const screen = buffer
      .map((row: string[]) => row.join('').trimEnd())
      .join('\n');
    log(`Screen content:\n${screen}`);
  }
});

test('measure Loading... duration on slash-key navigation', async ({
  terminal,
}) => {
  fs.appendFileSync(LOG, '\n=== Loading... Diagnosis: Slash key ===\n');

  await expect(
    terminal.getByText(/backlog/gi, { strict: false })
  ).toBeVisible();
  log('Board rendered');

  terminal.write('/');
  log('Pressed /');

  const t0 = Date.now();

  // Check what appears
  try {
    await expect(
      terminal.getByText(/Start New Agent/gi, { strict: false })
    ).toBeVisible();
    log(`Create session dialog appeared after ${Date.now() - t0}ms`);
    terminal.submit();
    log('Confirmed dialog');
  } catch {
    log(`No create dialog after ${Date.now() - t0}ms`);
  }

  const t1 = Date.now();
  try {
    await expect(
      terminal.getByText('Loading...', { strict: false })
    ).toBeVisible();
    const appearedMs = Date.now() - t1;
    log(`"Loading..." APPEARED after ${appearedMs}ms`);

    await expect(
      terminal.getByText('Loading...', { strict: false })
    ).not.toBeVisible();
    const disappearedMs = Date.now() - t1;
    log(`"Loading..." DISAPPEARED after ${disappearedMs}ms`);
    log(`VISIBLE DURATION: ${disappearedMs - appearedMs}ms`);
  } catch {
    const elapsed = Date.now() - t1;
    log(`"Loading..." never appeared OR never disappeared within ${elapsed}ms`);

    const buffer = terminal.getViewableBuffer();
    const screen = buffer
      .map((row: string[]) => row.join('').trimEnd())
      .join('\n');
    log(`Screen content:\n${screen}`);
  }
});
