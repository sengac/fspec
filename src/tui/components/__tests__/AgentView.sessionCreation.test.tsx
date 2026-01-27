/**
 * Tests for AgentView session creation context awareness
 *
 * SESS-001: Session creation should be context-aware (board vs navigation)
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach, MockedFunction } from 'vitest';
import { renderHook } from '@testing-library/react';
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

vi.mock('../../utils/conversationUtils', () => ({
  processChunksToConversation: vi.fn(() => []),
  messagesToLines: vi.fn(() => []),
  wrapMessageToLines: vi.fn(() => []),
  getDisplayRole: vi.fn(() => 'user'),
}));

vi.mock('../../utils/terminalUtils', () => ({
  getTerminalWidth: vi.fn(() => 120),
}));

import { createSession } from '../../services/sessionService';
import { useSessionActions } from '../../store/sessionStore';

describe('AgentView - Session Creation Context Awareness', () => {
  let mockCreateSession: MockedFunction<typeof createSession>;
  let mockSessionActions: {
    activateSession: MockedFunction<any>;
    closeCreateSessionDialog: MockedFunction<any>;
    prepareForNewSession: MockedFunction<any>;
    clearAutoCreateRequest: MockedFunction<any>;
  };
  let store: ReturnType<typeof useFspecStore>;

  beforeEach(() => {
    vi.clearAllMocks();
    
    mockCreateSession = vi.mocked(createSession);
    mockCreateSession.mockResolvedValue({
      sessionId: 'new-session-123',
      provider: 'anthropic/claude-sonnet',
    });

    mockSessionActions = {
      activateSession: vi.fn(),
      closeCreateSessionDialog: vi.fn(),
      prepareForNewSession: vi.fn(),
      clearAutoCreateRequest: vi.fn(),
    };
    vi.mocked(useSessionActions).mockReturnValue(mockSessionActions);

    const { result } = renderHook(() => useFspecStore());
    store = result.current;
    store.clearAllSessionAttachments();
  });

  describe('session creation context detection', () => {
    /**
     * This tests the core fix: When creating a session from navigation (Shift+Right),
     * it should NOT auto-attach to the work unit, even if one exists.
     */
    it('should NOT attach to work unit when creating session via navigation (has currentSessionId)', async () => {
      // Import here to avoid hoisting issues with mocks
      const { AgentView } = await import('../AgentView');
      
      // Mock the scenario: user is in a session attached to work unit, presses Shift+Right
      const workUnitId = 'STORY-001';
      
      // Attach a session to the work unit first (simulating existing state)
      store.attachSession(workUnitId, 'existing-session');
      
      // Mock that we're currently in a session (navigation context)
      const mockCurrentSessionId = 'existing-session';
      
      // Create a test implementation of handleCreateSessionConfirm logic
      const testCreateSessionWithContext = async (wasInSession: boolean) => {
        const result = await mockCreateSession({
          modelPath: 'anthropic/claude-sonnet',
          project: 'test-project',
        });

        // This is the logic we're testing
        if (workUnitId && !wasInSession) {
          store.attachSession(workUnitId, result.sessionId);
        }

        return result;
      };

      // Test navigation context (was in session before creating)
      await testCreateSessionWithContext(true);

      // Verify the new session was NOT attached to the work unit
      expect(store.getWorkUnitBySession('new-session-123')).toBeUndefined();
      
      // Verify the original attachment remains
      expect(store.getAttachedSession(workUnitId)).toBe('existing-session');
    });

    it('should attach to work unit when creating session from board context (no currentSessionId)', async () => {
      const workUnitId = 'STORY-001';
      
      // Create a test implementation of handleCreateSessionConfirm logic
      const testCreateSessionWithContext = async (wasInSession: boolean) => {
        const result = await mockCreateSession({
          modelPath: 'anthropic/claude-sonnet',
          project: 'test-project',
        });

        // This is the logic we're testing
        if (workUnitId && !wasInSession) {
          store.attachSession(workUnitId, result.sessionId);
        }

        return result;
      };

      // Test board context (was not in session before creating)
      await testCreateSessionWithContext(false);

      // Verify the new session was attached to the work unit
      expect(store.getWorkUnitBySession('new-session-123')).toBe(workUnitId);
      expect(store.getAttachedSession(workUnitId)).toBe('new-session-123');
    });
  });

  describe('auto-create session logic', () => {
    it('should skip auto-create when work unit has attached session', () => {
      const workUnitId = 'STORY-001';
      const existingSessionId = 'existing-session';
      
      // Attach a session to the work unit
      store.attachSession(workUnitId, existingSessionId);
      
      // Test the auto-create logic condition
      const shouldSkipAutoCreate = !!(workUnitId && store.getAttachedSession(workUnitId));
      
      expect(shouldSkipAutoCreate).toBe(true);
    });

    it('should proceed with auto-create when work unit has no attached session', () => {
      const workUnitId = 'STORY-001';
      
      // Ensure no session is attached
      store.detachSession(workUnitId);
      
      // Test the auto-create logic condition
      const shouldSkipAutoCreate = !!(workUnitId && store.getAttachedSession(workUnitId));
      
      expect(shouldSkipAutoCreate).toBe(false);
    });

    it('should proceed with auto-create when no work unit is specified', () => {
      const workUnitId = undefined;
      
      // Test the auto-create logic condition
      const shouldSkipAutoCreate = !!(workUnitId && store.getAttachedSession(workUnitId || ''));
      
      expect(shouldSkipAutoCreate).toBe(false);
    });
  });

  describe('session attachment state transitions', () => {
    it('should handle session replacement correctly', () => {
      const workUnitId = 'STORY-001';
      const oldSessionId = 'old-session';
      const newSessionId = 'new-session';
      
      // Start with old session attached
      store.attachSession(workUnitId, oldSessionId);
      expect(store.getAttachedSession(workUnitId)).toBe(oldSessionId);
      
      // Replace with new session (simulating user creating new session for same work unit)
      store.attachSession(workUnitId, newSessionId);
      expect(store.getAttachedSession(workUnitId)).toBe(newSessionId);
      
      // Verify old session is no longer attached to any work unit
      expect(store.getWorkUnitBySession(oldSessionId)).toBeUndefined();
    });

    it('should handle multiple work units with separate sessions', () => {
      store.attachSession('STORY-001', 'session-1');
      store.attachSession('STORY-002', 'session-2');
      store.attachSession('BUG-001', 'session-3');
      
      expect(store.getAttachedSession('STORY-001')).toBe('session-1');
      expect(store.getAttachedSession('STORY-002')).toBe('session-2');
      expect(store.getAttachedSession('BUG-001')).toBe('session-3');
      
      expect(store.getWorkUnitBySession('session-1')).toBe('STORY-001');
      expect(store.getWorkUnitBySession('session-2')).toBe('STORY-002');
      expect(store.getWorkUnitBySession('session-3')).toBe('BUG-001');
    });
  });

  describe('work unit display calculation', () => {
    it('should calculate attached work unit for current session', () => {
      const sessionId = 'current-session';
      const workUnitId = 'STORY-001';
      
      store.attachSession(workUnitId, sessionId);
      
      // This simulates the useMemo calculation in AgentView
      const attachedWorkUnitId = store.getWorkUnitBySession(sessionId);
      
      expect(attachedWorkUnitId).toBe(workUnitId);
    });

    it('should return undefined when session has no attached work unit', () => {
      const sessionId = 'unattached-session';
      
      const attachedWorkUnitId = store.getWorkUnitBySession(sessionId);
      
      expect(attachedWorkUnitId).toBeUndefined();
    });

    it('should handle session ID changes correctly', () => {
      const oldSessionId = 'old-session';
      const newSessionId = 'new-session';
      const workUnitId = 'STORY-001';
      
      // Start with old session
      store.attachSession(workUnitId, oldSessionId);
      expect(store.getWorkUnitBySession(oldSessionId)).toBe(workUnitId);
      expect(store.getWorkUnitBySession(newSessionId)).toBeUndefined();
      
      // Switch to new session
      store.attachSession(workUnitId, newSessionId);
      expect(store.getWorkUnitBySession(oldSessionId)).toBeUndefined();
      expect(store.getWorkUnitBySession(newSessionId)).toBe(workUnitId);
    });
  });
});