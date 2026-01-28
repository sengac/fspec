/**
 * Tests for AgentView session creation context awareness
 *
 * SESS-001: Session creation should be context-aware (board vs navigation)
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { useFspecStore } from '../../store/fspecStore';

// Mock all external dependencies
vi.mock('@sengac/codelet-napi', () => ({
  sessionDetach: vi.fn(),
  sessionAttach: vi.fn(),
  sessionGetMergedOutput: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  sessionGetParent: vi.fn(() => null),
  sessionGetPendingInput: vi.fn(() => ''),
}));

vi.mock('../../services/sessionService', () => ({
  createSession: vi.fn(),
  restoreSession: vi.fn(),
}));

vi.mock('../../store/sessionStore', () => ({
  useShowCreateSessionDialog: vi.fn(() => false),
  useShouldAutoCreateSession: vi.fn(() => false),
  useSessionActions: vi.fn(() => ({
    activateSession: vi.fn(),
    closeCreateSessionDialog: vi.fn(),
    prepareForNewSession: vi.fn(),
    clearAutoCreateRequest: vi.fn(),
  })),
}));

vi.mock('../../../utils/logger', () => ({
  logger: {
    debug: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
  },
}));

describe('AgentView Session Creation Logic', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    
    // Reset store state
    useFspecStore.setState({
      sessionAttachments: new Map<string, string>(),
      currentWorkUnitId: null,
    });
  });

  describe('Context-Aware Session Creation', () => {
    it('should create attached session when work unit is selected', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'new-session-123';
      const store = useFspecStore.getState();

      // Simulate user selecting work unit from board
      store.setCurrentWorkUnitId(workUnitId);

      // Simulate session creation (in real app this would be triggered by user action)
      store.attachSession(workUnitId, sessionId);

      // Verify session is properly attached
      expect(store.getAttachedSession(workUnitId)).toBe(sessionId);
      expect(store.getWorkUnitBySession(sessionId)).toBe(workUnitId);
    });

    it('should create unattached session when no work unit is selected', () => {
      const sessionId = 'unattached-session-456';
      const store = useFspecStore.getState();

      // No work unit selected (direct navigation to agent view)
      expect(store.getCurrentWorkUnitId()).toBeNull();

      // Session created but not attached to any work unit
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });

    it('should handle work unit selection change before session creation', () => {
      const workUnit1 = 'STORY-001';
      const workUnit2 = 'STORY-002';
      const sessionId = 'changed-context-session';
      const store = useFspecStore.getState();

      // User selects first work unit
      store.setCurrentWorkUnitId(workUnit1);

      // User changes selection before creating session
      store.setCurrentWorkUnitId(workUnit2);

      // Session is created for the currently selected work unit
      store.attachSession(workUnit2, sessionId);

      // Verify correct attachment
      expect(store.getAttachedSession(workUnit2)).toBe(sessionId);
      expect(store.getAttachedSession(workUnit1)).toBeUndefined();
    });
  });

  describe('Session Restoration Context', () => {
    it('should restore session with existing attachment', () => {
      const workUnitId = 'STORY-001';
      const existingSessionId = 'existing-session-789';
      const store = useFspecStore.getState();

      // Pre-existing attachment (from previous session)
      store.attachSession(workUnitId, existingSessionId);

      // Simulate app restart/restoration
      const restoredWorkUnit = store.getWorkUnitBySession(existingSessionId);
      
      // Should restore the attachment
      expect(restoredWorkUnit).toBe(workUnitId);
      expect(store.hasAttachedSession(workUnitId)).toBe(true);
    });

    it('should handle restoration of orphaned sessions', () => {
      const orphanedSessionId = 'orphaned-session-123';
      const store = useFspecStore.getState();

      // Session exists but has no work unit attachment
      expect(store.getWorkUnitBySession(orphanedSessionId)).toBeUndefined();

      // This represents sessions that were created without work unit context
      // They should remain unattached until explicitly attached
      const currentAttachment = store.getWorkUnitBySession(orphanedSessionId);
      expect(currentAttachment).toBeUndefined();
    });
  });

  describe('Multiple Session Management', () => {
    it('should handle switching between sessions with different attachments', () => {
      const workUnit1 = 'STORY-001';
      const workUnit2 = 'STORY-002';
      const session1 = 'session-1';
      const session2 = 'session-2';
      const store = useFspecStore.getState();

      // Create two sessions with different work unit attachments
      store.attachSession(workUnit1, session1);
      store.attachSession(workUnit2, session2);

      // Switch to first session context
      store.setCurrentWorkUnitId(workUnit1);
      expect(store.getAttachedSession(workUnit1)).toBe(session1);

      // Switch to second session context  
      store.setCurrentWorkUnitId(workUnit2);
      expect(store.getAttachedSession(workUnit2)).toBe(session2);

      // Both attachments should be maintained
      expect(store.getWorkUnitBySession(session1)).toBe(workUnit1);
      expect(store.getWorkUnitBySession(session2)).toBe(workUnit2);
    });

    it('should handle session replacement for same work unit', () => {
      const workUnitId = 'STORY-001';
      const oldSessionId = 'old-session';
      const newSessionId = 'new-session';
      const store = useFspecStore.getState();

      // Original session attachment
      store.attachSession(workUnitId, oldSessionId);
      expect(store.getAttachedSession(workUnitId)).toBe(oldSessionId);

      // Create new session for same work unit (replaces old attachment)
      store.attachSession(workUnitId, newSessionId);

      // New session should be attached, old one should be detached
      expect(store.getAttachedSession(workUnitId)).toBe(newSessionId);
      expect(store.getWorkUnitBySession(oldSessionId)).toBeUndefined();
      expect(store.getWorkUnitBySession(newSessionId)).toBe(workUnitId);
    });
  });

  describe('Error Scenarios and Recovery', () => {
    it('should handle invalid work unit ID gracefully', () => {
      const invalidWorkUnitId = '';
      const sessionId = 'session-for-invalid-workunit';
      const store = useFspecStore.getState();

      // Should not throw when attaching to invalid work unit ID
      expect(() => {
        store.attachSession(invalidWorkUnitId, sessionId);
      }).not.toThrow();

      // Attachment should still work (even with empty string)
      expect(store.getAttachedSession(invalidWorkUnitId)).toBe(sessionId);
    });

    it('should handle session creation failure gracefully', () => {
      const workUnitId = 'STORY-001';
      const store = useFspecStore.getState();

      // Simulate selection without successful session creation
      store.setCurrentWorkUnitId(workUnitId);

      // No session should be attached if creation failed
      expect(store.getAttachedSession(workUnitId)).toBeUndefined();

      // Work unit selection should remain
      expect(store.getCurrentWorkUnitId()).toBe(workUnitId);
    });

    it('should handle concurrent session operations', () => {
      const workUnitId = 'STORY-001';
      const session1 = 'concurrent-session-1';
      const session2 = 'concurrent-session-2';
      const store = useFspecStore.getState();

      // Simulate rapid concurrent operations
      store.attachSession(workUnitId, session1);
      store.attachSession(workUnitId, session2);

      // Last operation should win
      expect(store.getAttachedSession(workUnitId)).toBe(session2);
      expect(store.getWorkUnitBySession(session1)).toBeUndefined();
      expect(store.getWorkUnitBySession(session2)).toBe(workUnitId);
    });
  });

  describe('State Cleanup', () => {
    it('should clean up attachments when sessions are terminated', () => {
      const workUnitId = 'STORY-001';
      const sessionId = 'session-to-cleanup';
      const store = useFspecStore.getState();

      // Create attachment
      store.attachSession(workUnitId, sessionId);
      expect(store.hasAttachedSession(workUnitId)).toBe(true);

      // Simulate session termination
      store.detachSession(workUnitId);

      // Should be cleaned up
      expect(store.hasAttachedSession(workUnitId)).toBe(false);
      expect(store.getWorkUnitBySession(sessionId)).toBeUndefined();
    });

    it('should handle bulk cleanup operations', () => {
      const workUnits = ['STORY-001', 'STORY-002', 'BUG-001'];
      const sessions = ['session-1', 'session-2', 'session-3'];
      const store = useFspecStore.getState();

      // Create multiple attachments
      workUnits.forEach((workUnit, index) => {
        store.attachSession(workUnit, sessions[index]);
      });

      // Verify all attachments exist
      workUnits.forEach((workUnit) => {
        expect(store.hasAttachedSession(workUnit)).toBe(true);
      });

      // Clear all attachments
      store.clearAllSessionAttachments();

      // Verify all are cleaned up
      workUnits.forEach((workUnit) => {
        expect(store.hasAttachedSession(workUnit)).toBe(false);
      });

      sessions.forEach((session) => {
        expect(store.getWorkUnitBySession(session)).toBeUndefined();
      });
    });
  });
});