/**
 * Feature: spec/features/tui-session-chunk-filtering.feature
 *
 * Tests for TUI session chunk filtering.
 * TUI (AgentView) registers a handler with GlobalSessionStreamManager for its current session.
 * It only displays chunks that match the currently-viewed session_id.
 * When viewing Session A, chunks from Session B are ignored.
 * Bridge input works because bridge chunks have the correct session_id.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import type { StreamChunk } from '@sengac/codelet-napi';
import {
  GlobalSessionStreamManager,
  injectTestChunk,
} from '../services/globalSessionStreamManager';

describe('Feature: TUI session chunk filtering', () => {
  beforeEach(() => {
    // Reset singleton state for each test
    GlobalSessionStreamManager.resetInstance();
  });

  afterEach(() => {
    GlobalSessionStreamManager.resetInstance();
  });

  describe('Background: User Story', () => {
    it('validates the user story for session-specific chunk display', () => {
      // @step As a developer
      // @step I want the TUI to only display chunks for the session I'm viewing
      // @step So that multiple sessions don't mix their output

      // This validates that the manager supports per-session handlers
      const manager = GlobalSessionStreamManager.getInstance();
      expect(manager.registerHandler).toBeDefined();
      expect(manager.subscribeToSession).toBeDefined();
    });
  });

  describe('Scenario: TUI displays only current session chunks', () => {
    it('should display chunks from viewed session and ignore others', async () => {
      // @step Given the TUI is viewing session "session-a"
      const currentViewedSession = 'session-a';
      const displayedChunks: StreamChunk[] = [];

      const manager = GlobalSessionStreamManager.getInstance();

      // Subscribe to both sessions
      manager.subscribeToSession('session-a');
      manager.subscribeToSession('session-b');

      // TUI registers handler ONLY for the session it's viewing
      manager.registerHandler(
        currentViewedSession,
        (_sessionId: string, chunk: StreamChunk) => {
          displayedChunks.push(chunk);
        }
      );

      // @step And session "session-a" is running
      // @step And session "session-b" is running in background

      // @step When session "session-a" emits a TextDelta chunk
      const chunkA: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Output from session A',
      };
      injectTestChunk('session-a', chunkA);

      // @step And session "session-b" emits a TextDelta chunk
      const chunkB: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Output from session B',
      };
      injectTestChunk('session-b', chunkB);

      // @step Then the TUI should display the chunk from session "session-a"
      expect(displayedChunks).toHaveLength(1);
      expect(displayedChunks[0]).toEqual(chunkA);

      // @step And the TUI should not display the chunk from session "session-b"
      expect(
        displayedChunks.some(c => c.textDelta === 'Output from session B')
      ).toBe(false);
    });
  });

  describe('Scenario: Bridge input displays in TUI with correct session', () => {
    it('should display bridge input and LLM response for bridged session', async () => {
      // @step Given the TUI is viewing session "session-main"
      const currentViewedSession = 'session-main';
      const displayedChunks: StreamChunk[] = [];

      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession('session-main');

      // TUI registers handler for the viewed session
      manager.registerHandler(
        currentViewedSession,
        (_sessionId: string, chunk: StreamChunk) => {
          displayedChunks.push(chunk);
        }
      );

      // @step And a bridge is connected to session "session-main"
      const bridgeSessionId = 'session-main';

      // @step When the bridge sends input to session "session-main"
      // Bridge input comes through as SupervisorInput chunk with session_id
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: 'Hello from bridge',
        authorName: 'BridgeUser',
      };
      injectTestChunk(bridgeSessionId, supervisorInputChunk);

      // @step Then the TUI should display the bridge input
      expect(displayedChunks).toContainEqual(supervisorInputChunk);

      // @step And the TUI should display the LLM response chunks
      const llmResponseChunk: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'LLM response to bridge input',
      };
      injectTestChunk(bridgeSessionId, llmResponseChunk);

      expect(displayedChunks).toHaveLength(2);
      expect(displayedChunks[1]).toEqual(llmResponseChunk);

      // @step And all displayed chunks should have session_id "session-main"
      // Verified implicitly - all chunks came through the session-main handler
      expect(displayedChunks).toHaveLength(2);
    });
  });

  describe('Scenario: Session switch changes which chunks are displayed', () => {
    it('should display chunks from new session after switch', async () => {
      // @step Given the TUI is viewing session "session-a"
      let currentViewedSession = 'session-a';
      const displayedChunks: StreamChunk[] = [];

      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession('session-a');
      manager.subscribeToSession('session-b');

      // Register handler for session-a initially
      const displayHandler = (_sessionId: string, chunk: StreamChunk) => {
        displayedChunks.push(chunk);
      };
      let unregister = manager.registerHandler(
        currentViewedSession,
        displayHandler
      );

      // @step And session "session-b" is running in background

      // @step When the user switches to view session "session-b"
      unregister(); // Unregister from session-a
      currentViewedSession = 'session-b';
      unregister = manager.registerHandler(
        currentViewedSession,
        displayHandler
      );

      // Clear displayed chunks for new view
      displayedChunks.length = 0;

      // @step And session "session-a" emits a chunk
      const chunkA: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Late chunk from A',
      };
      injectTestChunk('session-a', chunkA);

      // @step And session "session-b" emits a chunk
      const chunkB: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Chunk from B',
      };
      injectTestChunk('session-b', chunkB);

      // @step Then only the chunk from session "session-b" should be displayed
      expect(displayedChunks).toHaveLength(1);
      expect(displayedChunks[0]).toEqual(chunkB);
    });
  });
});
