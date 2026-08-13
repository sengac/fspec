/**
 * E2E: Rust fspec binary smoke test (RPC-068 debugging)
 *
 * Writes the actual rendered PTY buffer to disk so we can examine it.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir, tmpdir } from 'os';
import { join } from 'path';
import { mkdtempSync, writeFileSync } from 'fs';

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
const tmpWorkspace = mkdtempSync(join(tmpdir(), 'rust-fspec-smoke-'));

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

test.describe('real workspace (reproduces RPC-068 screenshot)', () => {
  test.use({
    program: { file: rustFspec, args: ['--workspace', realWorkspace] },
    rows: 40,
    columns: 200,
  });

  test('captures buffer when run against real fspec workspace', async ({
    terminal,
  }) => {
    await new Promise(resolve => setTimeout(resolve, 6000));
    const text = bufferToText(terminal.getBuffer());
    writeFileSync('/tmp/rust_fspec_real_buffer.txt', text);
    await expect(
      terminal.getByText(/block_on/g, { strict: false, full: true })
    ).not.toBeVisible();
  });
});

test.describe('empty temp workspace (control)', () => {
  test.use({
    program: { file: rustFspec, args: ['--workspace', tmpWorkspace] },
    rows: 40,
    columns: 160,
  });

  test('captures buffer when run against empty workspace', async ({
    terminal,
  }) => {
    await new Promise(resolve => setTimeout(resolve, 4000));
    const text = bufferToText(terminal.getBuffer());
    writeFileSync('/tmp/rust_fspec_temp_buffer.txt', text);
    await expect(
      terminal.getByText(/block_on/g, { strict: false, full: true })
    ).not.toBeVisible();
  });
});
