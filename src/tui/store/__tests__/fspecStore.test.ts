/**
 * Tests for fspecStore - Session attachment functionality
 *
 * SESS-001: Session-Work Unit Attachment System
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useFspecStore } from '../fspecStore';
import { renderHook, act } from '@testing-library/react';

describe('fspecStore - Session Attachments', () => {
  let store: ReturnType<typeof useFspecStore>;

  beforeEach(() => {
    const { result } = renderHook(() => useFspecStore());
    store = result.current;

    // Clear any existing attachments
    act(() => {
      store.clearAllSessionAttachments();
    });
  });

  describe('attachSession', () => {
    it('should attach session to work unit', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
      });

      expect(store.getAttachedSession('STORY-001')).toBe('session-123');
      expect(store.hasAttachedSession('STORY-001')).toBe(true);
    });

    it('should overwrite existing attachment for same work unit', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
        store.attachSession('STORY-001', 'session-456');
      });

      expect(store.getAttachedSession('STORY-001')).toBe('session-456');
    });

    it('should allow multiple work units with different sessions', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
        store.attachSession('STORY-002', 'session-456');
      });

      expect(store.getAttachedSession('STORY-001')).toBe('session-123');
      expect(store.getAttachedSession('STORY-002')).toBe('session-456');
    });
  });

  describe('detachSession', () => {
    it('should detach session from work unit', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
        store.detachSession('STORY-001');
      });

      expect(store.getAttachedSession('STORY-001')).toBeUndefined();
      expect(store.hasAttachedSession('STORY-001')).toBe(false);
    });

    it('should handle detaching non-existent attachment gracefully', () => {
      act(() => {
        store.detachSession('STORY-001');
      });

      expect(store.getAttachedSession('STORY-001')).toBeUndefined();
    });
  });

  describe('getWorkUnitBySession', () => {
    it('should return work unit ID for attached session', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
      });

      expect(store.getWorkUnitBySession('session-123')).toBe('STORY-001');
    });

    it('should return undefined for non-attached session', () => {
      expect(store.getWorkUnitBySession('session-nonexistent')).toBeUndefined();
    });

    it('should handle multiple attachments correctly', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
        store.attachSession('STORY-002', 'session-456');
      });

      expect(store.getWorkUnitBySession('session-123')).toBe('STORY-001');
      expect(store.getWorkUnitBySession('session-456')).toBe('STORY-002');
    });
  });

  describe('currentWorkUnitId tracking', () => {
    it('should set and get current work unit ID', () => {
      act(() => {
        store.setCurrentWorkUnitId('STORY-001');
      });

      expect(store.getCurrentWorkUnitId()).toBe('STORY-001');
    });

    it('should allow clearing current work unit ID', () => {
      act(() => {
        store.setCurrentWorkUnitId('STORY-001');
        store.setCurrentWorkUnitId(null);
      });

      expect(store.getCurrentWorkUnitId()).toBeNull();
    });
  });

  describe('clearAllSessionAttachments', () => {
    it('should clear all session attachments', () => {
      act(() => {
        store.attachSession('STORY-001', 'session-123');
        store.attachSession('STORY-002', 'session-456');
        store.clearAllSessionAttachments();
      });

      expect(store.getAttachedSession('STORY-001')).toBeUndefined();
      expect(store.getAttachedSession('STORY-002')).toBeUndefined();
      expect(store.hasAttachedSession('STORY-001')).toBe(false);
      expect(store.hasAttachedSession('STORY-002')).toBe(false);
    });
  });

  describe('session attachment state isolation', () => {
    it('should manage session attachments independently of work unit data', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-123';

      // Test that session attachments work independently
      act(() => {
        store.attachSession(workUnitId, sessionId);
      });

      expect(store.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);

      act(() => {
        store.detachSession(workUnitId);
      });

      expect(store.getAttachedSession(workUnitId)).toBeUndefined();
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();

      // Session attachments should be completely isolated state
      expect(store.sessionAttachments.size).toBe(0);
    });
  });
});
