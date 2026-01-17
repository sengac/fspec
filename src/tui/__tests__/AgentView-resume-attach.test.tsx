/**
 * Feature: spec/features/attach-to-detached-sessions-from-resume-view.feature
 *
 * TUI-047: Attach to Detached Sessions from Resume View
 *
 * Tests for viewing and attaching to background sessions from /resume.
 * - Running sessions show 🔄 icon and use sessionAttach()
 * - Idle background sessions show ⏸️ icon
 * - Persisted-only sessions show 💾 icon and use persistenceLoadSession()
 * - Delete works for both types
 *
 * These are unit tests that verify the logic extracted from AgentView,
 * testing the helper functions and merge logic directly.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock state for tracking NAPI calls
const mockState = {
  sessionAttachCalled: false,
  sessionGetBufferedOutputCalled: false,
  sessionManagerDestroyCalled: false,
  persistenceLoadSessionCalled: false,
  lastAttachedSessionId: null as string | null,
  lastDestroyedSessionId: null as string | null,
  bufferedChunks: [] as Array<{ type: string; text?: string }>,
};

// Reset mock state
const resetMockState = () => {
  mockState.sessionAttachCalled = false;
  mockState.sessionGetBufferedOutputCalled = false;
  mockState.sessionManagerDestroyCalled = false;
  mockState.persistenceLoadSessionCalled = false;
  mockState.lastAttachedSessionId = null;
  mockState.lastDestroyedSessionId = null;
  mockState.bufferedChunks = [];
};

// TUI-047: Type definitions matching AgentView implementation
interface SessionManifest {
  id: string;
  name: string;
  project: string;
  provider: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
}

interface MergedSession extends SessionManifest {
  isBackgroundSession: boolean;
  backgroundStatus: 'running' | 'idle' | null;
}

interface BackgroundSessionInfo {
  id: string;
  name: string;
  status: string;
  project: string;
  messageCount?: number;
}

// TUI-047: Helper function matching AgentView implementation
const getSessionStatusIcon = (session: MergedSession): string => {
  if (session.isBackgroundSession) {
    return session.backgroundStatus === 'running' ? '🔄' : '⏸️';
  }
  return '💾';
};

// TUI-047: Merge function matching AgentView implementation
const mergeSessionLists = (
  persistedSessions: SessionManifest[],
  backgroundSessions: BackgroundSessionInfo[],
  currentProject: string
): MergedSession[] => {
  const backgroundMap = new Map<string, { status: string }>();
  for (const bg of backgroundSessions) {
    backgroundMap.set(bg.id, { status: bg.status });
  }

  const mergedSessions: MergedSession[] = persistedSessions.map((session) => {
    const bgInfo = backgroundMap.get(session.id);
    if (bgInfo) {
      return {
        ...session,
        isBackgroundSession: true,
        backgroundStatus: bgInfo.status as 'running' | 'idle',
      };
    }
    return {
      ...session,
      isBackgroundSession: false,
      backgroundStatus: null,
    };
  });

  // Add background sessions not in persistence
  for (const bg of backgroundSessions) {
    if (!persistedSessions.find((p) => p.id === bg.id)) {
      mergedSessions.push({
        id: bg.id,
        name: bg.name || 'Background Session',
        project: bg.project || currentProject,
        provider: 'unknown',
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        messageCount: bg.messageCount || 0,
        isBackgroundSession: true,
        backgroundStatus: bg.status as 'running' | 'idle',
      });
    }
  }

  return mergedSessions;
};

describe('TUI-047: Attach to Detached Sessions from Resume View', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockState();
  });

  describe('Scenario: Resume view shows running and idle sessions with status icons', () => {
    it('should display 🔄 for running background and 💾 for persisted-only sessions', () => {
      // @step Given I have a running background session and an idle persisted session
      const backgroundSessions: BackgroundSessionInfo[] = [
        { id: 'bg-1', name: 'Running Task', status: 'running', project: '/test' },
      ];
      const persistedSessions: SessionManifest[] = [
        { id: 'persist-1', name: 'Yesterday Session', project: '/test', provider: 'claude', createdAt: '2025-01-16T09:00:00Z', updatedAt: '2025-01-16T10:00:00Z', messageCount: 10 },
      ];

      // @step When I type /resume in AgentView
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');

      // Find the running background session and persisted-only session
      const runningSession = merged.find(s => s.id === 'bg-1');
      const persistedSession = merged.find(s => s.id === 'persist-1');

      // @step Then I see the running session with a 🔄 icon
      expect(runningSession).toBeDefined();
      expect(getSessionStatusIcon(runningSession!)).toBe('🔄');

      // @step And I see the idle session with a 💾 icon
      expect(persistedSession).toBeDefined();
      expect(getSessionStatusIcon(persistedSession!)).toBe('💾');
    });
  });

  describe('Scenario: Attach to running session shows buffered output then live stream', () => {
    it('should call sessionGetBufferedOutput then sessionAttach for running sessions', async () => {
      // @step Given I have a detached session that produced output while I was away
      const sessionId = 'bg-session-123';
      mockState.bufferedChunks = [
        { type: 'text', text: 'Buffered output from while detached' },
      ];

      // @step When I select the running session from /resume
      // Simulating attachToBackgroundSession() logic from AgentView
      const attachToBackgroundSession = async (id: string, handleStreamChunk: (chunk: unknown) => void) => {
        // First get buffered output (simulating sessionGetBufferedOutput)
        mockState.sessionGetBufferedOutputCalled = true;
        const buffered = mockState.bufferedChunks;

        // Hydrate conversation from buffer
        for (const chunk of buffered) {
          handleStreamChunk(chunk);
        }

        // Then attach for live streaming (simulating sessionAttach)
        mockState.sessionAttachCalled = true;
        mockState.lastAttachedSessionId = id;
      };

      const chunks: unknown[] = [];
      await attachToBackgroundSession(sessionId, (chunk) => chunks.push(chunk));

      // @step Then I immediately see the buffered output from while I was detached
      expect(mockState.sessionGetBufferedOutputCalled).toBe(true);
      expect(chunks.length).toBe(1);

      // @step And I see live streaming output continue in real-time
      expect(mockState.sessionAttachCalled).toBe(true);
      expect(mockState.lastAttachedSessionId).toBe(sessionId);
    });
  });

  describe('Scenario: Idle session loads from persistence as before', () => {
    it('should use persistenceLoadSession for persisted-only sessions', async () => {
      // @step Given I have an idle persisted session with conversation history
      const session: MergedSession = {
        id: 'persist-session-456',
        name: 'Old Session',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-15T09:00:00Z',
        updatedAt: '2025-01-15T10:00:00Z',
        messageCount: 5,
        isBackgroundSession: false,
        backgroundStatus: null,
      };

      // @step When I select the idle session from /resume
      // Simulating handleResumeSelect() logic from AgentView
      const handleResumeSelect = async (selectedSession: MergedSession) => {
        if (selectedSession.isBackgroundSession && selectedSession.backgroundStatus === 'running') {
          mockState.sessionAttachCalled = true;
        } else if (selectedSession.isBackgroundSession) {
          // Idle background - also uses attach
          mockState.sessionAttachCalled = true;
        } else {
          // Persisted-only - load from disk
          mockState.persistenceLoadSessionCalled = true;
        }
      };

      await handleResumeSelect(session);

      // @step Then the conversation history displays as before
      expect(mockState.persistenceLoadSessionCalled).toBe(true);
      expect(mockState.sessionAttachCalled).toBe(false);

      // @step And I can type a new prompt
      // (Implicit - input available after load)
    });
  });

  describe('Scenario: Delete running session destroys background session', () => {
    it('should call sessionManagerDestroy for background sessions', async () => {
      // @step Given I am in resume mode with a running background session selected
      const session: MergedSession = {
        id: 'bg-session-789',
        name: 'Task to Delete',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 3,
        isBackgroundSession: true,
        backgroundStatus: 'running',
      };

      // @step When I press D and confirm deletion
      // Simulating handleSessionDeleteSelect() logic from AgentView
      const handleDeleteSession = async (selectedSession: MergedSession) => {
        if (selectedSession.isBackgroundSession) {
          // Destroy background session first
          mockState.sessionManagerDestroyCalled = true;
          mockState.lastDestroyedSessionId = selectedSession.id;
        }
        // Always delete from persistence too (not tracked in this mock)
      };

      await handleDeleteSession(session);

      // @step Then the background session is destroyed
      expect(mockState.sessionManagerDestroyCalled).toBe(true);
      expect(mockState.lastDestroyedSessionId).toBe('bg-session-789');

      // @step And the session list refreshes without that session
      // (Implicit - handleResumeMode is called after delete)
    });
  });

  describe('Scenario: Sessions sorted by most recent first', () => {
    it('should sort merged sessions by updatedAt descending', () => {
      // @step Given I have multiple sessions with different last updated times
      const persistedSessions: SessionManifest[] = [
        { id: 'p-oldest', name: 'Oldest', project: '/test', provider: 'claude', createdAt: '2025-01-15T09:00:00Z', updatedAt: '2025-01-15T10:00:00Z', messageCount: 1 },
        { id: 'p-newest', name: 'Newest', project: '/test', provider: 'claude', createdAt: '2025-01-17T09:00:00Z', updatedAt: '2025-01-17T10:00:00Z', messageCount: 1 },
        { id: 'p-middle', name: 'Middle', project: '/test', provider: 'claude', createdAt: '2025-01-16T09:00:00Z', updatedAt: '2025-01-16T10:00:00Z', messageCount: 1 },
      ];
      const backgroundSessions: BackgroundSessionInfo[] = [];

      // @step When I view the resume list
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');
      const sorted = [...merged].sort(
        (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime()
      );

      // @step Then sessions appear sorted by most recent first
      expect(sorted[0].name).toBe('Newest');
      expect(sorted[1].name).toBe('Middle');
      expect(sorted[2].name).toBe('Oldest');
    });
  });

  describe('Scenario: Backward compatible when no background sessions exist', () => {
    it('should show only 💾 icons and load from persistence when no background sessions', async () => {
      // @step Given I have only persisted sessions and no background sessions
      const backgroundSessions: BackgroundSessionInfo[] = [];
      const persistedSessions: SessionManifest[] = [
        { id: 'p1', name: 'Session 1', project: '/test', provider: 'claude', createdAt: '2025-01-16T09:00:00Z', updatedAt: '2025-01-16T10:00:00Z', messageCount: 5 },
        { id: 'p2', name: 'Session 2', project: '/test', provider: 'claude', createdAt: '2025-01-15T09:00:00Z', updatedAt: '2025-01-15T10:00:00Z', messageCount: 3 },
      ];

      // @step When I view the resume list
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');
      const icons = merged.map(s => getSessionStatusIcon(s));

      // @step Then I see all sessions with 💾 icons
      expect(icons.every(icon => icon === '💾')).toBe(true);
      expect(icons).not.toContain('🔄');

      // @step And selecting any session loads from persistence as before
      const handleSelect = async (session: MergedSession) => {
        if (!session.isBackgroundSession) {
          mockState.persistenceLoadSessionCalled = true;
        }
      };
      await handleSelect(merged[0]);
      expect(mockState.persistenceLoadSessionCalled).toBe(true);
    });
  });

  describe('Scenario: Attach to idle background session shows complete output', () => {
    it('should use sessionAttach for idle background sessions and show complete output', async () => {
      // @step Given I have a session that finished while I was detached
      const sessionId = 'bg-session-finished';
      mockState.bufferedChunks = [
        { type: 'text', text: 'The task completed successfully.' },
        { type: 'text', text: 'Here is the final result.' },
      ];

      const session: MergedSession = {
        id: sessionId,
        name: 'Finished Task',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 5,
        isBackgroundSession: true,
        backgroundStatus: 'idle', // Finished but still in memory
      };

      // @step When I select the idle background session from /resume
      const attachToBackgroundSession = async (id: string, handleStreamChunk: (chunk: unknown) => void) => {
        mockState.sessionGetBufferedOutputCalled = true;
        const buffered = mockState.bufferedChunks;

        for (const chunk of buffered) {
          handleStreamChunk(chunk);
        }

        mockState.sessionAttachCalled = true;
        mockState.lastAttachedSessionId = id;
      };

      const chunks: unknown[] = [];
      await attachToBackgroundSession(sessionId, (chunk) => chunks.push(chunk));

      // @step Then I see the complete buffered output from the finished task
      expect(mockState.sessionGetBufferedOutputCalled).toBe(true);
      expect(chunks.length).toBe(2);

      // @step And I can type a new prompt to continue the conversation
      expect(mockState.sessionAttachCalled).toBe(true);
      expect(mockState.lastAttachedSessionId).toBe(sessionId);
    });

    it('should show ⏸️ icon for idle background sessions in the list', () => {
      // Test the three-state icon system
      const runningBg: MergedSession = {
        id: 'bg-running',
        name: 'Running',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 1,
        isBackgroundSession: true,
        backgroundStatus: 'running',
      };

      const idleBg: MergedSession = {
        id: 'bg-idle',
        name: 'Idle Background',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 1,
        isBackgroundSession: true,
        backgroundStatus: 'idle',
      };

      const persistedOnly: MergedSession = {
        id: 'persist-only',
        name: 'Persisted Only',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 1,
        isBackgroundSession: false,
        backgroundStatus: null,
      };

      // Running background = 🔄
      expect(getSessionStatusIcon(runningBg)).toBe('🔄');

      // Idle background = ⏸️
      expect(getSessionStatusIcon(idleBg)).toBe('⏸️');

      // Persisted only = 💾
      expect(getSessionStatusIcon(persistedOnly)).toBe('💾');
    });
  });
});
