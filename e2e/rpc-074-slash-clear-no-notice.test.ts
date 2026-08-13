/**
 * E2E: RPC-074 — Real fspec binary `/clear` slash command produces NO
 * synthetic `[notice] /clear: history cleared` line in rendered
 * scrollback. TS parity test against the TypeScript reference at
 * src/tui/components/AgentView.tsx:1554-1564 (handleClearCommand).
 *
 * Feature: spec/features/rpc074-clear-ts-parity.feature
 *
 * Spawns the real `fspec` binary built with `--features
 * test-stub-provider`, walks to a DONE work unit, opens its Work Agent,
 * sends "hello" so the stub replies with "hi back" (proving scrollback
 * has output), THEN sends `/clear` and asserts the post-/clear rendered
 * scrollback contains:
 *   - NO `[notice] /clear ...` line
 *   - NO `[error] /clear failed: ...` line
 *   - NO `history cleared` substring anywhere
 *
 * Pre-RPC-074 the binary pushed a synthetic
 * `[notice] /clear: history cleared` line into the focused session's
 * scrollback. This test is the user-facing regression net for that
 * fix.
 *
 * Pattern mirrors `e2e/rpc-072-work-agent-roundtrip.test.ts`.
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
  env: { RUST_BACKTRACE: '1', RUST_LOG: 'codelet_agent_loop=debug' },
});

function bufferToText(buffer: ReadonlyArray<ReadonlyArray<string>>): string {
  return buffer.map(row => row.join('').trimEnd()).join('\n');
}

test('Work Agent /clear emits no TS-divergent notice line in scrollback', async ({
  terminal,
}) => {
  // @step Given the real fspec binary built with --features test-stub-provider is launched under tui-test against the project workspace
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();

  // Walk to the DONE column (5 right-arrows from BACKLOG).
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();

  await new Promise(resolve => setTimeout(resolve, 500));

  // @step And the user has opened a Work Agent on a DONE work unit and sent at least one message so the scrollback contains output
  terminal.submit();
  await new Promise(resolve => setTimeout(resolve, 2000));

  // Send "hello" so the stub responds with "hi back" — this proves the
  // session has scrollback output BEFORE we /clear it.
  terminal.write('hello');
  await new Promise(resolve => setTimeout(resolve, 200));
  terminal.submit();

  await expect(
    terminal.getByText(/hi back/g, { strict: false, full: true })
  ).toBeVisible({ timeout: 30_000 });

  writeFileSync(
    '/tmp/rpc074_pre_clear.txt',
    bufferToText(terminal.getBuffer())
  );

  // @step When the user types "/clear" and presses Enter
  terminal.write('/clear');
  await new Promise(resolve => setTimeout(resolve, 300));
  terminal.submit();

  // Allow the dispatcher's scrollback reset + backend round-trip to
  // settle.
  await new Promise(resolve => setTimeout(resolve, 1500));

  writeFileSync(
    '/tmp/rpc074_post_clear.txt',
    bufferToText(terminal.getBuffer())
  );

  // @step Then within 5 seconds the rendered scrollback does NOT contain the substring "[notice] /clear"
  await expect(
    terminal.getByText(/\[notice\] \/clear/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible({ timeout: 5_000 });

  // @step And within 5 seconds the rendered scrollback does NOT contain the substring "history cleared"
  await expect(
    terminal.getByText(/history cleared/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible({ timeout: 5_000 });

  // @step And within 5 seconds the rendered scrollback does NOT contain the substring "[error] /clear failed"
  await expect(
    terminal.getByText(/\[error\] \/clear failed/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible({ timeout: 5_000 });
});
