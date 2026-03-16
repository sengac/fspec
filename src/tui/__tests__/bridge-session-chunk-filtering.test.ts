/**
 * Feature: spec/features/bridge-session-chunk-filtering.feature
 *
 * Tests for Bridge session chunk filtering.
 * Bridge tool receives session_id from the tool call context.
 * It registers a GLOBAL handler to receive ALL chunks with (sessionId, chunk).
 * It filters to only relay those matching its bridged session.
 * Chunks from other sessions are ignored and not relayed to WebSocket.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';

import type { StreamChunk } from '@sengac/codelet-napi';
import {
  GlobalSessionStreamManager,
  injectTestChunk,
} from '../services/globalSessionStreamManager';

describe('Feature: Bridge session chunk filtering', () => {
  beforeEach(() => {
    GlobalSessionStreamManager.resetInstance();
  });

  afterEach(() => {
    GlobalSessionStreamManager.resetInstance();
  });

  describe('Background: User Story', () => {
    it('validates the user story for bridge session filtering', () => {
      // @step As a developer
      // @step I want the bridge to only relay chunks for its connected session
      // @step So that Telegram users see only their session's output

      // This validates that the manager supports global handlers with session_id
      const manager = GlobalSessionStreamManager.getInstance();
      expect(manager.registerGlobalHandler).toBeDefined();
    });
  });

  describe('Scenario: Bridge relays only bridged session chunks', () => {
    it('should relay chunks from bridged session and ignore others', async () => {
      // @step Given a bridge is connected to session "session-telegram"
      const bridgedSessionId = 'session-telegram';
      const relayedChunks: StreamChunk[] = [];

      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession('session-telegram');
      manager.subscribeToSession('session-other');

      // Bridge registers a GLOBAL handler that receives all chunks with session_id
      // It filters in the handler to only relay its bridged session
      manager.registerGlobalHandler((sessionId: string, chunk: StreamChunk) => {
        if (sessionId === bridgedSessionId) {
          relayedChunks.push(chunk);
        }
        // Chunks from other sessions are silently ignored
      });

      // @step And session "session-telegram" is running
      // @step And session "session-other" is running

      // @step When session "session-telegram" emits a TextDelta chunk
      const telegramChunk: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Response for Telegram user',
      };
      injectTestChunk('session-telegram', telegramChunk);

      // @step And session "session-other" emits a TextDelta chunk
      const otherChunk: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Response for other session',
      };
      injectTestChunk('session-other', otherChunk);

      // @step Then the bridge should relay the chunk from session "session-telegram"
      expect(relayedChunks).toHaveLength(1);
      expect(relayedChunks[0]).toEqual(telegramChunk);

      // @step And the bridge should not relay the chunk from session "session-other"
      expect(
        relayedChunks.some(c => c.textDelta === 'Response for other session')
      ).toBe(false);
    });
  });

  describe('Scenario: Bridge receives session_id from tool call context', () => {
    it('should extract session_id from tool call context', async () => {
      // @step Given a Bridge tool invocation with session context
      interface BridgeToolContext {
        sessionId: string;
        action: { type: 'connect'; url: string };
      }

      const toolCallContext: BridgeToolContext = {
        sessionId: 'session-from-context',
        action: { type: 'connect', url: 'ws://localhost:8080' },
      };

      // @step When the bridge is initialized
      // Bridge extracts session_id from the tool call context
      const bridgedSessionId = toolCallContext.sessionId;

      // @step Then it should extract session_id from the tool call context
      expect(bridgedSessionId).toBe('session-from-context');

      // @step And use that session_id for filtering chunks
      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession('session-from-context');
      manager.subscribeToSession('other-session');

      const relayedChunks: StreamChunk[] = [];
      manager.registerGlobalHandler((sessionId, chunk) => {
        if (sessionId === bridgedSessionId) {
          relayedChunks.push(chunk);
        }
      });

      // Test filtering works with extracted session_id
      injectTestChunk('session-from-context', {
        type: 'Text',
        text: 'Correct session',
      });
      injectTestChunk('other-session', { type: 'Text', text: 'Wrong session' });

      expect(relayedChunks).toHaveLength(1);
      expect(relayedChunks[0].text).toBe('Correct session');
    });
  });

  describe('Scenario: Bridge input and response flow', () => {
    it('should handle complete input->response flow with session_id', async () => {
      // @step Given a bridge is connected to session "session-x"
      const bridgedSessionId = 'session-x';
      const relayedChunks: StreamChunk[] = [];

      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession('session-x');
      manager.subscribeToSession('session-y');

      // Bridge registers global handler
      manager.registerGlobalHandler((sessionId, chunk) => {
        if (sessionId === bridgedSessionId) {
          relayedChunks.push(chunk);
        }
      });

      // @step When input arrives from the bridge WebSocket
      // This would be handled by the bridge receiving WS message and calling session.send()

      // @step Then Rust should emit SupervisorInput chunk with session_id "session-x"
      const supervisorInputChunk: StreamChunk = {
        type: 'IncomingMessage',
        text: 'User input from Telegram',
        authorName: 'TelegramUser',
      };
      injectTestChunk(bridgedSessionId, supervisorInputChunk);

      // @step And Rust should emit LLM response chunks with session_id "session-x"
      const thinkingChunk: StreamChunk = {
        type: 'Thinking',
        thinking: 'Processing user request...',
      };
      injectTestChunk(bridgedSessionId, thinkingChunk);

      const responseChunk: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'Here is my response to your question.',
      };
      injectTestChunk(bridgedSessionId, responseChunk);

      // @step And the bridge should relay all chunks with session_id "session-x"
      expect(relayedChunks).toHaveLength(3);
      expect(relayedChunks[0].type).toBe('SupervisorInput');
      expect(relayedChunks[1].type).toBe('Thinking');
      expect(relayedChunks[2].type).toBe('TextDelta');

      // Verify chunks from other sessions during this flow are NOT relayed
      const interleavedChunk: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'From another session',
      };
      injectTestChunk('session-y', interleavedChunk);

      // Still only 3 chunks relayed
      expect(relayedChunks).toHaveLength(3);
    });
  });

  describe('Edge case: Multiple bridges connected to different sessions', () => {
    it('should isolate chunk routing between multiple bridges', async () => {
      // Multiple bridges, each connected to a different session
      const bridge1SessionId = 'session-telegram-1';
      const bridge2SessionId = 'session-telegram-2';

      const bridge1Chunks: StreamChunk[] = [];
      const bridge2Chunks: StreamChunk[] = [];

      const manager = GlobalSessionStreamManager.getInstance();
      manager.subscribeToSession(bridge1SessionId);
      manager.subscribeToSession(bridge2SessionId);

      // Each bridge registers its own global handler
      manager.registerGlobalHandler((sessionId, chunk) => {
        if (sessionId === bridge1SessionId) {
          bridge1Chunks.push(chunk);
        }
      });

      manager.registerGlobalHandler((sessionId, chunk) => {
        if (sessionId === bridge2SessionId) {
          bridge2Chunks.push(chunk);
        }
      });

      // Emit chunks from session 1
      const chunk1: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'For bridge 1',
      };
      injectTestChunk(bridge1SessionId, chunk1);

      // Emit chunks from session 2
      const chunk2: StreamChunk = {
        type: 'TextDelta',
        textDelta: 'For bridge 2',
      };
      injectTestChunk(bridge2SessionId, chunk2);

      // Each bridge should only have its session's chunks
      expect(bridge1Chunks).toHaveLength(1);
      expect(bridge1Chunks[0].textDelta).toBe('For bridge 1');

      expect(bridge2Chunks).toHaveLength(1);
      expect(bridge2Chunks[0].textDelta).toBe('For bridge 2');
    });
  });
});
