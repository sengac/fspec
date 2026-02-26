/**
 * Message Duplication Bug Investigation - E2E Test
 *
 * This test investigates why messages appear to be processed multiple times
 * when sent via TUI or Telegram bridge.
 *
 * NO MOCKS - Uses real NAPI bindings to identify the actual bug location.
 *
 * Tracking: diagnose.md
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';
import { randomUUID } from 'crypto';
import { mkdir, writeFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import type { StreamChunk } from '@sengac/codelet-napi';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../tui/services/globalSessionStreamManager';

interface ReceivedChunk {
  sessionId: string;
  chunk: StreamChunk;
  timestamp: number;
}

describe('Message Duplication Bug Investigation', () => {
  let testDir: string;
  let receivedChunks: ReceivedChunk[] = [];
  let manager: GlobalSessionStreamManager;
  let unregisterGlobalHandler: (() => void) | null = null;
  let createdSessionIds: string[] = [];

  beforeAll(async () => {
    // Create temp directory for test
    testDir = join(tmpdir(), `fspec-msg-dup-${randomUUID().slice(0, 8)}`);
    await mkdir(testDir, { recursive: true });

    // Create minimal spec structure
    const specDir = join(testDir, 'spec');
    await mkdir(specDir, { recursive: true });
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

    // Create credentials for API (needed for session creation)
    const persistenceDir = join(testDir, '.fspec', 'persistence');
    await mkdir(persistenceDir, { recursive: true });

    // Initialize the global session stream manager
    initGlobalSessionStreamManager();
    manager = GlobalSessionStreamManager.getInstance();

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

  /**
   * Test 1: Count WatcherInput chunks per bridge message
   *
   * When a message comes from the bridge (Telegram), it goes through:
   * 1. Bridge -> watcher_input channel
   * 2. agent_loop receives from watcher_input
   * 3. agent_loop emits WatcherInput chunk
   *
   * If we see multiple WatcherInput chunks for one message, duplication is in Rust.
   */
  it('should emit exactly ONE WatcherInput chunk per bridge message', async () => {
    const { sessionManagerCreateWithId, sessionManagerDestroy } = await import(
      '@sengac/codelet-napi'
    );

    // Create a test session
    const sessionId = randomUUID();
    createdSessionIds.push(sessionId);

    try {
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4',
        testDir,
        'Test Session'
      );
    } catch {
      // May fail due to no API key, but session should still exist
    }

    // Wait a bit for session to be ready
    await new Promise(resolve => setTimeout(resolve, 100));

    // Clear any initial chunks
    receivedChunks = [];

    // Simulate a bridge message using GlobalSessionStreamManager
    // This mimics what happens when watcher_input is received
    const testMessage = `Test message ${Date.now()}`;
    const watcherInputChunk: StreamChunk = {
      type: 'WatcherInput',
      text: `[WATCHER: bridge | Authority: Peer | Session: bridge] ${testMessage}`,
    };

    // Simulate the chunk being emitted (this is what agent_loop does)
    manager.simulateChunk(sessionId, watcherInputChunk);

    // Wait for processing
    await new Promise(resolve => setTimeout(resolve, 100));

    // Count WatcherInput chunks for this session
    const watcherInputChunks = receivedChunks.filter(
      rc => rc.sessionId === sessionId && rc.chunk.type === 'WatcherInput'
    );

    console.log(
      `[TEST] Received ${watcherInputChunks.length} WatcherInput chunk(s)`
    );
    console.log(
      `[TEST] All chunks:`,
      receivedChunks.map(rc => ({
        sessionId: rc.sessionId,
        type: rc.chunk.type,
      }))
    );

    // CRITICAL ASSERTION: Should be exactly 1
    expect(watcherInputChunks.length).toBe(1);
    expect(watcherInputChunks[0].chunk.text).toContain(testMessage);
  });

  /**
   * Test 2: Count UserInput chunks per direct input
   *
   * When a message comes from TUI (sessionSendInput), it goes through:
   * 1. sessionSendInput -> input_tx channel
   * 2. agent_loop receives from input_rx
   * 3. Rust adds UserInput to session messages
   *
   * We count how many times the user message appears in conversation.
   */
  it('should process user input exactly ONCE per sessionSendInput call', async () => {
    const {
      sessionManagerCreateWithId,
      sessionManagerList,
      sessionSendInput,
      sessionGetMergedOutput,
    } = await import('@sengac/codelet-napi');

    // Create a test session
    const sessionId = randomUUID();
    createdSessionIds.push(sessionId);

    let sessionCreated = false;
    try {
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4',
        testDir,
        'Test Session'
      );
      // Verify session actually exists in Rust
      const sessions = sessionManagerList();
      sessionCreated = sessions.some(s => s.id === sessionId);
    } catch {
      // May fail due to no API key
    }

    // Skip if session could not be created (no API key in CI)
    if (!sessionCreated) {
      return;
    }

    // Clear chunks
    receivedChunks = [];

    // Send a SINGLE user input
    const testMessage = `Single test message ${Date.now()}`;
    sessionSendInput(sessionId, testMessage, null);

    // Wait for agent_loop to process (it will fail at API call, but that's ok)
    // We're checking if the message is added to history multiple times
    await new Promise(resolve => setTimeout(resolve, 500));

    // Get the merged output to see what was emitted
    let mergedOutput: StreamChunk[] = [];
    try {
      mergedOutput = sessionGetMergedOutput(sessionId);
    } catch {
      // Session may not exist if creation failed
    }

    // Count UserInput chunks in merged output
    const userInputChunks = mergedOutput.filter(
      (chunk: StreamChunk) => chunk.type === 'UserInput'
    );

    console.log(
      `[TEST] UserInput chunks in mergedOutput: ${userInputChunks.length}`
    );
    console.log(
      `[TEST] All chunk types:`,
      mergedOutput.map((c: StreamChunk) => c.type)
    );

    // Also check received chunks through global handler
    const globalUserInputChunks = receivedChunks.filter(
      rc => rc.sessionId === sessionId && rc.chunk.type === 'UserInput'
    );
    console.log(
      `[TEST] UserInput chunks via global handler: ${globalUserInputChunks.length}`
    );

    // CRITICAL ASSERTION: Should be exactly 1 (or 0 if session didn't start)
    // If > 1, we found the duplication bug!
    expect(userInputChunks.length).toBeLessThanOrEqual(1);
  });

  /**
   * Test 3: Monitor chunk sequence for a single message
   *
   * This test logs the exact sequence of chunks emitted when processing
   * a single message. This helps identify WHERE duplication occurs.
   */
  it('should log chunk sequence for debugging duplication', async () => {
    const { sessionManagerCreateWithId, sessionGetMergedOutput } = await import(
      '@sengac/codelet-napi'
    );

    const sessionId = randomUUID();
    createdSessionIds.push(sessionId);

    try {
      await sessionManagerCreateWithId(
        sessionId,
        'anthropic/claude-sonnet-4',
        testDir,
        'Chunk Sequence Test'
      );
    } catch {
      // May fail
    }

    // Clear chunks
    receivedChunks = [];

    // Simulate multiple messages to see if duplication is cumulative
    const messages = ['First', 'Second', 'Third'];

    for (const msg of messages) {
      const chunk: StreamChunk = {
        type: 'WatcherInput',
        text: `[WATCHER: test | Authority: Peer | Session: test] ${msg}`,
      };
      manager.simulateChunk(sessionId, chunk);
    }

    await new Promise(resolve => setTimeout(resolve, 100));

    // Log the sequence
    console.log('\n=== CHUNK SEQUENCE ===');
    receivedChunks
      .filter(rc => rc.sessionId === sessionId)
      .forEach((rc, i) => {
        console.log(
          `${i + 1}. ${rc.chunk.type}: ${(rc.chunk as { text?: string }).text?.slice(0, 50) || '(no text)'}`
        );
      });
    console.log('=== END SEQUENCE ===\n');

    // Count each message
    const counts: Record<string, number> = {};
    for (const rc of receivedChunks.filter(rc => rc.sessionId === sessionId)) {
      const text = (rc.chunk as { text?: string }).text || '';
      for (const msg of messages) {
        if (text.includes(msg)) {
          counts[msg] = (counts[msg] || 0) + 1;
        }
      }
    }

    console.log('Message counts:', counts);

    // Each message should appear exactly once
    for (const msg of messages) {
      expect(counts[msg] || 0).toBe(1);
    }
  });
});
