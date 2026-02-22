/**
 * Integration Tests for Session-Work Unit Attachment System
 *
 * SESS-001: End-to-end testing of session creation context awareness
 * TUI-068: Updated to use sessionStore for currentWorkUnitId
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useFspecStore } from '../../store/fspecStore';
import { useSessionStore } from '../../store/sessionStore';

describe('Session-Work Unit Integration', () => {
  beforeEach(() => {
    // Reset fspecStore state
    useFspecStore.setState({
      sessionAttachments: new Map<string, string>(),
      workUnits: [],
    });
    // Reset sessionStore state
    useSessionStore.getState().setCurrentWorkUnit(null, null);
  });

  describe('Board → Session Creation Flow', () => {
    it('should create attached session when user selects work unit from board', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-from-board';
      const fspecStore = useFspecStore.getState();
      const sessionStore = useSessionStore.getState();

      // Simulate user workflow:
      // 1. User selects work unit on board
      sessionStore.setCurrentWorkUnit(workUnitId, 'specifying');

      // 2. User creates session (simulated - normally done by session manager)
      fspecStore.attachSession(workUnitId, sessionId);

      // 3. Verify session is attached to the work unit
      expect(fspecStore.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBe(workUnitId);
    });

    it('should maintain attachment when switching between sessions', () => {
      const workUnit1 = 'STORY-001';
      const workUnit2 = 'STORY-002';
      const session1 = 'session-1';
      const session2 = 'session-2';
      const fspecStore = useFspecStore.getState();
      const sessionStore = useSessionStore.getState();

      // Create attachments for two different work units
      fspecStore.attachSession(workUnit1, session1);
      fspecStore.attachSession(workUnit2, session2);

      // Switch context to first work unit
      sessionStore.setCurrentWorkUnit(workUnit1, 'specifying');
      expect(fspecStore.getAttachedSession(workUnit1)).toBe(session1);

      // Switch context to second work unit
      sessionStore.setCurrentWorkUnit(workUnit2, 'implementing');
      expect(fspecStore.getAttachedSession(workUnit2)).toBe(session2);

      // Verify first attachment still exists
      expect(fspecStore.getAttachedSession(workUnit1)).toBe(session1);
    });
  });

  describe('Direct Navigation → Session Creation Flow', () => {
    it('should create unattached session when user navigates directly to agent view', () => {
      const sessionId = 'unattached-session';
      const fspecStore = useFspecStore.getState();
      const sessionStore = useSessionStore.getState();

      // Simulate direct navigation (no work unit selection)
      // User lands on agent view without selecting a work unit first

      // Verify no work unit is selected
      expect(sessionStore.currentWorkUnitId).toBeNull();

      // Session is created but not attached to any work unit
      // (This would be handled by session creation logic, but we simulate the end state)
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBeUndefined();
    });

    it('should allow manual attachment of session to work unit later', () => {
      const sessionId = 'initially-unattached';
      const workUnitId = 'STORY-001';
      const fspecStore = useFspecStore.getState();

      // Start with unattached session
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBeUndefined();

      // User manually attaches session to work unit (e.g., via command or UI)
      fspecStore.attachSession(workUnitId, sessionId);

      // Verify attachment is created
      expect(fspecStore.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBe(workUnitId);
    });
  });

  describe('Session Lifecycle Management', () => {
    it('should handle session termination and cleanup', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-to-terminate';
      const fspecStore = useFspecStore.getState();

      // Create attachment
      fspecStore.attachSession(workUnitId, sessionId);
      expect(fspecStore.hasAttachedSession(workUnitId)).toBe(true);

      // Simulate session termination
      fspecStore.detachSession(workUnitId);

      // Verify cleanup
      expect(fspecStore.hasAttachedSession(workUnitId)).toBe(false);
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBeUndefined();
    });

    it('should support session migration between work units', () => {
      const originalWorkUnit = 'STORY-001';
      const newWorkUnit = 'STORY-002';
      const sessionId = 'migrating-session';
      const fspecStore = useFspecStore.getState();

      // Start with session attached to original work unit
      fspecStore.attachSession(originalWorkUnit, sessionId);
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBe(originalWorkUnit);

      // Migrate session to new work unit
      fspecStore.attachSession(newWorkUnit, sessionId);

      // Verify migration
      expect(fspecStore.getWorkUnitBySession(sessionId)).toBe(newWorkUnit);
      expect(fspecStore.getAttachedSession(newWorkUnit)).toBe(sessionId);

      // Original work unit should no longer have this session
      expect(fspecStore.getAttachedSession(originalWorkUnit)).toBeUndefined();
    });
  });

  describe('Multi-User/Multi-Session Scenarios', () => {
    it('should handle multiple sessions for the same work unit', () => {
      const workUnitId = 'STORY-001';
      const session1 = 'session-1';
      const session2 = 'session-2';
      const fspecStore = useFspecStore.getState();

      // Attach first session
      fspecStore.attachSession(workUnitId, session1);
      expect(fspecStore.getAttachedSession(workUnitId)).toBe(session1);

      // Attach second session (should overwrite)
      fspecStore.attachSession(workUnitId, session2);
      expect(fspecStore.getAttachedSession(workUnitId)).toBe(session2);

      // Original session should no longer be associated with this work unit
      expect(fspecStore.getWorkUnitBySession(session1)).toBeUndefined();
    });

    it('should handle concurrent work on different work units', () => {
      const sessions = [
        { workUnit: 'STORY-001', session: 'session-1' },
        { workUnit: 'STORY-002', session: 'session-2' },
        { workUnit: 'BUG-001', session: 'session-3' },
      ];
      const fspecStore = useFspecStore.getState();

      // Create multiple concurrent attachments
      sessions.forEach(({ workUnit, session }) => {
        fspecStore.attachSession(workUnit, session);
      });

      // Verify all attachments exist independently
      sessions.forEach(({ workUnit, session }) => {
        expect(fspecStore.getAttachedSession(workUnit)).toBe(session);
        expect(fspecStore.getWorkUnitBySession(session)).toBe(workUnit);
      });
    });
  });

  describe('Error Handling and Edge Cases', () => {
    it('should gracefully handle attachment operations on non-existent work units', () => {
      const fspecStore = useFspecStore.getState();

      // Should not throw when attaching to non-existent work unit
      expect(() => {
        fspecStore.attachSession('NON-EXISTENT', 'some-session');
      }).not.toThrow();

      // Attachment should still work (store doesn't validate work unit existence)
      expect(fspecStore.getAttachedSession('NON-EXISTENT')).toBe(
        'some-session'
      );
    });

    it('should handle rapid attachment/detachment operations', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'rapid-test-session';
      const fspecStore = useFspecStore.getState();

      // Rapid attach/detach cycles
      for (let i = 0; i < 10; i++) {
        fspecStore.attachSession(workUnitId, sessionId);
        expect(fspecStore.hasAttachedSession(workUnitId)).toBe(true);

        fspecStore.detachSession(workUnitId);
        expect(fspecStore.hasAttachedSession(workUnitId)).toBe(false);
      }
    });

    it('should maintain data integrity during bulk operations', () => {
      const fspecStore = useFspecStore.getState();

      // Create many attachments
      const attachments = Array.from({ length: 100 }, (_, i) => ({
        workUnit: `STORY-${String(i + 1).padStart(3, '0')}`,
        session: `session-${i + 1}`,
      }));

      attachments.forEach(({ workUnit, session }) => {
        fspecStore.attachSession(workUnit, session);
      });

      // Verify all attachments
      attachments.forEach(({ workUnit, session }) => {
        expect(fspecStore.getAttachedSession(workUnit)).toBe(session);
        expect(fspecStore.getWorkUnitBySession(session)).toBe(workUnit);
      });

      // Clear all
      fspecStore.clearAllSessionAttachments();

      // Verify cleanup
      attachments.forEach(({ workUnit, session }) => {
        expect(fspecStore.hasAttachedSession(workUnit)).toBe(false);
        expect(fspecStore.getWorkUnitBySession(session)).toBeUndefined();
      });
    });
  });
});
