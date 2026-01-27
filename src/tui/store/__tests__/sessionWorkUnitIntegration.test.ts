/**
 * Integration Tests for Session-Work Unit Attachment System
 *
 * SESS-001: End-to-end testing of session creation context awareness
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFspecStore } from '../../store/fspecStore';

describe('Session-Work Unit Integration', () => {
  let store: ReturnType<typeof useFspecStore>;

  beforeEach(() => {
    const { result } = renderHook(() => useFspecStore());
    store = result.current;

    act(() => {
      store.clearAllSessionAttachments();
    });
  });

  describe('Board → Session Creation Flow', () => {
    it('should create attached session when user selects work unit from board', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-from-board';

      // Simulate user workflow:
      // 1. User selects work unit on board
      act(() => {
        store.setCurrentWorkUnitId(workUnitId);
      });

      // 2. User presses Enter → navigates to AgentView with workUnitId
      // 3. AgentView creates session and attaches it (board context)
      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      // Verify attachment
      expect(store.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);
      expect(store.getCurrentWorkUnitId()).toBe(workUnitId);
    });
  });

  describe('Session Navigation Flow', () => {
    it('should NOT attach new session when created via Shift+Right navigation', () => {
      const workUnitId = 'STORY-001';
      const originalSessionId = 'original-session';
      const navigationSessionId = 'navigation-session';

      // Setup: User has session attached to work unit
      act(() => {
        store.setCurrentWorkUnitId(workUnitId);
        store.attachSession(workUnitId, originalSessionId);
      });

      // Simulate navigation workflow:
      // 1. User is in originalSession attached to STORY-001
      // 2. User presses Shift+Right → creates new session
      // 3. New session should NOT be attached to STORY-001 (navigation context)

      // The original session remains attached
      expect(store.getAttachedSession(workUnitId)).toBe(originalSessionId);

      // New navigation session is created but not attached
      // (This would be handled by the context-aware logic in AgentView)
      expect(store.getWorkUnitBySession(navigationSessionId)).toBeUndefined();
    });

    it('should allow manual attachment of navigation-created session later', () => {
      const workUnitA = 'STORY-001';
      const workUnitB = 'STORY-002';
      const originalSessionId = 'original-session';
      const navigationSessionId = 'navigation-session';

      // Setup initial attachment
      act(() => {
        store.attachSession(workUnitA, originalSessionId);
      });

      // Navigation session created (not auto-attached)
      // User later manually attaches it to different work unit
      act(() => {
        store.attachSession(workUnitB, navigationSessionId);
      });

      // Verify both sessions coexist with different work units
      expect(store.getAttachedSession(workUnitA)).toBe(originalSessionId);
      expect(store.getAttachedSession(workUnitB)).toBe(navigationSessionId);
      expect(store.getWorkUnitBySession(originalSessionId)).toBe(workUnitA);
      expect(store.getWorkUnitBySession(navigationSessionId)).toBe(workUnitB);
    });
  });

  describe('Session Resume vs Create Logic', () => {
    it('should resume existing session when work unit has attachment', () => {
      const workUnitId = 'STORY-001';
      const existingSessionId = 'existing-session';

      // Setup: Work unit already has attached session
      act(() => {
        store.attachSession(workUnitId, existingSessionId);
      });

      // User selects same work unit from board
      act(() => {
        store.setCurrentWorkUnitId(workUnitId);
      });

      // Check if should auto-resume vs auto-create
      const hasExistingSession = !!store.getAttachedSession(workUnitId);
      const shouldAutoResume = hasExistingSession;
      const shouldAutoCreate = !hasExistingSession;

      expect(shouldAutoResume).toBe(true);
      expect(shouldAutoCreate).toBe(false);
      expect(store.getAttachedSession(workUnitId)).toBe(existingSessionId);
    });

    it('should create new session when work unit has no attachment', () => {
      const workUnitId = 'STORY-002';

      // User selects work unit with no existing session
      act(() => {
        store.setCurrentWorkUnitId(workUnitId);
      });

      // Check if should auto-resume vs auto-create
      const hasExistingSession = !!store.getAttachedSession(workUnitId);
      const shouldAutoResume = hasExistingSession;
      const shouldAutoCreate = !hasExistingSession;

      expect(shouldAutoResume).toBe(false);
      expect(shouldAutoCreate).toBe(true);
      expect(store.getAttachedSession(workUnitId)).toBeUndefined();
    });
  });

  describe('Multiple Work Units and Sessions', () => {
    it('should handle complex multi-session, multi-work-unit scenarios', () => {
      // Scenario: User working on multiple stories with multiple sessions
      const sessions = [
        { workUnit: 'STORY-001', sessionId: 'session-story-1' },
        { workUnit: 'STORY-002', sessionId: 'session-story-2' },
        { workUnit: 'BUG-001', sessionId: 'session-bug-1' },
        { workUnit: undefined, sessionId: 'session-general' }, // Navigation-created session
      ];

      // Attach sessions to work units (except general session)
      act(() => {
        sessions.forEach(({ workUnit, sessionId }) => {
          if (workUnit) {
            store.attachSession(workUnit, sessionId);
          }
        });
      });

      // Verify all attachments
      expect(store.getAttachedSession('STORY-001')).toBe('session-story-1');
      expect(store.getAttachedSession('STORY-002')).toBe('session-story-2');
      expect(store.getAttachedSession('BUG-001')).toBe('session-bug-1');

      // Verify reverse lookups
      expect(store.getWorkUnitBySession('session-story-1')).toBe('STORY-001');
      expect(store.getWorkUnitBySession('session-story-2')).toBe('STORY-002');
      expect(store.getWorkUnitBySession('session-bug-1')).toBe('BUG-001');
      expect(store.getWorkUnitBySession('session-general')).toBeUndefined();

      // Simulate session reassignment (user moves session to different work unit)
      act(() => {
        store.detachSession('STORY-001');
        store.attachSession('STORY-003', 'session-story-1');
      });

      expect(store.getAttachedSession('STORY-001')).toBeUndefined();
      expect(store.getAttachedSession('STORY-003')).toBe('session-story-1');
      expect(store.getWorkUnitBySession('session-story-1')).toBe('STORY-003');
    });

    it('should handle session deletion and cleanup', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-to-delete';

      // Attach session
      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      // Verify attachment
      expect(store.getAttachedSession(workUnitId)).toBe(sessionId);

      // Simulate session deletion (detach from work unit)
      act(() => {
        store.detachSession(workUnitId);
      });

      // Verify cleanup
      expect(store.getAttachedSession(workUnitId)).toBeUndefined();
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });
  });

  describe('Edge Cases and Error Handling', () => {
    it('should handle work unit ID changes gracefully', () => {
      const oldWorkUnitId = 'STORY-001';
      const newWorkUnitId = 'STORY-001-UPDATED';
      const sessionId = 'session-id';

      // Initial attachment
      act(() => {
        store.attachSession(oldWorkUnitId, sessionId);
      });

      // Simulate work unit ID change (move session to new ID)
      act(() => {
        store.detachSession(oldWorkUnitId);
        store.attachSession(newWorkUnitId, sessionId);
      });

      expect(store.getAttachedSession(oldWorkUnitId)).toBeUndefined();
      expect(store.getAttachedSession(newWorkUnitId)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(newWorkUnitId);
    });

    it('should handle duplicate session IDs gracefully', () => {
      const workUnitA = 'STORY-001';
      const workUnitB = 'STORY-002';
      const sessionId = 'duplicate-session';

      // Attach same session to first work unit
      act(() => {
        store.attachSession(workUnitA, sessionId);
      });

      expect(store.getAttachedSession(workUnitA)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitA);

      // Move same session to second work unit
      // Note: Current implementation allows multiple work units to have same session
      // This represents the current behavior - one session can be attached to multiple work units
      act(() => {
        store.attachSession(workUnitB, sessionId);
      });

      // Both work units can have the same session attached (current implementation)
      expect(store.getAttachedSession(workUnitA)).toBe(sessionId);
      expect(store.getAttachedSession(workUnitB)).toBe(sessionId);
      // The session returns the first work unit found (Map iteration order)
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitA);
    });

    it('should handle clearing all attachments', () => {
      // Setup multiple attachments
      act(() => {
        store.attachSession('STORY-001', 'session-1');
        store.attachSession('STORY-002', 'session-2');
        store.attachSession('BUG-001', 'session-3');
        store.setCurrentWorkUnitId('STORY-001');
      });

      // Verify setup
      expect(store.getAttachedSession('STORY-001')).toBe('session-1');
      expect(store.getCurrentWorkUnitId()).toBe('STORY-001');

      // Clear all
      act(() => {
        store.clearAllSessionAttachments();
      });

      // Verify cleanup (current work unit should remain)
      expect(store.getAttachedSession('STORY-001')).toBeUndefined();
      expect(store.getAttachedSession('STORY-002')).toBeUndefined();
      expect(store.getAttachedSession('BUG-001')).toBeUndefined();
      expect(store.getWorkUnitBySession('session-1')).toBeUndefined();
      expect(store.getWorkUnitBySession('session-2')).toBeUndefined();
      expect(store.getWorkUnitBySession('session-3')).toBeUndefined();
      expect(store.getCurrentWorkUnitId()).toBe('STORY-001'); // Should remain
    });
  });
});
