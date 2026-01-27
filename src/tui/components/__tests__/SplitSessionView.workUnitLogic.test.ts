/**
 * Unit tests for work unit attachment logic in SplitSessionView
 *
 * SESS-001: Test work unit retrieval logic without full component rendering
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useFspecStore } from '../../store/fspecStore';

describe('SplitSessionView - Work Unit Logic', () => {
  let store: ReturnType<typeof useFspecStore>;

  beforeEach(() => {
    const { result } = renderHook(() => useFspecStore());
    store = result.current;

    act(() => {
      store.clearAllSessionAttachments();
    });
  });

  describe('work unit ID retrieval logic', () => {
    it('should retrieve work unit ID when session is attached', () => {
      const sessionId = 'test-session-123';
      const workUnitId = 'STORY-001';

      // Attach session to work unit
      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      // This is the logic used in SplitSessionView
      const retrievedWorkUnitId = store.getWorkUnitBySession(sessionId);

      expect(retrievedWorkUnitId).toBe(workUnitId);
    });

    it('should return undefined when session is not attached to any work unit', () => {
      const sessionId = 'unattached-session-456';

      // Don't attach anything
      const retrievedWorkUnitId = store.getWorkUnitBySession(sessionId);

      expect(retrievedWorkUnitId).toBeUndefined();
    });

    it('should update retrieved work unit when attachment changes', () => {
      const sessionId = 'test-session-789';
      const initialWorkUnit = 'STORY-001';
      const newWorkUnit = 'STORY-002';

      // Start with initial attachment
      act(() => {
        store.attachSession(initialWorkUnit, sessionId);
      });

      expect(store.getWorkUnitBySession(sessionId)).toBe(initialWorkUnit);

      // Change attachment
      act(() => {
        store.detachSession(initialWorkUnit);
        store.attachSession(newWorkUnit, sessionId);
      });

      expect(store.getWorkUnitBySession(sessionId)).toBe(newWorkUnit);
    });

    it('should handle attachment/detachment cycles', () => {
      const sessionId = 'cycling-session';
      const workUnitId = 'STORY-001';

      // Initially no attachment
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();

      // Attach
      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);

      // Detach
      act(() => {
        store.detachSession(workUnitId);
      });

      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();

      // Re-attach
      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);
    });

    it('should handle multiple sessions with different work units', () => {
      const sessions = [
        { sessionId: 'session-1', workUnitId: 'STORY-001' },
        { sessionId: 'session-2', workUnitId: 'STORY-002' },
        { sessionId: 'session-3', workUnitId: 'BUG-001' },
        { sessionId: 'session-4', workUnitId: undefined }, // No attachment
      ];

      // Attach sessions (except session-4)
      act(() => {
        sessions.forEach(({ sessionId, workUnitId }) => {
          if (workUnitId) {
            store.attachSession(workUnitId, sessionId);
          }
        });
      });

      // Verify retrieval
      expect(store.getWorkUnitBySession('session-1')).toBe('STORY-001');
      expect(store.getWorkUnitBySession('session-2')).toBe('STORY-002');
      expect(store.getWorkUnitBySession('session-3')).toBe('BUG-001');
      expect(store.getWorkUnitBySession('session-4')).toBeUndefined();
    });

    it('should handle empty or invalid session IDs gracefully', () => {
      expect(store.getWorkUnitBySession('')).toBeUndefined();
      expect(store.getWorkUnitBySession('nonexistent-session')).toBeUndefined();
    });
  });

  describe('integration with SessionHeader prop logic', () => {
    it('should provide correct prop value for SessionHeader when attached', () => {
      const sessionId = 'test-session';
      const workUnitId = 'STORY-001';

      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      // This simulates the logic in SplitSessionView that passes workUnitId to SessionHeader
      const sessionHeaderWorkUnitProp = store.getWorkUnitBySession(sessionId);

      expect(sessionHeaderWorkUnitProp).toBe(workUnitId);
    });

    it('should provide undefined prop value for SessionHeader when not attached', () => {
      const sessionId = 'unattached-session';

      const sessionHeaderWorkUnitProp = store.getWorkUnitBySession(sessionId);

      expect(sessionHeaderWorkUnitProp).toBeUndefined();
    });

    it('should update prop value when attachment changes during session lifecycle', () => {
      const sessionId = 'lifecycle-session';
      const workUnitA = 'STORY-001';
      const workUnitB = 'STORY-002';

      // Start unattached
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();

      // Attach to work unit A
      act(() => {
        store.attachSession(workUnitA, sessionId);
      });
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitA);

      // Move to work unit B
      act(() => {
        store.detachSession(workUnitA);
        store.attachSession(workUnitB, sessionId);
      });
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitB);

      // Detach completely
      act(() => {
        store.detachSession(workUnitB);
      });
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });
  });
});
