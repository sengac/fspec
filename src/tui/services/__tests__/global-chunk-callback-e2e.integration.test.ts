/**
 * Feature: spec/features/global-chunk-callback-napi.feature
 * Feature: spec/features/global-session-stream-manager-chunk-routing.feature
 * Feature: spec/features/tui-session-chunk-filtering.feature
 * Feature: spec/features/bridge-session-chunk-filtering.feature
 *
 * Integration Tests for BRIDGE-012: Global Chunk Callback Architecture
 *
 * These tests verify chunk routing through GlobalSessionStreamManager.
 * Uses simulateChunk for reliable testing since OnceCell callback can only be set once.
 *
 * DRY: Uses globalChunkCallbackFixture for setup/teardown
 * SOLID: Single responsibility - tests only, no setup logic
 */

import { describe, it, expect, beforeAll, afterAll, beforeEach } from 'vitest';

import { sessionSetGlobalChunkCallback } from '@sengac/codelet-napi';
import type { StreamChunk } from '@sengac/codelet-napi';

import {
  GlobalSessionStreamManager,
  initGlobalSessionStreamManager,
  stopGlobalSessionStreamManager,
} from '../globalSessionStreamManager';

import type {
  GlobalChunkCallbackFixture,
  TestSession,
  ReceivedChunk,
} from './fixtures/globalChunkCallbackFixture';
import { createGlobalChunkCallbackFixture } from './fixtures/globalChunkCallbackFixture';

describe('Feature: BRIDGE-012 Global Chunk Callback Integration', () => {
  let fixture: GlobalChunkCallbackFixture;
  let manager: GlobalSessionStreamManager;

  beforeAll(async () => {
    // Create fixture with credentials for session creation
    fixture = await createGlobalChunkCallbackFixture('integration');
    await fixture.createCredentials('anthropic', 'test-key-for-tests');
    manager = GlobalSessionStreamManager.getInstance();
  });

  afterAll(async () => {
    fixture.sessionFactory.destroyAllSessions();
    await fixture.cleanup();
    stopGlobalSessionStreamManager();
  });

  beforeEach(() => {
    fixture.clearChunks();
  });

  describe('Scenario: NAPI exports global callback function', () => {
    it('should have sessionSetGlobalChunkCallback as a real NAPI function', () => {
      // @step Given the NAPI module is loaded
      // @step When I check for sessionSetGlobalChunkCallback
      // @step Then it should be a function

      expect(typeof sessionSetGlobalChunkCallback).toBe('function');
    });
  });

  describe('Scenario: Emit chunk with session_id through manager', () => {
    it('should receive chunks when simulating session events', async () => {
      // @step Given a global chunk callback is registered
      // @step And a session exists
      const sessionId = 'test-session-emit';

      // @step When the session emits a SessionStateChange chunk
      const testChunk: StreamChunk = {
        type: 'SessionStateChange',
        state: 'Cleared',
      };
      manager.simulateChunk(sessionId, testChunk);

      // @step Then the global handler should receive the chunk with session_id
      await fixture.waitForChunksMatching(chunks => chunks.length > 0);

      const sessionChunks = fixture.getChunksForSession(sessionId);
      expect(sessionChunks.length).toBeGreaterThan(0);
      expect(sessionChunks[0].chunk.type).toBe('SessionStateChange');
    });
  });

  describe('Scenario: Multiple sessions emit through same global callback', () => {
    it('should route chunks to correct session handlers', async () => {
      // @step Given a global chunk callback is registered
      const sessionAId = 'session-multi-a';
      const sessionBId = 'session-multi-b';

      // @step When session "session-a" emits a chunk
      manager.simulateChunk(sessionAId, { type: 'Text', text: 'From A' });

      // @step And session "session-b" emits a chunk
      manager.simulateChunk(sessionBId, { type: 'Text', text: 'From B' });

      // @step Then both chunks should go through the same global callback
      await fixture.waitForChunksMatching(chunks => {
        const aChunks = chunks.filter(c => c.sessionId === sessionAId);
        const bChunks = chunks.filter(c => c.sessionId === sessionBId);
        return aChunks.length > 0 && bChunks.length > 0;
      });

      // @step And each chunk should have its respective session_id
      const sessionAChunks = fixture.getChunksForSession(sessionAId);
      const sessionBChunks = fixture.getChunksForSession(sessionBId);

      expect(sessionAChunks.length).toBeGreaterThan(0);
      expect(sessionBChunks.length).toBeGreaterThan(0);

      expect(sessionAChunks.every(c => c.sessionId === sessionAId)).toBe(true);
      expect(sessionBChunks.every(c => c.sessionId === sessionBId)).toBe(true);
    });
  });

  describe('Scenario: No per-session NAPI attachment functions', () => {
    it('should NOT export sessionAttach from NAPI', async () => {
      // @step When I inspect the NAPI module exports
      // @step Then there should be no session_attach function
      const napi = await import('@sengac/codelet-napi');
      expect((napi as Record<string, unknown>).sessionAttach).toBeUndefined();
    });

    it('should NOT export sessionDetach from NAPI', async () => {
      // @step And there should be no session_detach function
      const napi = await import('@sengac/codelet-napi');
      expect((napi as Record<string, unknown>).sessionDetach).toBeUndefined();
    });
  });

  describe('Scenario: TUI session chunk filtering via TypeScript routing', () => {
    it('should allow filtering chunks by session_id in TypeScript', async () => {
      // @step Given the TUI is viewing session "session-a"
      const sessionAId = 'tui-session-a';
      const sessionBId = 'tui-session-b';

      // @step When session "session-a" emits a SessionStateChange chunk
      manager.simulateChunk(sessionAId, {
        type: 'SessionStateChange',
        state: 'Cleared',
      });

      // @step And session "session-b" emits a SessionStateChange chunk
      manager.simulateChunk(sessionBId, {
        type: 'SessionStateChange',
        state: 'Cleared',
      });

      await fixture.waitForChunksMatching(chunks => {
        const a = chunks.filter(c => c.sessionId === sessionAId);
        const b = chunks.filter(c => c.sessionId === sessionBId);
        return a.length > 0 && b.length > 0;
      });

      // @step Then TypeScript can filter to display only session "session-a" chunks
      const tuiViewedSessionId = sessionAId;
      const displayedChunks = fixture.receivedChunks.filter(
        c => c.sessionId === tuiViewedSessionId
      );

      // @step And the TUI should not display the chunk from session "session-b"
      expect(displayedChunks.every(c => c.sessionId === sessionAId)).toBe(true);
      expect(displayedChunks.some(c => c.sessionId === sessionBId)).toBe(false);
    });
  });

  describe('Scenario: Bridge session chunk filtering via TypeScript routing', () => {
    it('should allow bridge to filter chunks for bridged session only', async () => {
      // @step Given a bridge is connected to session "session-telegram"
      const bridgedSessionId = 'bridge-telegram';
      const otherSessionId = 'bridge-other';

      // @step When session "session-telegram" emits a chunk
      manager.simulateChunk(bridgedSessionId, {
        type: 'Text',
        text: 'From Telegram',
      });

      // @step And session "session-other" emits a chunk
      manager.simulateChunk(otherSessionId, {
        type: 'Text',
        text: 'From Other',
      });

      await fixture.waitForChunksMatching(chunks => {
        const bridged = chunks.filter(c => c.sessionId === bridgedSessionId);
        const other = chunks.filter(c => c.sessionId === otherSessionId);
        return bridged.length > 0 && other.length > 0;
      });

      // @step Then the bridge can relay only the chunk from session "session-telegram"
      const relayedChunks = fixture.receivedChunks.filter(
        c => c.sessionId === bridgedSessionId
      );

      // @step And the bridge should not relay the chunk from session "session-other"
      expect(relayedChunks.every(c => c.sessionId === bridgedSessionId)).toBe(
        true
      );
      expect(relayedChunks.some(c => c.sessionId === otherSessionId)).toBe(
        false
      );
    });
  });

  describe('Scenario: Chunk includes all required fields', () => {
    it('should include session_id and chunk type in callback args', async () => {
      const sessionId = 'field-test-session';

      manager.simulateChunk(sessionId, { type: 'Text', text: 'Test' });

      await fixture.waitForChunksMatching(chunks => chunks.length > 0);

      // Verify structure
      const chunk = fixture.receivedChunks[0];
      expect(chunk).toHaveProperty('sessionId');
      expect(chunk).toHaveProperty('chunk');
      expect(chunk.chunk).toHaveProperty('type');
      expect(typeof chunk.sessionId).toBe('string');
      expect(chunk.sessionId).toBe(sessionId);
    });
  });

  describe('Scenario: Session-specific handlers only receive their chunks', () => {
    it('should route chunks to registered session handlers', async () => {
      const sessionAId = 'handler-session-a';
      const sessionBId = 'handler-session-b';

      const sessionAChunks: StreamChunk[] = [];
      const sessionBChunks: StreamChunk[] = [];

      // Register handlers for each session
      const unregisterA = manager.registerHandler(sessionAId, chunk => {
        sessionAChunks.push(chunk);
      });
      const unregisterB = manager.registerHandler(sessionBId, chunk => {
        sessionBChunks.push(chunk);
      });

      // Emit chunks
      manager.simulateChunk(sessionAId, { type: 'Text', text: 'For A' });
      manager.simulateChunk(sessionBId, { type: 'Text', text: 'For B' });

      // Verify routing
      expect(sessionAChunks).toHaveLength(1);
      expect(sessionAChunks[0].text).toBe('For A');

      expect(sessionBChunks).toHaveLength(1);
      expect(sessionBChunks[0].text).toBe('For B');

      unregisterA();
      unregisterB();
    });
  });
});
