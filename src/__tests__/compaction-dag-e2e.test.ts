/**
 * Feature: Compaction DAG E2E — Full /compact → DAG → inject_summary Flow
 *
 * NO MOCKS — Uses real NAPI bindings + real agent loop.
 * Tests the complete compaction lifecycle:
 *   1. Create session, send a few messages to build history
 *   2. Call sessionCompact (simulating /compact)
 *   3. Verify the agent receives the compaction instruction
 *   4. Verify the agent calls SessionSearch + inject_summary
 *   5. Verify the DAG ends up pinned in the session
 *
 * Requires credentials in .env (ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN).
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { randomUUID } from 'crypto';
import { existsSync } from 'fs';
import { mkdir, writeFile, rm, copyFile } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import type { StreamChunk } from '@sengac/codelet-napi';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../tui/services/globalSessionStreamManager';

/** Collected chunk from global handler */
interface ReceivedChunk {
  sessionId: string;
  chunk: StreamChunk;
  timestamp: number;
}

/** Wait for a condition to become true, polling every `interval` ms */
async function waitFor(
  predicate: () => boolean,
  timeoutMs: number,
  intervalMs = 100
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (predicate()) {
      return true;
    }
    await new Promise(r => setTimeout(r, intervalMs));
  }
  return predicate();
}

const PROJECT_ROOT = join(__dirname, '..', '..');
const ENV_FILE = join(PROJECT_ROOT, '.env');

describe('Compaction DAG E2E — Full /compact Flow', () => {
  let testDir: string;
  let receivedChunks: ReceivedChunk[] = [];
  let manager: GlobalSessionStreamManager;
  let unregisterGlobalHandler: (() => void) | null = null;
  const createdSessionIds: string[] = [];

  const hasCredentials = existsSync(ENV_FILE);

  beforeAll(async () => {
    if (!hasCredentials) {
      return;
    }

    testDir = join(tmpdir(), `fspec-compact-e2e-${randomUUID().slice(0, 8)}`);
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });
    await mkdir(join(specDir, 'features'), { recursive: true });
    await writeFile(
      join(specDir, 'work-units.json'),
      JSON.stringify({
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {},
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
      })
    );

    await copyFile(ENV_FILE, join(testDir, '.env'));

    const persistenceDir = join(testDir, '.fspec', 'persistence');
    await mkdir(persistenceDir, { recursive: true });

    const { persistenceSetDataDirectory } = await import(
      '@sengac/codelet-napi'
    );
    persistenceSetDataDirectory(testDir);

    initGlobalSessionStreamManager();
    manager = GlobalSessionStreamManager.getInstance();

    await new Promise(r => setTimeout(r, 200));

    unregisterGlobalHandler = manager.registerGlobalHandler(
      (sessionId, chunk) => {
        receivedChunks.push({
          sessionId,
          chunk,
          timestamp: Date.now(),
        });
      }
    );
  });

  afterAll(async () => {
    if (!hasCredentials) {
      return;
    }

    if (unregisterGlobalHandler) {
      unregisterGlobalHandler();
    }

    const { sessionManagerDestroy } = await import('@sengac/codelet-napi');
    for (const id of createdSessionIds) {
      try {
        sessionManagerDestroy(id);
      } catch {
        // Ignore cleanup errors
      }
    }

    stopGlobalSessionStreamManager();

    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  beforeEach(() => {
    receivedChunks = [];
  });

  it('should trigger agent DAG building after sessionCompact', async () => {
    if (!hasCredentials) {
      console.log('SKIP: No .env credentials — cannot run compaction e2e');
      return;
    }

    const {
      sessionManagerCreateWithId,
      sessionSendInput,
      sessionCompact,
      sessionGetStatus,
      sessionInterrupt,
    } = await import('@sengac/codelet-napi');

    // ── Step 1: Create session ──
    const sessionId = randomUUID();
    createdSessionIds.push(sessionId);

    await sessionManagerCreateWithId(
      sessionId,
      'anthropic/claude-sonnet-4-20250514',
      testDir,
      'Compaction E2E Test'
    );

    manager.subscribeToSession(sessionId);
    await new Promise(r => setTimeout(r, 300));

    // ── Step 2: Send a message to build some history ──
    console.log('[E2E] Sending initial message to build history...');
    sessionSendInput(sessionId, 'Say "hello world" and nothing else.', null);

    // Wait for first response to complete
    const firstDone = await waitFor(
      () =>
        receivedChunks.some(
          rc => rc.sessionId === sessionId && rc.chunk.type === 'Done'
        ),
      15_000
    );

    if (!firstDone) {
      console.error('[E2E] TIMEOUT waiting for first response');
      try {
        sessionInterrupt(sessionId);
      } catch {
        // ignore
      }
      expect(firstDone).toBe(true);
      return;
    }

    const firstTextChunks = receivedChunks
      .filter(rc => rc.sessionId === sessionId && rc.chunk.type === 'Text')
      .map(rc => rc.chunk.text || '')
      .join('');
    console.log('[E2E] First response:', firstTextChunks.slice(0, 200));

    // ── Step 3: Call sessionCompact (simulates /compact) ──
    console.log('[E2E] Calling sessionCompact...');
    receivedChunks = []; // Reset chunks for compaction phase

    const compactResult = await sessionCompact(sessionId);
    console.log('[E2E] sessionCompact returned:', compactResult);

    // At this point, execute_compaction has:
    // - Set compaction_in_progress = true
    // - Cleared messages (kept system reminders)
    // - Injected COMPACTION_SYSTEM_INSTRUCTION as user message
    //
    // THE BUG: Nobody sends "Continue" to the agent loop, so the agent
    // never sees the compaction instruction and never builds a DAG.

    // ── Step 4: Wait to see if agent auto-processes the instruction ──
    console.log('[E2E] Waiting for agent to process compaction instruction...');

    // If the compaction flow works correctly, the agent should:
    // 1. Receive the compaction instruction
    // 2. Call SessionSearch tool(s)
    // 3. Build a DAG
    // 4. Call inject_summary tool
    // 5. Emit a Done chunk
    const COMPACTION_TIMEOUT = 180_000;
    const agentProcessed = await waitFor(
      () =>
        receivedChunks.some(
          rc =>
            rc.sessionId === sessionId &&
            (rc.chunk.type === 'Done' || rc.chunk.type === 'Error')
        ),
      COMPACTION_TIMEOUT
    );

    // ── Step 5: Verify compaction state lifecycle ──
    const postCompactChunks = receivedChunks.filter(
      rc => rc.sessionId === sessionId
    );
    const chunkTypes = postCompactChunks.map(rc => rc.chunk.type);

    // Extract SessionStateChange events to verify lifecycle
    const stateChanges = postCompactChunks
      .filter(rc => rc.chunk.type === 'SessionStateChange')
      .map(rc => ({
        state: (rc.chunk as Record<string, unknown>).state as string,
        timestamp: rc.timestamp,
      }));
    console.log('[E2E] State changes:', stateChanges);

    // First state change must be Compacting (from CompactionStarted)
    expect(stateChanges.length).toBeGreaterThanOrEqual(2);
    expect(stateChanges[0].state).toBe('Compacting');

    // Second state change must be Running (from CompactionContinuing)
    // This proves stream_loop emits CompactionContinuing, NOT CompactionComplete
    expect(stateChanges[1].state).toBe('Running');

    // Compacting phase must be brief (< 2s) — it's just in-memory setup
    const compactingDurationMs =
      stateChanges[1].timestamp - stateChanges[0].timestamp;
    console.log(`[E2E] Compacting phase duration: ${compactingDurationMs}ms`);
    expect(compactingDurationMs).toBeLessThan(2000);
    const textChunks = postCompactChunks.filter(rc => rc.chunk.type === 'Text');
    const toolCallChunks = postCompactChunks.filter(
      rc => rc.chunk.type === 'ToolCall'
    );
    const toolResultChunks = postCompactChunks.filter(
      rc => rc.chunk.type === 'ToolResult'
    );
    const errorChunks = postCompactChunks.filter(
      rc => rc.chunk.type === 'Error'
    );
    const doneChunks = postCompactChunks.filter(rc => rc.chunk.type === 'Done');

    console.log('[E2E] Post-compact chunk types:', chunkTypes);
    console.log('[E2E] Tool calls:', toolCallChunks.length);
    console.log('[E2E] Tool results:', toolResultChunks.length);
    console.log('[E2E] Text chunks:', textChunks.length);
    console.log('[E2E] Error chunks:', errorChunks.length);
    console.log('[E2E] Done chunks:', doneChunks.length);

    if (toolCallChunks.length > 0) {
      for (const tc of toolCallChunks) {
        console.log('[E2E] Tool call:', JSON.stringify(tc.chunk).slice(0, 200));
      }
    }

    if (errorChunks.length > 0) {
      for (const ec of errorChunks) {
        console.log('[E2E] Error:', JSON.stringify(ec.chunk));
      }
    }

    const fullText = textChunks.map(tc => tc.chunk.text || '').join('');
    if (fullText.length > 0) {
      console.log('[E2E] Agent text output:', fullText.slice(0, 500));
    }

    if (!agentProcessed) {
      console.log(
        '[E2E] *** BUG CONFIRMED: Agent never processed compaction instruction! ***'
      );
      console.log(
        '[E2E] sessionCompact cleared context and injected instruction,'
      );
      console.log(
        '[E2E] but nobody sent input to agent_loop to trigger processing.'
      );

      // Dump all chunks for debugging
      if (postCompactChunks.length > 0) {
        console.log('[E2E] All post-compact chunks:');
        for (const rc of postCompactChunks) {
          const preview =
            rc.chunk.type === 'Text'
              ? ` "${(rc.chunk.text || '').slice(0, 80)}"`
              : '';
          console.log(
            `  ${rc.chunk.type}${preview} @ +${rc.timestamp - postCompactChunks[0].timestamp}ms`
          );
        }
      } else {
        console.log('[E2E] Zero chunks received after sessionCompact!');
      }
    }

    const status = sessionGetStatus(sessionId);
    console.log('[E2E] Final session status:', status);

    // ── ASSERTIONS ──
    // The agent MUST have processed the compaction instruction
    expect(agentProcessed).toBe(true);

    // The agent MUST have called inject_summary (look for tool calls)
    const toolNames = toolCallChunks.map(tc => {
      try {
        const chunk = tc.chunk as Record<string, unknown>;
        const toolCall = chunk.toolCall as Record<string, unknown> | undefined;
        return (toolCall?.name as string) || '';
      } catch {
        return '';
      }
    });
    console.log('[E2E] Tool names called:', toolNames);

    // inject_summary MUST have been called
    expect(toolNames).toContain('inject_summary');

    // CompactionComplete must arrive AFTER inject_summary (DAG applied)
    const compactionCompleteChunks = postCompactChunks.filter(
      rc => rc.chunk.type === 'CompactionComplete'
    );
    console.log(
      '[E2E] CompactionComplete chunks:',
      compactionCompleteChunks.length
    );
    expect(compactionCompleteChunks.length).toBe(1);

    // CompactionComplete must come after the last ToolResult (inject_summary result)
    const lastToolResultIdx = chunkTypes.lastIndexOf('ToolResult');
    const compactionCompleteIdx = chunkTypes.indexOf('CompactionComplete');
    console.log(
      `[E2E] Last ToolResult at index ${lastToolResultIdx}, CompactionComplete at ${compactionCompleteIdx}`
    );
    expect(compactionCompleteIdx).toBeGreaterThan(lastToolResultIdx);

    // Done chunk must exist (stream completed)
    expect(doneChunks.length).toBe(1);

    // Session should be idle after
    expect(status).toBe('idle');
  }, 240_000); // 240s test timeout
});
