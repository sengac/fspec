/**
 * Integration Tests for Session-Work Unit Attachment System
 *
 * SESS-001: End-to-end testing of session creation context awareness
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useFspecStore } from '../../store/fspecStore';

describe('Session-Work Unit Integration', () => {
  beforeEach(() => {
    // Reset store state before each test
    useFspecStore.setState({
      sessionAttachments: new Map<string, string>(),
      workUnits: [],
      currentWorkUnitId: null,
    });
  });

  describe('Board → Session Creation Flow', () => {
    it('should create attached session when user selects work unit from board', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-from-board';
      const store = useFspecStore.getState();

      // Simulate user workflow:
      // 1. User selects work unit on board
      store.setCurrentWorkUnitId(workUnitId);

      // 2. User creates session (simulated - normally done by session manager)
      store.attachSession(workUnitId, sessionId);

      // 3. Verify session is attached to the work unit
      expect(store.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);
    });

    it('should maintain attachment when switching between sessions', () => {
      const workUnit1 = 'STORY-001';
      const workUnit2 = 'STORY-002';
      const session1 = 'session-1';
      const session2 = 'session-2';
      const store = useFspecStore.getState();

      // Create attachments for two different work units
      store.attachSession(workUnit1, session1);
      store.attachSession(workUnit2, session2);

      // Switch context to first work unit
      store.setCurrentWorkUnitId(workUnit1);
      expect(store.getAttachedSession(workUnit1)).toBe(session1);

      // Switch context to second work unit
      store.setCurrentWorkUnitId(workUnit2);
      expect(store.getAttachedSession(workUnit2)).toBe(session2);

      // Verify first attachment still exists
      expect(store.getAttachedSession(workUnit1)).toBe(session1);
    });
  });

  describe('Direct Navigation → Session Creation Flow', () => {
    it('should create unattached session when user navigates directly to agent view', () => {
      const sessionId = 'unattached-session';
      const store = useFspecStore.getState();

      // Simulate direct navigation (no work unit selection)
      // User lands on agent view without selecting a work unit first

      // Verify no work unit is selected
      expect(store.getCurrentWorkUnitId()).toBeNull();

      // Session is created but not attached to any work unit
      // (This would be handled by session creation logic, but we simulate the end state)
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });

    it('should allow manual attachment of session to work unit later', () => {
      const sessionId = 'initially-unattached';
      const workUnitId = 'STORY-001';
      const store = useFspecStore.getState();

      // Start with unattached session
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();

      // User manually attaches session to work unit (e.g., via command or UI)
      store.attachSession(workUnitId, sessionId);

      // Verify attachment is created
      expect(store.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);
    });
  });

  describe('Session Lifecycle Management', () => {
    it('should handle session termination and cleanup', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-to-terminate';
      const store = useFspecStore.getState();

      // Create attachment
      store.attachSession(workUnitId, sessionId);
      expect(store.hasAttachedSession(workUnitId)).toBe(true);

      // Simulate session termination
      store.detachSession(workUnitId);

      // Verify cleanup
      expect(store.hasAttachedSession(workUnitId)).toBe(false);
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });

    it('should support session migration between work units', () => {
      const originalWorkUnit = 'STORY-001';
      const newWorkUnit = 'STORY-002';
      const sessionId = 'migrating-session';
      const store = useFspecStore.getState();

      // Start with session attached to original work unit
      store.attachSession(originalWorkUnit, sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(originalWorkUnit);

      // Migrate session to new work unit
      store.attachSession(newWorkUnit, sessionId);

      // Verify migration
      expect(store.getWorkUnitBySession(sessionId)).toBe(newWorkUnit);
      expect(store.getAttachedSession(newWorkUnit)).toBe(sessionId);

      // Original work unit should no longer have this session
      expect(store.getAttachedSession(originalWorkUnit)).toBeUndefined();
    });
  });

  describe('Multi-User/Multi-Session Scenarios', () => {
    it('should handle multiple sessions for the same work unit', () => {
      const workUnitId = 'STORY-001';
      const session1 = 'session-1';
      const session2 = 'session-2';
      const store = useFspecStore.getState();

      // Attach first session
      store.attachSession(workUnitId, session1);
      expect(store.getAttachedSession(workUnitId)).toBe(session1);

      // Attach second session (should overwrite)
      store.attachSession(workUnitId, session2);
      expect(store.getAttachedSession(workUnitId)).toBe(session2);

      // Original session should no longer be associated with this work unit
      expect(store.getWorkUnitBySession(session1)).toBeUndefined();
    });

    it('should handle concurrent work on different work units', () => {
      const sessions = [
        { workUnit: 'STORY-001', session: 'session-1' },
        { workUnit: 'STORY-002', session: 'session-2' },
        { workUnit: 'BUG-001', session: 'session-3' },
      ];
      const store = useFspecStore.getState();

      // Create multiple concurrent attachments
      sessions.forEach(({ workUnit, session }) => {
        store.attachSession(workUnit, session);
      });

      // Verify all attachments exist independently
      sessions.forEach(({ workUnit, session }) => {
        expect(store.getAttachedSession(workUnit)).toBe(session);
        expect(store.getWorkUnitBySession(session)).toBe(workUnit);
      });
    });
  });

  describe('Error Handling and Edge Cases', () => {
    it('should gracefully handle attachment operations on non-existent work units', () => {
      const store = useFspecStore.getState();

      // Should not throw when attaching to non-existent work unit
      expect(() => {
        store.attachSession('NON-EXISTENT', 'some-session');
      }).not.toThrow();

      // Attachment should still work (store doesn't validate work unit existence)
      expect(store.getAttachedSession('NON-EXISTENT')).toBe('some-session');
    });

    it('should handle rapid attachment/detachment operations', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'rapid-test-session';
      const store = useFspecStore.getState();

      // Rapid attach/detach cycles
      for (let i = 0; i < 10; i++) {
        store.attachSession(workUnitId, sessionId);
        expect(store.hasAttachedSession(workUnitId)).toBe(true);

        store.detachSession(workUnitId);
        expect(store.hasAttachedSession(workUnitId)).toBe(false);
      }
    });

    it('should maintain data integrity during bulk operations', () => {
      const store = useFspecStore.getState();

      // Create many attachments
      const attachments = Array.from({ length: 100 }, (_, i) => ({
        workUnit: `STORY-${String(i + 1).padStart(3, '0')}`,
        session: `session-${i + 1}`,
      }));

      attachments.forEach(({ workUnit, session }) => {
        store.attachSession(workUnit, session);
      });

      // Verify all attachments
      attachments.forEach(({ workUnit, session }) => {
        expect(store.getAttachedSession(workUnit)).toBe(session);
        expect(store.getWorkUnitBySession(session)).toBe(workUnit);
      });

      // Clear all
      store.clearAllSessionAttachments();

      // Verify cleanup
      attachments.forEach(({ workUnit, session }) => {
        expect(store.hasAttachedSession(workUnit)).toBe(false);
        expect(store.getWorkUnitBySession(session)).toBeUndefined();
      });
    });
  });
});
