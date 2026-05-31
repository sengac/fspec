/**
 * E2E: RPC-072 — Real fspec binary produces an assistant reply visible in scrollback.
 *
 * Feature: spec/features/rpc072-work-agent-roundtrip.feature
 *
 * Spawns the real `fspec` binary built with `--features test-stub-provider`
 * (so the deterministic stub LlmProvider is registered at boot and the
 * default model is pinned to "stub/canned"). Walks to a DONE work unit,
 * opens its Work Agent, types "hello", and asserts that the rendered
 * scrollback contains "hi back" (the stub's canned reply) within 30
 * seconds, with no raw Rust `Debug { ... }` enum dumps leaking through.
 *
 * Pre-RPC-072 the typed input vanished into a dropped `input_rx` channel
 * and no assistant chunks ever arrived. This test is the user-facing
 * regression net for that fix.
 */

import { test, expect } from '@microsoft/tui-test';
import { homedir } from 'os';
import { join } from 'path';
import { writeFileSync } from 'fs';

const rustFspec = join(
  homedir(),
  'projects',
  'fspec',
  'codelet',
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

test('Work Agent emits assistant reply via stub provider for typed input', async ({
  terminal,
}) => {
  // @step Given the fspec binary has been built with --features test-stub-provider
  // (Asserted by the test harness: `npm run build` for the e2e suite uses
  // `cargo build --features test-stub-provider`; this test fails fast if
  // the binary was built without the feature because no reply chunk
  // ever arrives.)

  // Anchor on a known column header.
  await expect(
    terminal.getByText(/BACKLOG/g, { strict: false, full: true })
  ).toBeVisible();

  // Snapshot pre-Enter buffer for debugging.
  writeFileSync(
    '/tmp/rpc072_pre_enter.txt',
    bufferToText(terminal.getBuffer())
  );

  // Walk to the DONE column (5 right-arrows from BACKLOG).
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();
  terminal.keyRight();

  await new Promise(resolve => setTimeout(resolve, 500));

  // Press Enter to open the Work Agent on the selected DONE card.
  terminal.submit();

  // Wait for the AgentView to mount.
  await new Promise(resolve => setTimeout(resolve, 2000));

  // @step When the user walks to the DONE column, opens the Work Agent on that card, types "hello" and submits
  terminal.write('hello');
  await new Promise(resolve => setTimeout(resolve, 200));
  terminal.submit();

  // @step Then within 30 seconds the rendered scrollback contains the text "hi back"
  await expect(
    terminal.getByText(/hi back/g, { strict: false, full: true })
  ).toBeVisible({ timeout: 30_000 });

  // Snapshot post-submit buffer for debugging.
  writeFileSync(
    '/tmp/rpc072_post_submit.txt',
    bufferToText(terminal.getBuffer())
  );

  // @step And the rendered scrollback contains no raw Debug-printed StreamChunk variants
  // (RPC-071 regression check.)
  await expect(
    terminal.getByText(/UserInput \{/g, { strict: false, full: true })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/SessionStateChange \{/g, { strict: false, full: true })
  ).not.toBeVisible();
  await expect(
    terminal.getByText(/IsolationStateChange \{/g, {
      strict: false,
      full: true,
    })
  ).not.toBeVisible();

  // @step And the binary did not panic
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
