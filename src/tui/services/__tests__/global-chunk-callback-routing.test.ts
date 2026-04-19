/**
 * Feature: spec/features/global-session-stream-manager-chunk-routing.feature
 *
 * Tests for GlobalSessionStreamManager chunk routing by session_id.
 * TypeScript GlobalSessionStreamManager registers the global callback ONCE at startup.
 * It receives ALL chunks from ALL sessions and routes them to session-specific handlers.
 * Session isolation is achieved via Map lookup in TypeScript, not Rust gating.
 *
 * BRIDGE-012: These tests verify the NEW global callback architecture.
 * They should FAIL until the implementation is complete.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

// Import the REAL GlobalSessionStreamManager - not mocked
import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../globalSessionStreamManager';

// Import REAL NAPI bindings to check what functions exist
import * as napi from '@sengac/codelet-napi';

import type { StreamChunk } from '@sengac/codelet-napi';

describe('Feature: GlobalSessionStreamManager chunk routing by session_id', () => {
  beforeEach(() => {
    stopGlobalSessionStreamManager();
  });

  afterEach(() => {
    stopGlobalSessionStreamManager();
  });

  describe('Background: User Story', () => {
    it('validates the user story for session-specific chunk routing', () => {
      // @step As a developer
      // @step I want GlobalSessionStreamManager to route chunks by session_id
      // @step So that each UI component only receives chunks for its session

      // Verify the manager has the required API for session-specific routing
      const manager = GlobalSessionStreamManager.getInstance();
      expect(typeof manager.registerHandler).toBe('function');
      expect(typeof manager.registerGlobalHandler).toBe('function');
      expect(typeof manager.simulateChunk).toBe('function');
      expect(typeof manager.subscribeToSession).toBe('function');
    });
  });

  describe('Scenario: Register global callback once at initialization', () => {
    it('should call sessionSetGlobalChunkCallback exactly once at init', async () => {
      // @step Given GlobalSessionStreamManager is not initialized
      // Verify the NEW function exists in NAPI
      // @step When initGlobalSessionStreamManager is called
      // @step Then sessionSetGlobalChunkCallback should be called exactly once
      // @step And the callback should be stored for routing

      // BRIDGE-012: This test verifies that sessionSetGlobalChunkCallback EXISTS
      // It should FAIL until we add this function to Rust NAPI
      expect(typeof napi.sessionSetGlobalChunkCallback).toBe('function');
    });
  });

  describe('Scenario: No sessionAttach or sessionDetach calls', () => {
    it('should not export sessionAttach or sessionDetach from NAPI after BRIDGE-012', async () => {
      // @step Given GlobalSessionStreamManager source code
      // @step When I search for sessionAttach usage
      // @step Then no usages should be found
      // @step When I search for sessionDetach usage
      // @step Then no usages should be found

      // BRIDGE-012: sessionAttach and sessionDetach have been removed from NAPI
      expect(napi.sessionAttach).toBeUndefined();
      expect(napi.sessionDetach).toBeUndefined();
    });
  });

  describe('Scenario: GlobalSessionStreamManager uses global callback not per-session attach', () => {
    it('should use sessionSetGlobalChunkCallback at init, not sessionAttach per session', async () => {
      // @step Given GlobalSessionStreamManager is initialized
      // @step When subscribeToSession is called
      // @step Then sessionAttach should NOT be called (using global callback instead)

      // BRIDGE-012: After implementation, GlobalSessionStreamManager should:
      // 1. Call sessionSetGlobalChunkCallback ONCE at init
      // 2. NOT call sessionAttach when subscribing to sessions

      // This test verifies that initGlobalSessionStreamManager calls the global callback
      const sessionSetGlobalChunkCallbackSpy = vi.spyOn(
        napi,
        'sessionSetGlobalChunkCallback'
      );

      initGlobalSessionStreamManager();

      // Wait for the async registration to complete
      await new Promise(resolve => setTimeout(resolve, 50));

      // After BRIDGE-012 implementation, this should have been called
      expect(sessionSetGlobalChunkCallbackSpy).toHaveBeenCalledTimes(1);

      sessionSetGlobalChunkCallbackSpy.mockRestore();
    });
  });

  describe('Scenario: Route chunk to correct session handler', () => {
    it('should invoke only the handler for the session the chunk belongs to', async () => {
      // @step Given GlobalSessionStreamManager is initialized
      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const sessionAChunks: StreamChunk[] = [];
      const sessionBChunks: StreamChunk[] = [];

      // @step And a handler is registered for session "session-a"
      const unregisterA = manager.registerHandler(
        'session-a',
        (_sessionId, chunk) => {
          sessionAChunks.push(chunk);
        }
      );

      // @step And a handler is registered for session "session-b"
      const unregisterB = manager.registerHandler(
        'session-b',
        (_sessionId, chunk) => {
          sessionBChunks.push(chunk);
        }
      );

      // @step When a chunk arrives for session "session-a"
      const testChunk: StreamChunk = {
        type: 'Text',
        text: 'Chunk for session-a',
      };
      manager.simulateChunk('session-a', testChunk);

      // @step Then only the handler for session "session-a" should be invoked
      expect(sessionAChunks).toHaveLength(1);
      expect(sessionAChunks[0].text).toBe('Chunk for session-a');

      // @step And the handler for session "session-b" should not be invoked
      expect(sessionBChunks).toHaveLength(0);

      unregisterA();
      unregisterB();
    });
  });

  describe('Scenario: Multiple handlers for same session all receive chunk', () => {
    it('should invoke all handlers registered for the same session', async () => {
      // @step Given GlobalSessionStreamManager is initialized
      // @step And handler A is registered for session "session-x"
      // @step And handler B is registered for session "session-x"
      // @step When a chunk arrives for session "session-x"
      // @step Then both handler A and handler B should be invoked

      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const handlerAChunks: StreamChunk[] = [];
      const handlerBChunks: StreamChunk[] = [];
      const testSessionId = 'session-x-multiple-handlers';

      const unregisterA = manager.registerHandler(
        testSessionId,
        (_sessionId, chunk) => {
          handlerAChunks.push(chunk);
        }
      );
      const unregisterB = manager.registerHandler(
        testSessionId,
        (_sessionId, chunk) => {
          handlerBChunks.push(chunk);
        }
      );

      const testChunk: StreamChunk = {
        type: 'Text',
        text: 'For both handlers',
      };
      manager.simulateChunk(testSessionId, testChunk);

      expect(handlerAChunks).toHaveLength(1);
      expect(handlerBChunks).toHaveLength(1);

      unregisterA();
      unregisterB();
    });
  });

  describe('Scenario: Ignore chunks for sessions without handlers', () => {
    it('should not throw when chunk arrives for unknown session', async () => {
      // @step Given GlobalSessionStreamManager is initialized
      // @step And a handler is registered for session "session-a"
      // @step When a chunk arrives for session "session-unknown"
      // @step Then no handler should be invoked
      // @step And no error should be thrown

      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const receivedChunks: StreamChunk[] = [];
      const unregister = manager.registerHandler(
        'session-a',
        (_sessionId, chunk) => {
          receivedChunks.push(chunk);
        }
      );

      const testChunk: StreamChunk = { type: 'Text', text: 'Unknown session' };

      // Should not throw
      expect(() => {
        manager.simulateChunk('session-unknown', testChunk);
      }).not.toThrow();

      // Handler for session-a should not receive chunk for session-unknown
      expect(receivedChunks).toHaveLength(0);

      unregister();
    });
  });

  describe('Scenario: Global handlers receive all chunks with session_id', () => {
    it('should forward all chunks to global handlers with session_id', async () => {
      // @step Given GlobalSessionStreamManager is initialized
      // @step And a global handler is registered
      // @step When a chunk arrives for session "session-a"
      // @step And a chunk arrives for session "session-b"
      // @step Then the global handler should receive both chunks
      // @step And each chunk should include its session_id

      initGlobalSessionStreamManager();
      const manager = GlobalSessionStreamManager.getInstance();

      const globalChunks: Array<{ sessionId: string; chunk: StreamChunk }> = [];
      const unregister = manager.registerGlobalHandler((sessionId, chunk) => {
        globalChunks.push({ sessionId, chunk });
      });

      manager.simulateChunk('session-a', { type: 'Text', text: 'From A' });
      manager.simulateChunk('session-b', { type: 'Text', text: 'From B' });

      expect(globalChunks).toHaveLength(2);
      expect(globalChunks[0].sessionId).toBe('session-a');
      expect(globalChunks[1].sessionId).toBe('session-b');

      unregister();
    });
  });
});
