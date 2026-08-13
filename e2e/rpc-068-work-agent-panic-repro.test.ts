/**
 * E2E: Rust fspec Work Agent panic repro (RPC-068)
 *
 * Reproduces the panic the user captured in the screenshot:
 *   1. Launch Rust fspec against ~/projects/fspec
 *   2. Navigate to DONE column (← / →)
 *   3. Select RPC-068
 *   4. Press Enter ("Work Agent")
 *   5. Capture the resulting buffer — we expect the tokio
 *      "Cannot start a runtime from within a runtime" panic.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir } from 'os';
import { join } from 'path';
import { writeFileSync } from 'fs';

const rustFspec = join(
  homedir(),
  'projects',
  'fspec',
  'rust',
  'target',
  'debug',
  'fspec'
);
const realWorkspace = join(homedir(), 'projects', 'fspec');

test.use({
  program: { file: rustFspec, args: ['--workspace', realWorkspace] },
  rows: 50,
  columns: 220,
  env: { RUST_BACKTRACE: '1' },
});

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

test('pressing Enter on a work unit (Work Agent) does not panic', async ({
  terminal,
}) => {
  // Wait for the board to render — anchor on a known column header.
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();

  // Snapshot the pre-Enter buffer for reference.
  writeFileSync(
    '/tmp/rust_fspec_pre_enter.txt',
    bufferToText(terminal.getBuffer())
  );

  // Move to the DONE column — board has 7 columns:
  // BACKLOG, SPECIFYING, TESTING, IMPLEMENTING, VALIDATING, DONE, BLOCKED.
  // From the initial BACKLOG selection we need 5 keyRight presses to reach
  // DONE (where RPC-068 lives — see screenshot).
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();

  // Give the focus change a beat to settle.
  await new Promise(resolve => setTimeout(resolve, 500));

  // Press Enter ("Work Agent") on whatever is selected in DONE.
  terminal.submit();

  // Wait for the agent panel or panic output.
  await new Promise(resolve => setTimeout(resolve, 5000));

  const text = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/rust_fspec_post_enter.txt', text);

  // Negative assertion — if this fails, we have positive evidence of
  // the tokio runtime panic the screenshot showed.
  await expect(
    terminal.getByText(/block_on/g, { strict: false, full: true })
  ).not.toBeVisible();

  await expect(
    terminal.getByText(/Cannot start a runtime/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible();
});
