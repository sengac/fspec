/**
 * Tests for fspecStore - Session attachment functionality
 *
 * SESS-001: Session-Work Unit Attachment System
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { useFspecStore } from '../fspecStore';

describe('fspecStore - Session Attachments', () => {
  beforeEach(() => {
    // Reset store state before each test
    useFspecStore.setState({
      sessionAttachments: new Map<string, string>(),
    });
  });

  describe('attachSession', () => {
    it('should attach session to work unit', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');

      expect(store.getAttachedSession('STORY-001')).toBe('session-123');
      expect(store.hasAttachedSession('STORY-001')).toBe(true);
    });

    it('should overwrite existing attachment for same work unit', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');
      store.attachSession('STORY-001', 'session-456');

      expect(store.getAttachedSession('STORY-001')).toBe('session-456');
    });

    it('should allow multiple work units with different sessions', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');
      store.attachSession('STORY-002', 'session-456');

      expect(store.getAttachedSession('STORY-001')).toBe('session-123');
      expect(store.getAttachedSession('STORY-002')).toBe('session-456');
    });
  });

  describe('detachSession', () => {
    it('should detach session from work unit', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');
      expect(store.hasAttachedSession('STORY-001')).toBe(true);

      store.detachSession('STORY-001');
      expect(store.hasAttachedSession('STORY-001')).toBe(false);
      expect(store.getAttachedSession('STORY-001')).toBeUndefined();
    });

    it('should handle detaching non-existent session gracefully', () => {
      const store = useFspecStore.getState();

      // Should not throw
      expect(() => {
        store.detachSession('non-existent-work-unit');
      }).not.toThrow();
    });

    it('should not affect other attachments when detaching one session', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');
      store.attachSession('STORY-002', 'session-456');

      store.detachSession('STORY-001');

      expect(store.hasAttachedSession('STORY-001')).toBe(false);
      expect(store.hasAttachedSession('STORY-002')).toBe(true);
      expect(store.getAttachedSession('STORY-002')).toBe('session-456');
    });
  });

  describe('getWorkUnitBySession', () => {
    it('should return work unit ID for attached session', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');

      expect(store.getWorkUnitBySession('session-123')).toBe('STORY-001');
    });

    it('should return undefined for unattached session', () => {
      const store = useFspecStore.getState();

      expect(store.getWorkUnitBySession('unattached-session')).toBeUndefined();
    });

    it('should handle session that was previously attached but then detached', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');
      expect(store.getWorkUnitBySession('session-123')).toBe('STORY-001');

      store.detachSession('STORY-001');
      expect(store.getWorkUnitBySession('session-123')).toBeUndefined();
    });
  });

  describe('clearAllSessionAttachments', () => {
    it('should remove all session attachments', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', 'session-123');
      store.attachSession('STORY-002', 'session-456');
      store.attachSession('STORY-003', 'session-789');

      expect(store.hasAttachedSession('STORY-001')).toBe(true);
      expect(store.hasAttachedSession('STORY-002')).toBe(true);
      expect(store.hasAttachedSession('STORY-003')).toBe(true);

      store.clearAllSessionAttachments();

      expect(store.hasAttachedSession('STORY-001')).toBe(false);
      expect(store.hasAttachedSession('STORY-002')).toBe(false);
      expect(store.hasAttachedSession('STORY-003')).toBe(false);
      expect(store.getWorkUnitBySession('session-123')).toBeUndefined();
      expect(store.getWorkUnitBySession('session-456')).toBeUndefined();
      expect(store.getWorkUnitBySession('session-789')).toBeUndefined();
    });

    it('should handle clearing empty attachments gracefully', () => {
      const store = useFspecStore.getState();

      expect(() => {
        store.clearAllSessionAttachments();
      }).not.toThrow();
    });
  });

  describe('edge cases', () => {
    it('should handle empty string session IDs', () => {
      const store = useFspecStore.getState();

      store.attachSession('STORY-001', '');
      expect(store.getAttachedSession('STORY-001')).toBe('');
      expect(store.getWorkUnitBySession('')).toBe('STORY-001');
    });

    it('should handle empty string work unit IDs', () => {
      const store = useFspecStore.getState();

      store.attachSession('', 'session-123');
      expect(store.getAttachedSession('')).toBe('session-123');
      expect(store.getWorkUnitBySession('session-123')).toBe('');
    });

    it('should handle special characters in IDs', () => {
      const store = useFspecStore.getState();
      const specialWorkUnit = 'STORY-001-PART-A';
      const specialSession = 'session-123-test_session';

      store.attachSession(specialWorkUnit, specialSession);
      expect(store.getAttachedSession(specialWorkUnit)).toBe(specialSession);
      expect(store.getWorkUnitBySession(specialSession)).toBe(specialWorkUnit);
    });
  });
});
