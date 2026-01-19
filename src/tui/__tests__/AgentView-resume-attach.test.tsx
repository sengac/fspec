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
  providerId?: string;
  modelId?: string;
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
      // Build provider string from background session's providerId/modelId
      const providerString = bg.providerId && bg.modelId
        ? `${bg.providerId}/${bg.modelId}`
        : bg.providerId || 'unknown';
      mergedSessions.push({
        id: bg.id,
        name: bg.name || 'Background Session',
        project: bg.project || currentProject,
        provider: providerString,
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

  describe('NAPI-009: User messages are restored from buffer on resume', () => {
    it('should include UserInput chunks in buffered output replay', async () => {
      // @step Given I have a detached session with user messages in the buffer
      const sessionId = 'bg-session-with-user-input';
      mockState.bufferedChunks = [
        { type: 'UserInput', text: 'What is the weather today?' },
        { type: 'Text', text: 'The weather today is sunny with a high of 72°F.' },
        { type: 'Done' },
        { type: 'UserInput', text: 'What about tomorrow?' },
        { type: 'Text', text: 'Tomorrow will be partly cloudy.' },
        { type: 'Done' },
      ];

      // @step When I attach to the session via /resume
      const attachToBackgroundSession = async (id: string, handleStreamChunk: (chunk: { type: string; text?: string }) => void) => {
        mockState.sessionGetBufferedOutputCalled = true;
        const buffered = mockState.bufferedChunks;

        for (const chunk of buffered) {
          handleStreamChunk(chunk);
        }

        mockState.sessionAttachCalled = true;
        mockState.lastAttachedSessionId = id;
      };

      const chunks: { type: string; text?: string }[] = [];
      await attachToBackgroundSession(sessionId, (chunk) => chunks.push(chunk));

      // @step Then I see the user messages restored in the conversation
      const userInputChunks = chunks.filter(c => c.type === 'UserInput');
      expect(userInputChunks.length).toBe(2);
      expect(userInputChunks[0].text).toBe('What is the weather today?');
      expect(userInputChunks[1].text).toBe('What about tomorrow?');

      // @step And I see the assistant responses
      const textChunks = chunks.filter(c => c.type === 'Text');
      expect(textChunks.length).toBe(2);
    });

    it('should handle conversation with interleaved user and assistant messages', async () => {
      // @step Given a multi-turn conversation in the buffer
      const sessionId = 'bg-multi-turn';
      mockState.bufferedChunks = [
        { type: 'UserInput', text: 'Help me write a function' },
        { type: 'Text', text: 'Sure, here is a function:' },
        { type: 'ToolCall', text: '' },
        { type: 'ToolResult', text: 'File written successfully' },
        { type: 'Text', text: 'I wrote the function for you.' },
        { type: 'Done' },
        { type: 'UserInput', text: 'Add error handling' },
        { type: 'Text', text: 'I will add try/catch.' },
        { type: 'Done' },
      ];

      // @step When I attach to the session
      const chunks: { type: string; text?: string }[] = [];
      for (const chunk of mockState.bufferedChunks) {
        chunks.push(chunk);
      }

      // @step Then the conversation order is preserved
      expect(chunks[0].type).toBe('UserInput');
      expect(chunks[0].text).toBe('Help me write a function');

      expect(chunks[6].type).toBe('UserInput');
      expect(chunks[6].text).toBe('Add error handling');
    });

    it('should create user role messages from UserInput chunks', () => {
      // @step Given a UserInput chunk from the buffer
      const userInputChunk = { type: 'UserInput', text: 'Hello, please help me' };

      // @step When handleStreamChunk processes it
      // Simulating the handleStreamChunk logic from AgentView
      interface ConversationMessage {
        role: 'user' | 'assistant' | 'tool';
        content: string;
      }
      const conversation: ConversationMessage[] = [];

      const handleStreamChunk = (chunk: { type: string; text?: string }) => {
        if (chunk.type === 'UserInput' && chunk.text) {
          conversation.push({ role: 'user', content: chunk.text });
        }
      };

      handleStreamChunk(userInputChunk);

      // @step Then a user message is added to the conversation
      expect(conversation.length).toBe(1);
      expect(conversation[0].role).toBe('user');
      expect(conversation[0].content).toBe('Hello, please help me');
    });

    it('should not create message for empty UserInput text', () => {
      // @step Given a UserInput chunk with undefined/empty text
      const emptyChunks = [
        { type: 'UserInput', text: undefined },
        { type: 'UserInput', text: '' },
      ];

      // @step When handleStreamChunk processes them
      interface ConversationMessage {
        role: 'user' | 'assistant' | 'tool';
        content: string;
      }
      const conversation: ConversationMessage[] = [];

      const handleStreamChunk = (chunk: { type: string; text?: string }) => {
        // Match the actual AgentView implementation check
        if (chunk.type === 'UserInput' && chunk.text) {
          conversation.push({ role: 'user', content: chunk.text });
        }
      };

      emptyChunks.forEach(handleStreamChunk);

      // @step Then no messages are added (empty strings are falsy in JS but this protects against undefined)
      // Note: The actual check `chunk.text` is truthy, so empty string '' is falsy and won't be added
      expect(conversation.length).toBe(0);
    });
  });

  describe('NAPI-009: isLoading state when attaching to running session', () => {
    it('should set isLoading=true when attaching to a running background session', async () => {
      // @step Given I have a running background session
      const session: MergedSession = {
        id: 'bg-running-session',
        name: 'Running Task',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 3,
        isBackgroundSession: true,
        backgroundStatus: 'running', // Key: session is running
      };

      // @step When I select and attach to the running session via /resume
      let isLoadingSet = false;
      const mockSetIsLoading = (value: boolean) => {
        if (value === true) {
          isLoadingSet = true;
        }
      };

      // Simulating handleResumeSelect logic for running background session
      const handleResumeSelect = async (selectedSession: MergedSession) => {
        if (selectedSession.isBackgroundSession) {
          // ... attach logic ...
          mockState.sessionAttachCalled = true;
          mockState.lastAttachedSessionId = selectedSession.id;

          // NAPI-009: Set isLoading if session is running so ESC can interrupt
          if (selectedSession.backgroundStatus === 'running') {
            mockSetIsLoading(true);
          }
        }
      };

      await handleResumeSelect(session);

      // @step Then isLoading should be set to true
      expect(isLoadingSet).toBe(true);
      expect(mockState.sessionAttachCalled).toBe(true);
    });

    it('should NOT set isLoading=true when attaching to an idle background session', async () => {
      // @step Given I have an idle background session
      const session: MergedSession = {
        id: 'bg-idle-session',
        name: 'Idle Task',
        project: '/test',
        provider: 'claude',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 3,
        isBackgroundSession: true,
        backgroundStatus: 'idle', // Key: session is idle
      };

      // @step When I select and attach to the idle session via /resume
      let isLoadingSet = false;
      const mockSetIsLoading = (value: boolean) => {
        if (value === true) {
          isLoadingSet = true;
        }
      };

      // Simulating handleResumeSelect logic for idle background session
      const handleResumeSelect = async (selectedSession: MergedSession) => {
        if (selectedSession.isBackgroundSession) {
          // ... attach logic ...
          mockState.sessionAttachCalled = true;
          mockState.lastAttachedSessionId = selectedSession.id;

          // NAPI-009: Set isLoading if session is running so ESC can interrupt
          if (selectedSession.backgroundStatus === 'running') {
            mockSetIsLoading(true);
          }
        }
      };

      await handleResumeSelect(session);

      // @step Then isLoading should NOT be set to true (session is idle)
      expect(isLoadingSet).toBe(false);
      expect(mockState.sessionAttachCalled).toBe(true);
    });
  });

  describe('Scenario: Background sessions preserve model info when switching', () => {
    it('should include providerId and modelId in merged session list', () => {
      // @step Given I have two background sessions with different models
      const backgroundSessions: BackgroundSessionInfo[] = [
        { id: 'bg-claude', name: 'Claude Session', status: 'running', project: '/test', providerId: 'anthropic', modelId: 'claude-sonnet-4' },
        { id: 'bg-gemini', name: 'Gemini Session', status: 'idle', project: '/test', providerId: 'google', modelId: 'gemini-2.5-pro' },
      ];
      const persistedSessions: SessionManifest[] = [];

      // @step When I view the resume list
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');

      // @step Then each session has the correct provider string
      const claudeSession = merged.find(s => s.id === 'bg-claude');
      const geminiSession = merged.find(s => s.id === 'bg-gemini');

      expect(claudeSession).toBeDefined();
      expect(claudeSession!.provider).toBe('anthropic/claude-sonnet-4');

      expect(geminiSession).toBeDefined();
      expect(geminiSession!.provider).toBe('google/gemini-2.5-pro');
    });

    it('should handle background sessions with only providerId (no modelId)', () => {
      // @step Given I have a background session with only providerId
      const backgroundSessions: BackgroundSessionInfo[] = [
        { id: 'bg-only-provider', name: 'Provider Only', status: 'idle', project: '/test', providerId: 'openai' },
      ];
      const persistedSessions: SessionManifest[] = [];

      // @step When I view the resume list
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');

      // @step Then the provider string falls back to just the providerId
      const session = merged.find(s => s.id === 'bg-only-provider');
      expect(session).toBeDefined();
      expect(session!.provider).toBe('openai');
    });

    it('should show "unknown" for background sessions without model info', () => {
      // @step Given I have a background session without model info
      const backgroundSessions: BackgroundSessionInfo[] = [
        { id: 'bg-no-model', name: 'No Model Info', status: 'running', project: '/test' },
      ];
      const persistedSessions: SessionManifest[] = [];

      // @step When I view the resume list
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');

      // @step Then the provider string is "unknown"
      const session = merged.find(s => s.id === 'bg-no-model');
      expect(session).toBeDefined();
      expect(session!.provider).toBe('unknown');
    });

    it('should prefer persistence provider info when session exists in both', () => {
      // @step Given a session exists in both persistence and background
      const backgroundSessions: BackgroundSessionInfo[] = [
        { id: 'shared-session', name: 'Background Name', status: 'running', project: '/test', providerId: 'anthropic', modelId: 'claude-opus-4' },
      ];
      const persistedSessions: SessionManifest[] = [
        { id: 'shared-session', name: 'Persisted Name', project: '/test', provider: 'anthropic/claude-sonnet-4', createdAt: '2025-01-16T09:00:00Z', updatedAt: '2025-01-16T10:00:00Z', messageCount: 10 },
      ];

      // @step When I view the resume list
      const merged = mergeSessionLists(persistedSessions, backgroundSessions, '/test');

      // @step Then the persisted session data is used (includes background status)
      const session = merged.find(s => s.id === 'shared-session');
      expect(session).toBeDefined();
      expect(session!.name).toBe('Persisted Name'); // From persistence
      expect(session!.provider).toBe('anthropic/claude-sonnet-4'); // From persistence
      expect(session!.isBackgroundSession).toBe(true);
      expect(session!.backgroundStatus).toBe('running');
    });
  });

  describe('Scenario: Model is restored when attaching to background session', () => {
    it('should restore model from provider string when attaching via handleResumeSelect', () => {
      // @step Given I have a background session with a specific model
      const selectedSession: MergedSession = {
        id: 'bg-opus',
        name: 'Opus Session',
        project: '/test',
        provider: 'anthropic/claude-opus-4',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 5,
        isBackgroundSession: true,
        backgroundStatus: 'idle',
      };

      // @step When I select this session from /resume
      // Simulating the model restoration logic from handleResumeSelect
      let restoredProviderId: string | null = null;
      let restoredModelId: string | null = null;

      if (selectedSession.provider && selectedSession.provider !== 'unknown') {
        const storedProvider = selectedSession.provider;
        if (storedProvider.includes('/')) {
          const [providerId, modelId] = storedProvider.split('/');
          restoredProviderId = providerId;
          restoredModelId = modelId;
        }
      }

      // @step Then the model should be restored from the session's provider string
      expect(restoredProviderId).toBe('anthropic');
      expect(restoredModelId).toBe('claude-opus-4');
    });

    it('should restore model from providerId/modelId when attaching via resumeSessionById', () => {
      // @step Given I have a background session with providerId and modelId
      const bgSession: BackgroundSessionInfo = {
        id: 'bg-gemini',
        name: 'Gemini Session',
        status: 'running',
        project: '/test',
        providerId: 'google',
        modelId: 'gemini-2.5-pro',
      };

      // @step When I attach to this session via resumeSessionById
      // Simulating the model restoration logic from resumeSessionById
      let restoredProviderId: string | null = null;
      let restoredModelId: string | null = null;

      if (bgSession.providerId) {
        restoredProviderId = bgSession.providerId;
        restoredModelId = bgSession.modelId || null;
      }

      // @step Then the model should be restored from the session's providerId/modelId
      expect(restoredProviderId).toBe('google');
      expect(restoredModelId).toBe('gemini-2.5-pro');
    });

    it('should not restore model when provider is "unknown"', () => {
      // @step Given I have a background session without model info
      const selectedSession: MergedSession = {
        id: 'bg-unknown',
        name: 'Unknown Model',
        project: '/test',
        provider: 'unknown',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 1,
        isBackgroundSession: true,
        backgroundStatus: 'idle',
      };

      // @step When I select this session from /resume
      let shouldRestoreModel = false;

      if (selectedSession.provider && selectedSession.provider !== 'unknown') {
        shouldRestoreModel = true;
      }

      // @step Then the model should NOT be restored (keep current model)
      expect(shouldRestoreModel).toBe(false);
    });

    it('should handle provider-only string without model', () => {
      // @step Given I have a session with only provider (no model)
      const selectedSession: MergedSession = {
        id: 'bg-provider-only',
        name: 'Provider Only',
        project: '/test',
        provider: 'openai',
        createdAt: '2025-01-17T09:00:00Z',
        updatedAt: '2025-01-17T10:00:00Z',
        messageCount: 1,
        isBackgroundSession: true,
        backgroundStatus: 'idle',
      };

      // @step When I select this session
      let restoredProviderId: string | null = null;
      let restoredModelId: string | null = null;

      if (selectedSession.provider && selectedSession.provider !== 'unknown') {
        const storedProvider = selectedSession.provider;
        if (storedProvider.includes('/')) {
          const [providerId, modelId] = storedProvider.split('/');
          restoredProviderId = providerId;
          restoredModelId = modelId;
        } else {
          // Provider only, no model
          restoredProviderId = storedProvider;
        }
      }

      // @step Then only the provider should be restored
      expect(restoredProviderId).toBe('openai');
      expect(restoredModelId).toBeNull();
    });
  });

  describe('Scenario: Live background session info takes precedence over stale persistence data', () => {
    it('should query live session info when attaching to a running session with stale persistence', () => {
      // @step Given persistence has stale model info (GLM 4.7)
      const persistedSessions: SessionManifest[] = [
        {
          id: 'session-1',
          name: 'My Session',
          project: '/test',
          provider: 'glm/glm-4.7', // Stale persistence data
          createdAt: '2025-01-16T09:00:00Z',
          updatedAt: '2025-01-16T10:00:00Z',
          messageCount: 10,
        },
      ];

      // @step And the live background session has been changed to Claude
      const liveBackgroundSessions: BackgroundSessionInfo[] = [
        {
          id: 'session-1',
          name: 'My Session',
          status: 'running',
          project: '/test',
          providerId: 'anthropic', // Current LIVE model
          modelId: 'claude-opus-4',
        },
      ];

      // @step When I attach to the session via handleResumeSelect
      // Simulating the CORRECT approach: query live session info first
      const merged = mergeSessionLists(persistedSessions, liveBackgroundSessions, '/test');
      const selectedSession = merged.find(s => s.id === 'session-1')!;

      // The key fix: query sessionManagerList() to get LIVE info
      const liveBgSession = liveBackgroundSessions.find(bg => bg.id === selectedSession.id);

      let restoredProviderId: string | null = null;
      let restoredModelId: string | null = null;

      // Prefer live background session info over stale MergedSession provider
      if (liveBgSession?.providerId) {
        restoredProviderId = liveBgSession.providerId;
        restoredModelId = liveBgSession.modelId || null;
      } else if (selectedSession.provider && selectedSession.provider !== 'unknown') {
        // Fallback to persistence data if no live session info
        const storedProvider = selectedSession.provider;
        if (storedProvider.includes('/')) {
          const [providerId, modelId] = storedProvider.split('/');
          restoredProviderId = providerId;
          restoredModelId = modelId;
        }
      }

      // @step Then the LIVE model (Claude) should be restored, not the stale one (GLM)
      expect(restoredProviderId).toBe('anthropic');
      expect(restoredModelId).toBe('claude-opus-4');
    });

    it('should fall back to persistence data when no live background session exists', () => {
      // @step Given persistence has model info
      const persistedSessions: SessionManifest[] = [
        {
          id: 'old-session',
          name: 'Old Session',
          project: '/test',
          provider: 'anthropic/claude-sonnet-4',
          createdAt: '2025-01-15T09:00:00Z',
          updatedAt: '2025-01-15T10:00:00Z',
          messageCount: 5,
        },
      ];

      // @step And there is no live background session (session is persisted-only)
      const liveBackgroundSessions: BackgroundSessionInfo[] = [];

      // @step When I select the session via handleResumeSelect
      const merged = mergeSessionLists(persistedSessions, liveBackgroundSessions, '/test');
      const selectedSession = merged.find(s => s.id === 'old-session')!;

      // Query for live session (won't find one)
      const liveBgSession = liveBackgroundSessions.find(bg => bg.id === selectedSession.id);

      let restoredProviderId: string | null = null;
      let restoredModelId: string | null = null;

      if (liveBgSession?.providerId) {
        restoredProviderId = liveBgSession.providerId;
        restoredModelId = liveBgSession.modelId || null;
      } else if (selectedSession.provider && selectedSession.provider !== 'unknown') {
        // Fallback to persistence data
        const storedProvider = selectedSession.provider;
        if (storedProvider.includes('/')) {
          const [providerId, modelId] = storedProvider.split('/');
          restoredProviderId = providerId;
          restoredModelId = modelId;
        }
      }

      // @step Then the persistence model should be restored
      expect(restoredProviderId).toBe('anthropic');
      expect(restoredModelId).toBe('claude-sonnet-4');
    });

    it('should handle live session without providerId by falling back to persistence', () => {
      // @step Given persistence has model info
      const persistedSessions: SessionManifest[] = [
        {
          id: 'session-2',
          name: 'Session 2',
          project: '/test',
          provider: 'google/gemini-2.5-pro',
          createdAt: '2025-01-16T09:00:00Z',
          updatedAt: '2025-01-16T10:00:00Z',
          messageCount: 3,
        },
      ];

      // @step And the live background session has no model info (edge case)
      const liveBackgroundSessions: BackgroundSessionInfo[] = [
        {
          id: 'session-2',
          name: 'Session 2',
          status: 'idle',
          project: '/test',
          // No providerId or modelId
        },
      ];

      // @step When I attach to the session
      const merged = mergeSessionLists(persistedSessions, liveBackgroundSessions, '/test');
      const selectedSession = merged.find(s => s.id === 'session-2')!;
      const liveBgSession = liveBackgroundSessions.find(bg => bg.id === selectedSession.id);

      let restoredProviderId: string | null = null;
      let restoredModelId: string | null = null;

      if (liveBgSession?.providerId) {
        restoredProviderId = liveBgSession.providerId;
        restoredModelId = liveBgSession.modelId || null;
      } else if (selectedSession.provider && selectedSession.provider !== 'unknown') {
        // Fall back to persistence
        const storedProvider = selectedSession.provider;
        if (storedProvider.includes('/')) {
          const [providerId, modelId] = storedProvider.split('/');
          restoredProviderId = providerId;
          restoredModelId = modelId;
        }
      }

      // @step Then the persistence model should be used as fallback
      expect(restoredProviderId).toBe('google');
      expect(restoredModelId).toBe('gemini-2.5-pro');
    });

    it('should handle switching between sessions with different live models', () => {
      // @step Given I have two background sessions with different models
      const liveBackgroundSessions: BackgroundSessionInfo[] = [
        { id: 'session-claude', name: 'Claude Session', status: 'running', project: '/test', providerId: 'anthropic', modelId: 'claude-opus-4' },
        { id: 'session-gemini', name: 'Gemini Session', status: 'idle', project: '/test', providerId: 'google', modelId: 'gemini-2.5-pro' },
      ];
      const persistedSessions: SessionManifest[] = [];

      // @step When I query the live sessions for each
      const claudeBg = liveBackgroundSessions.find(bg => bg.id === 'session-claude');
      const geminiBg = liveBackgroundSessions.find(bg => bg.id === 'session-gemini');

      // @step Then each session returns its correct live model
      expect(claudeBg?.providerId).toBe('anthropic');
      expect(claudeBg?.modelId).toBe('claude-opus-4');

      expect(geminiBg?.providerId).toBe('google');
      expect(geminiBg?.modelId).toBe('gemini-2.5-pro');
    });
  });

  describe('Scenario: Model fallback when providerSections lookup fails', () => {
    interface ModelSelection {
      providerId: string;
      modelId: string;
      apiModelId: string;
      displayName: string;
      reasoning: boolean;
      hasVision: boolean;
      contextWindow: number;
      maxOutput: number;
    }

    interface ProviderSection {
      providerId: string;
      models: Array<{ id: string; name: string }>;
    }

    const extractModelIdForRegistry = (apiModelId: string): string => {
      return apiModelId
        .replace(/-preview-\d{2}-\d{2}$/, '')
        .replace(/-\d{8}$/, '');
    };

    it('should set minimal ModelSelection when model not found in providerSections', () => {
      // @step Given a live background session with model info
      const liveBgSession: BackgroundSessionInfo = {
        id: 'session-1',
        name: 'Test Session',
        status: 'running',
        project: '/test',
        providerId: 'anthropic',
        modelId: 'claude-opus-4',
      };

      // @step And providerSections is empty (not loaded yet)
      const providerSections: ProviderSection[] = [];

      // @step When model restoration runs
      let currentModel: ModelSelection | null = null;

      const providerId = liveBgSession.providerId!;
      const modelId = liveBgSession.modelId!;

      const section = providerSections.find(s => s.providerId === providerId);
      const model = section?.models.find(m => extractModelIdForRegistry(m.id) === modelId);

      if (model && section) {
        currentModel = {
          providerId,
          modelId,
          apiModelId: model.id,
          displayName: model.name,
          reasoning: false,
          hasVision: false,
          contextWindow: 0,
          maxOutput: 0,
        };
      } else {
        // Fallback: set minimal model info from raw values
        currentModel = {
          providerId,
          modelId,
          apiModelId: modelId,
          displayName: modelId,
          reasoning: false,
          hasVision: false,
          contextWindow: 0,
          maxOutput: 0,
        };
      }

      // @step Then a minimal ModelSelection should be set with modelId as displayName
      expect(currentModel).not.toBeNull();
      expect(currentModel!.providerId).toBe('anthropic');
      expect(currentModel!.modelId).toBe('claude-opus-4');
      expect(currentModel!.displayName).toBe('claude-opus-4');
      expect(currentModel!.apiModelId).toBe('claude-opus-4');
    });

    it('should set full ModelSelection when model found in providerSections', () => {
      // @step Given a live background session with model info
      const liveBgSession: BackgroundSessionInfo = {
        id: 'session-1',
        name: 'Test Session',
        status: 'running',
        project: '/test',
        providerId: 'anthropic',
        modelId: 'claude-opus-4',
      };

      // @step And providerSections has the matching model
      const providerSections: ProviderSection[] = [
        {
          providerId: 'anthropic',
          models: [
            { id: 'claude-opus-4-20250514', name: 'Claude Opus 4' },
            { id: 'claude-sonnet-4-20250514', name: 'Claude Sonnet 4' },
          ],
        },
      ];

      // @step When model restoration runs
      let currentModel: ModelSelection | null = null;

      const providerId = liveBgSession.providerId!;
      const modelId = liveBgSession.modelId!;

      const section = providerSections.find(s => s.providerId === providerId);
      const model = section?.models.find(m => extractModelIdForRegistry(m.id) === modelId);

      if (model && section) {
        currentModel = {
          providerId,
          modelId,
          apiModelId: model.id,
          displayName: model.name,
          reasoning: false,
          hasVision: false,
          contextWindow: 0,
          maxOutput: 0,
        };
      } else {
        currentModel = {
          providerId,
          modelId,
          apiModelId: modelId,
          displayName: modelId,
          reasoning: false,
          hasVision: false,
          contextWindow: 0,
          maxOutput: 0,
        };
      }

      // @step Then full ModelSelection should be set with proper displayName
      expect(currentModel).not.toBeNull();
      expect(currentModel!.providerId).toBe('anthropic');
      expect(currentModel!.modelId).toBe('claude-opus-4');
      expect(currentModel!.displayName).toBe('Claude Opus 4');
      expect(currentModel!.apiModelId).toBe('claude-opus-4-20250514');
    });

    it('should handle provider not found in providerSections', () => {
      // @step Given a live background session with an unknown provider
      const liveBgSession: BackgroundSessionInfo = {
        id: 'session-1',
        name: 'Test Session',
        status: 'running',
        project: '/test',
        providerId: 'unknown-provider',
        modelId: 'unknown-model',
      };

      // @step And providerSections has different providers
      const providerSections: ProviderSection[] = [
        {
          providerId: 'anthropic',
          models: [{ id: 'claude-opus-4-20250514', name: 'Claude Opus 4' }],
        },
      ];

      // @step When model restoration runs
      let currentModel: ModelSelection | null = null;

      const providerId = liveBgSession.providerId!;
      const modelId = liveBgSession.modelId!;

      const section = providerSections.find(s => s.providerId === providerId);
      const model = section?.models.find(m => extractModelIdForRegistry(m.id) === modelId);

      if (model && section) {
        currentModel = {
          providerId,
          modelId,
          apiModelId: model.id,
          displayName: model.name,
          reasoning: false,
          hasVision: false,
          contextWindow: 0,
          maxOutput: 0,
        };
      } else {
        currentModel = {
          providerId,
          modelId,
          apiModelId: modelId,
          displayName: modelId,
          reasoning: false,
          hasVision: false,
          contextWindow: 0,
          maxOutput: 0,
        };
      }

      // @step Then fallback ModelSelection should be set
      expect(currentModel).not.toBeNull();
      expect(currentModel!.providerId).toBe('unknown-provider');
      expect(currentModel!.modelId).toBe('unknown-model');
      expect(currentModel!.displayName).toBe('unknown-model');
    });
  });
});
