/**
 * Unit tests for work unit attachment logic in SplitSessionView
 *
 * SESS-001: Test work unit retrieval logic without full component rendering
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useFspecStore } from '../../store/fspecStore';

describe('SplitSessionView - Work Unit Logic', () => {
  beforeEach(() => {
    // Reset store state before each test
    useFspecStore.setState({
      sessionAttachments: new Map<string, string>(),
    });
  });

  describe('work unit ID retrieval logic', () => {
    it('should retrieve work unit ID when session is attached', () => {
      const sessionId = 'test-session-123';
      const workUnitId = 'STORY-001';
      const store = useFspecStore.getState();

      // Attach session to work unit
      store.attachSession(workUnitId, sessionId);

      // This is the logic used in SplitSessionView
      const retrievedWorkUnitId = store.getWorkUnitBySession(sessionId);

      expect(retrievedWorkUnitId).toBe(workUnitId);
    });

    it('should return undefined when session is not attached to any work unit', () => {
      const sessionId = 'unattached-session-456';
      const store = useFspecStore.getState();

      // Don't attach anything
      const retrievedWorkUnitId = store.getWorkUnitBySession(sessionId);

      expect(retrievedWorkUnitId).toBeUndefined();
    });

    it('should update retrieved work unit when attachment changes', () => {
      const sessionId = 'test-session-789';
      const initialWorkUnit = 'STORY-001';
      const updatedWorkUnit = 'STORY-002';
      const store = useFspecStore.getState();

      // Initial attachment
      store.attachSession(initialWorkUnit, sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(initialWorkUnit);

      // Update attachment
      store.attachSession(updatedWorkUnit, sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(updatedWorkUnit);
    });

    it('should handle multiple sessions with different work units', () => {
      const session1 = 'session-1';
      const session2 = 'session-2';
      const workUnit1 = 'STORY-001';
      const workUnit2 = 'STORY-002';
      const store = useFspecStore.getState();

      store.attachSession(workUnit1, session1);
      store.attachSession(workUnit2, session2);

      expect(store.getWorkUnitBySession(session1)).toBe(workUnit1);
      expect(store.getWorkUnitBySession(session2)).toBe(workUnit2);
    });

    it('should handle detaching sessions', () => {
      const sessionId = 'test-session-detach';
      const workUnitId = 'STORY-001';
      const store = useFspecStore.getState();

      // Attach and verify
      store.attachSession(workUnitId, sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);

      // Detach and verify
      store.detachSession(workUnitId);
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });
  });

  describe('attachment state validation', () => {
    it('should correctly report attachment status', () => {
      const sessionId = 'status-test-session';
      const workUnitId = 'STORY-001';
      const store = useFspecStore.getState();

      expect(store.hasAttachedSession(workUnitId)).toBe(false);

      store.attachSession(workUnitId, sessionId);
      expect(store.hasAttachedSession(workUnitId)).toBe(true);

      store.detachSession(workUnitId);
      expect(store.hasAttachedSession(workUnitId)).toBe(false);
    });

    it('should handle clearing all attachments', () => {
      const store = useFspecStore.getState();

      // Create multiple attachments
      store.attachSession('STORY-001', 'session-1');
      store.attachSession('STORY-002', 'session-2');

      expect(store.getWorkUnitBySession('session-1')).toBe('STORY-001');
      expect(store.getWorkUnitBySession('session-2')).toBe('STORY-002');

      // Clear all
      store.clearAllSessionAttachments();

      expect(store.getWorkUnitBySession('session-1')).toBeUndefined();
      expect(store.getWorkUnitBySession('session-2')).toBeUndefined();
    });
  });
});
