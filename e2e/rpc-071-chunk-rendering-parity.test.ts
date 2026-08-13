/**
 * E2E: Rust fspec AgentView chunk_to_lines parity (RPC-071)
 *
 * Reproduces the bug captured in the 2026-05-27 screenshot:
 *   1. Launch Rust fspec against ~/projects/fspec
 *   2. Navigate to the DONE column (← / →)
 *   3. Select a work unit
 *   4. Press Enter ("Work Agent")
 *   5. Type "please review this card", press Enter
 *
 * Pre-fix, the rendered scrollback contained the Debug-printed Rust
 * enum variants:
 *
 *     user> please review this card
 *     UserInput { text: "please review this card" }
 *     SessionStateChange { state: Running }
 *     SessionStateChange { state: Idle }
 *
 * Post-fix, ONLY the `user> ...` line should remain. The three
 * Debug-dump lines must NOT appear.
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

test('AgentView never leaks raw StreamChunk Debug output into scrollback', async ({
  terminal,
}) => {
  // Wait for the board to render — anchor on a known column header.
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();

  // Snapshot pre-Enter buffer.
  writeFileSync(
    '/tmp/rpc071_pre_enter.txt',
    bufferToText(terminal.getBuffer())
  );

  // Walk to the DONE column. Board has 7 columns:
  // BACKLOG, SPECIFYING, TESTING, IMPLEMENTING, VALIDATING, DONE, BLOCKED.
  // From the initial BACKLOG selection we need 5 right-arrow presses to
  // reach DONE.
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();

  // Give the focus change a moment to settle.
  await new Promise(resolve => setTimeout(resolve, 500));

  // Press Enter ("Work Agent") on whatever is selected in DONE.
  terminal.submit();

  // Wait for the AgentView to mount.
  await new Promise(resolve => setTimeout(resolve, 2000));

  // Type the reproducer message and submit it.
  terminal.write('please review this card');
  await new Promise(resolve => setTimeout(resolve, 200));
  terminal.submit();

  // Wait long enough for SessionStateChange(Running) + SessionStateChange(Idle)
  // chunks to flow back through the embedded transport into the scrollback.
  await new Promise(resolve => setTimeout(resolve, 2500));

  const text = bufferToText(terminal.getBuffer());
  writeFileSync('/tmp/rpc071_post_submit.txt', text);

  // Positive assertion: the user-input line MUST be visible (parity
  // with the TS Ink `user> {text}` rendering).
  await expect(
    terminal.getByText(/user> please review this card/g, {
      strict: false,
      full: true,
    })
  ).toBeVisible();

  // Negative assertions: NONE of the raw Rust Debug-printed variants
  // may appear in the rendered terminal. Each of these substrings is
  // a smoking gun for the pre-RPC-071 regression.
  await expect(
    terminal.getByText(/UserInput \{/g, { strict: false, full: true })
  ).not.toBeVisible();

  await expect(
    terminal.getByText(/SessionStateChange \{/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible();

  await expect(
    terminal.getByText(/IsolationStateChange \{/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible();

  await expect(
    terminal.getByText(/FspecCommandRequest \{/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible();

  // Also ensure the binary did not panic.
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
