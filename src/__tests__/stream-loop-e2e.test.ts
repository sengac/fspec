/**
 * Feature: Stream Loop E2E — Basic Agent Interaction
 *
 * NO MOCKS — Uses real NAPI bindings + real GlobalSessionStreamManager.
 * Sends actual input to the agent loop and verifies streaming chunks
 * come back through the same path as the TUI.
 *
 * Requires credentials in .env (ANTHROPIC_API_KEY or CLAUDE_CODE_OAUTH_TOKEN).
 * The NAPI credential resolver reads the project .env automatically.
 *
 * This is the definitive test for "can I type a message and get a response?"
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

// The .env file in the project root has credentials
const PROJECT_ROOT = join(__dirname, '..', '..');
const ENV_FILE = join(PROJECT_ROOT, '.env');

describe('Stream Loop E2E — Basic Agent Interaction', () => {
  let testDir: string;
  let receivedChunks: ReceivedChunk[] = [];
  let manager: GlobalSessionStreamManager;
  let unregisterGlobalHandler: (() => void) | null = null;
  const createdSessionIds: string[] = [];

  // Skip if no .env with credentials
  const hasCredentials = existsSync(ENV_FILE);

  beforeAll(async () => {
    if (!hasCredentials) {
      return;
    }

    // Create temp project directory with minimal spec structure
    testDir = join(tmpdir(), `fspec-stream-e2e-${randomUUID().slice(0, 8)}`);
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

    // Copy .env from project root so NAPI credential resolver finds it
    await copyFile(ENV_FILE, join(testDir, '.env'));

    // Persistence directory for session storage
    const persistenceDir = join(testDir, '.fspec', 'persistence');
    await mkdir(persistenceDir, { recursive: true });

    // Set data directory for NAPI persistence layer
    const { persistenceSetDataDirectory } = await import(
      '@sengac/codelet-napi'
    );
    persistenceSetDataDirectory(testDir);

    // Initialize global session stream manager (registers NAPI callback)
    initGlobalSessionStreamManager();
    manager = GlobalSessionStreamManager.getInstance();

    // Wait for async registerGlobalCallback to finish
    await new Promise(r => setTimeout(r, 200));

    // Register global handler to capture ALL chunks from ALL sessions
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

    // Clean up global handler
    if (unregisterGlobalHandler) {
      unregisterGlobalHandler();
    }

    // Destroy all created sessions
    const { sessionManagerDestroy } = await import('@sengac/codelet-napi');
    for (const id of createdSessionIds) {
      try {
        sessionManagerDestroy(id);
      } catch {
        // Ignore cleanup errors
      }
    }

    stopGlobalSessionStreamManager();

    // Clean up temp directory
    try {
      await rm(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  beforeEach(() => {
    receivedChunks = [];
  });

  it('should stream text chunks back when sending a simple prompt', async () => {
    if (!hasCredentials) {
      console.log('SKIP: No .env credentials — cannot run stream loop e2e');
      return;
    }

    const {
      sessionManagerCreateWithId,
      sessionSendInput,
      sessionGetStatus,
      sessionInterrupt,
    } = await import('@sengac/codelet-napi');

    // Create a real session with a real provider
    const sessionId = randomUUID();
    createdSessionIds.push(sessionId);

    await sessionManagerCreateWithId(
      sessionId,
      'anthropic/claude-sonnet-4-20250514',
      testDir,
      'Stream E2E Test'
    );

    // Subscribe to receive chunks for this session
    manager.subscribeToSession(sessionId);

    // Wait for session to be ready
    await new Promise(r => setTimeout(r, 300));

    // Send a trivial prompt — no tools needed
    sessionSendInput(
      sessionId,
      'What is 1+1? Reply with just the number.',
      null
    );

    // Wait for Done or Error chunk — 10s timeout then kill
    const STREAM_TIMEOUT_MS = 10_000;
    const gotResponse = await waitFor(
      () =>
        receivedChunks.some(
          rc =>
            rc.sessionId === sessionId &&
            (rc.chunk.type === 'Done' || rc.chunk.type === 'Error')
        ),
      STREAM_TIMEOUT_MS
    );

    // If timed out, interrupt and dump diagnostics
    if (!gotResponse) {
      console.error(
        `[E2E] TIMEOUT after ${STREAM_TIMEOUT_MS}ms — stream loop hung!`
      );
      try {
        sessionInterrupt(sessionId);
      } catch {
        // ignore
      }
      // Brief wait for interrupt to propagate
      await new Promise(r => setTimeout(r, 500));
    }

    // Collect results
    const sessionChunks = receivedChunks.filter(
      rc => rc.sessionId === sessionId
    );
    const chunkTypes = sessionChunks.map(rc => rc.chunk.type);
    const textChunks = sessionChunks.filter(rc => rc.chunk.type === 'Text');
    const errorChunks = sessionChunks.filter(rc => rc.chunk.type === 'Error');
    const doneChunks = sessionChunks.filter(rc => rc.chunk.type === 'Done');

    // Debug output — always print so we can diagnose
    console.log('[E2E] Chunk types received:', chunkTypes);
    console.log('[E2E] Text chunks:', textChunks.length);
    console.log('[E2E] Error chunks:', errorChunks.length);
    if (errorChunks.length > 0) {
      for (const ec of errorChunks) {
        console.log('[E2E] Error:', JSON.stringify(ec.chunk));
      }
    }
    const fullText = textChunks.map(tc => tc.chunk.text || '').join('');
    console.log('[E2E] Full text response:', fullText.slice(0, 500));

    // Dump ALL chunks for debugging if stream hung
    if (!gotResponse) {
      console.log('[E2E] ALL chunks received before timeout:');
      for (const rc of sessionChunks) {
        const preview =
          rc.chunk.type === 'Text'
            ? ` "${(rc.chunk.text || '').slice(0, 80)}"`
            : '';
        console.log(
          `  ${rc.chunk.type}${preview} @ +${rc.timestamp - sessionChunks[0].timestamp}ms`
        );
      }
    }

    // Final status
    const status = sessionGetStatus(sessionId);
    console.log('[E2E] Final session status:', status);

    // ASSERTIONS
    // 1. Must have received a Done chunk (stream completed, didn't hang)
    expect(gotResponse).toBe(true);
    expect(doneChunks.length).toBe(1);

    // 2. Must have received at least one Text chunk
    expect(textChunks.length).toBeGreaterThan(0);

    // 3. The response should contain "2"
    expect(fullText).toContain('2');

    // 4. No Error chunks
    expect(errorChunks.length).toBe(0);

    // 5. Session should be idle after completion
    expect(status).toBe('idle');
  }, 20_000); // 20s test timeout — stream has 10s before we kill it
});
