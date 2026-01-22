// Feature: spec/features/parent-command-for-quick-return.feature
// Tests for WATCH-014: /parent Command for Quick Return

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetParent: vi.fn(),
  sessionDetach: vi.fn(),
  sessionAttach: vi.fn(),
  sessionGetMergedOutput: vi.fn(),
}));

import {
  sessionGetParent,
  sessionDetach,
  sessionAttach,
  sessionGetMergedOutput,
} from '@sengac/codelet-napi';

// Mock conversation state
interface ConversationMessage {
  type: string;
  content: string;
}

// Mock StreamChunk type
interface MockStreamChunk {
  chunk_type: string;
  text?: string;
}

// Helper to simulate /parent command handling logic
function handleParentCommand(
  currentSessionId: string | null,
  getParent: (id: string) => string | null,
  detachSession: (id: string) => void,
  attachSession: (
    id: string,
    callback: (err: Error | null, chunk: MockStreamChunk) => void
  ) => void,
  getMergedOutput: (id: string) => MockStreamChunk[],
  setCurrentSessionId: (id: string) => void,
  setConversation: (messages: ConversationMessage[]) => void,
  getSessionName?: (id: string) => string | undefined
): { switchedTo: string | null; statusMessage: string } {
  // Check if no active session
  if (!currentSessionId) {
    return {
      switchedTo: null,
      statusMessage: 'No active session. Start a session first.',
    };
  }

  // Check if session is a watcher
  const parentId = getParent(currentSessionId);

  if (!parentId) {
    return {
      switchedTo: null,
      statusMessage:
        'This session has no parent. /parent only works from watcher sessions.',
    };
  }

  // Get parent session name for status message
  const parentName = getSessionName?.(parentId) || parentId;

  // Detach from current watcher session
  detachSession(currentSessionId);

  // Switch to parent session
  setCurrentSessionId(parentId);

  // Attach to parent session
  attachSession(parentId, () => {});

  // Get merged output and restore conversation
  const chunks = getMergedOutput(parentId);
  setConversation(chunks.map(c => ({ type: 'text', content: c.text || '' })));

  return {
    switchedTo: parentId,
    statusMessage: `Switched to parent session: ${parentName}`,
  };
}

describe('/parent Command for Quick Return', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Switch to parent session from watcher', () => {
    it('should switch from watcher to parent session when /parent is typed', () => {
      // @step Given a parent session named "Main Dev Session" exists
      const parentSessionId = 'main-dev-session';
      const parentSessionName = 'Main Dev Session';
      const parentChunks: MockStreamChunk[] = [
        { chunk_type: 'Text', text: 'Previous conversation from parent' },
      ];
      vi.mocked(sessionGetMergedOutput).mockReturnValue(parentChunks);

      // @step And a watcher session named "Security Reviewer" is attached to "Main Dev Session"
      const watcherSessionId = 'security-reviewer';
      vi.mocked(sessionGetParent).mockImplementation((id: string) => {
        if (id === watcherSessionId) {
          return parentSessionId;
        }
        return null;
      });

      // @step And the current session is "Security Reviewer"
      const currentSessionId = watcherSessionId;
      let switchedToSessionId: string | null = null;
      let conversation: ConversationMessage[] = [];

      // Helper to get session name (simulates sessionManagerList lookup)
      const getSessionName = (id: string) => {
        if (id === parentSessionId) return parentSessionName;
        return undefined;
      };

      // @step When the user types "/parent"
      const result = handleParentCommand(
        currentSessionId,
        vi.mocked(sessionGetParent),
        vi.mocked(sessionDetach),
        vi.mocked(sessionAttach) as any,
        vi.mocked(sessionGetMergedOutput),
        id => {
          switchedToSessionId = id;
        },
        msgs => {
          conversation = msgs;
        },
        getSessionName
      );

      // @step Then the current session switches to "Main Dev Session"
      expect(switchedToSessionId).toBe(parentSessionId);
      expect(result.switchedTo).toBe(parentSessionId);

      // @step And a status message shows "Switched to parent session"
      expect(result.statusMessage).toBe(
        'Switched to parent session: Main Dev Session'
      );

      // @step And the parent session conversation is displayed
      expect(sessionDetach).toHaveBeenCalledWith(watcherSessionId);
      expect(sessionAttach).toHaveBeenCalledWith(
        parentSessionId,
        expect.any(Function)
      );
      expect(sessionGetMergedOutput).toHaveBeenCalledWith(parentSessionId);
      expect(conversation).toHaveLength(1);
      expect(conversation[0].content).toBe('Previous conversation from parent');
    });
  });

  describe('Scenario: Error when using /parent in regular session', () => {
    it('should show error when /parent is typed in a non-watcher session', () => {
      // @step Given a regular session named "Code Project" exists
      const regularSessionId = 'code-project';

      // @step And the session is not a watcher session
      vi.mocked(sessionGetParent).mockReturnValue(null);

      // @step And the current session is "Code Project"
      const currentSessionId = regularSessionId;
      let switchedToSessionId: string | null = null;
      let conversation: ConversationMessage[] = [];

      // @step When the user types "/parent"
      const result = handleParentCommand(
        currentSessionId,
        vi.mocked(sessionGetParent),
        vi.mocked(sessionDetach),
        vi.mocked(sessionAttach) as any,
        vi.mocked(sessionGetMergedOutput),
        id => {
          switchedToSessionId = id;
        },
        msgs => {
          conversation = msgs;
        }
      );

      // @step Then a status message shows "This session has no parent. /parent only works from watcher sessions."
      expect(result.statusMessage).toBe(
        'This session has no parent. /parent only works from watcher sessions.'
      );

      // @step And the current session remains "Code Project"
      expect(switchedToSessionId).toBeNull();
      expect(result.switchedTo).toBeNull();
      expect(sessionDetach).not.toHaveBeenCalled();
      expect(sessionAttach).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Error when no active session exists', () => {
    it('should show error when /parent is typed with no active session', () => {
      // @step Given no session is currently active
      const currentSessionId = null;
      let switchedToSessionId: string | null = null;
      let conversation: ConversationMessage[] = [];

      // @step When the user types "/parent"
      const result = handleParentCommand(
        currentSessionId,
        vi.mocked(sessionGetParent),
        vi.mocked(sessionDetach),
        vi.mocked(sessionAttach) as any,
        vi.mocked(sessionGetMergedOutput),
        id => {
          switchedToSessionId = id;
        },
        msgs => {
          conversation = msgs;
        }
      );

      // @step Then a status message shows "No active session. Start a session first."
      expect(result.statusMessage).toBe(
        'No active session. Start a session first.'
      );
      expect(switchedToSessionId).toBeNull();
      expect(result.switchedTo).toBeNull();
      expect(sessionGetParent).not.toHaveBeenCalled();
      expect(sessionDetach).not.toHaveBeenCalled();
      expect(sessionAttach).not.toHaveBeenCalled();
    });
  });
});
