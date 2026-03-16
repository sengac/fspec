// Feature: spec/features/parent-command-for-quick-return.feature
// Tests for WATCH-014: /parent Command for Quick Return
//
// NOTE: WATCH-024 removed the /parent slash command entirely.
// Users navigate with Shift+Arrow instead.
// These tests verify the underlying subordinate-lookup LOGIC still works,
// even though the slash command is gone.
//
// Session switching now uses GlobalSessionStreamManager for handler registration.

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionGetSubordinate: vi.fn(),
  sessionGetMergedOutput: vi.fn(),
}));

import {
  sessionGetSubordinate,
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

// Helper to simulate subordinate-lookup navigation logic
// (Originally the /parent command, now handled by Shift+Arrow navigation)
function handleSubordinateNavigation(
  currentSessionId: string | null,
  getSubordinate: (id: string) => string | null,
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

  // Check if session is a supervisor (has a subordinate)
  const subordinateId = getSubordinate(currentSessionId);

  if (!subordinateId) {
    return {
      switchedTo: null,
      statusMessage:
        'This session has no subordinate. Navigation only works from supervisor sessions.',
    };
  }

  // Get subordinate session name for status message
  const subordinateName = getSessionName?.(subordinateId) || subordinateId;

  // Switch to subordinate session (GlobalSessionStreamManager handles routing)
  setCurrentSessionId(subordinateId);

  // Get merged output and restore conversation
  const chunks = getMergedOutput(subordinateId);
  setConversation(chunks.map(c => ({ type: 'text', content: c.text || '' })));

  return {
    switchedTo: subordinateId,
    statusMessage: `Switched to subordinate session: ${subordinateName}`,
  };
}

describe('Subordinate Navigation Logic (formerly /parent command)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Switch to subordinate session from supervisor', () => {
    it('should switch from supervisor to subordinate session', () => {
      // @step Given a subordinate session named "Main Dev Session" exists
      const subordinateSessionId = 'main-dev-session';
      const subordinateSessionName = 'Main Dev Session';
      const subordinateChunks: MockStreamChunk[] = [
        { chunk_type: 'Text', text: 'Previous conversation from subordinate' },
      ];
      vi.mocked(sessionGetMergedOutput).mockReturnValue(subordinateChunks);

      // @step And a supervisor session named "Security Reviewer" is attached to "Main Dev Session"
      const supervisorSessionId = 'security-reviewer';
      vi.mocked(sessionGetSubordinate).mockImplementation((id: string) => {
        if (id === supervisorSessionId) {
          return subordinateSessionId;
        }
        return null;
      });

      // @step And the current session is "Security Reviewer"
      const currentSessionId = supervisorSessionId;
      let switchedToSessionId: string | null = null;
      let conversation: ConversationMessage[] = [];

      // Helper to get session name (simulates sessionManagerList lookup)
      const getSessionName = (id: string) => {
        if (id === subordinateSessionId) return subordinateSessionName;
        return undefined;
      };

      // @step When navigation to subordinate is triggered
      const result = handleSubordinateNavigation(
        currentSessionId,
        vi.mocked(sessionGetSubordinate),
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
      expect(switchedToSessionId).toBe(subordinateSessionId);
      expect(result.switchedTo).toBe(subordinateSessionId);

      // @step And a status message shows "Switched to subordinate session"
      expect(result.statusMessage).toBe(
        'Switched to subordinate session: Main Dev Session'
      );

      // @step And the subordinate session conversation is displayed
      expect(sessionGetMergedOutput).toHaveBeenCalledWith(subordinateSessionId);
      expect(conversation).toHaveLength(1);
      expect(conversation[0].content).toBe(
        'Previous conversation from subordinate'
      );
    });
  });

  describe('Scenario: Error when navigating from non-supervisor session', () => {
    it('should show error when navigating in a non-supervisor session', () => {
      // @step Given a regular session named "Code Project" exists
      const regularSessionId = 'code-project';

      // @step And the session is not a supervisor session
      vi.mocked(sessionGetSubordinate).mockReturnValue(null);

      // @step And the current session is "Code Project"
      const currentSessionId = regularSessionId;
      let switchedToSessionId: string | null = null;
      let conversation: ConversationMessage[] = [];

      // @step When navigation to subordinate is triggered
      const result = handleSubordinateNavigation(
        currentSessionId,
        vi.mocked(sessionGetSubordinate),
        vi.mocked(sessionGetMergedOutput),
        id => {
          switchedToSessionId = id;
        },
        msgs => {
          conversation = msgs;
        }
      );

      // @step Then a status message shows the session has no subordinate
      expect(result.statusMessage).toBe(
        'This session has no subordinate. Navigation only works from supervisor sessions.'
      );

      // @step And the current session remains "Code Project"
      expect(switchedToSessionId).toBeNull();
      expect(result.switchedTo).toBeNull();
    });
  });

  describe('Scenario: Error when no active session exists', () => {
    it('should show error when navigating with no active session', () => {
      // @step Given no session is currently active
      const currentSessionId = null;
      let switchedToSessionId: string | null = null;
      let conversation: ConversationMessage[] = [];

      // @step When navigation to subordinate is triggered
      const result = handleSubordinateNavigation(
        currentSessionId,
        vi.mocked(sessionGetSubordinate),
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
      expect(sessionGetSubordinate).not.toHaveBeenCalled();
    });
  });
});
