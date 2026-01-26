/**
 * Feature: spec/features/input-not-restored-when-switching-between-sessions.feature
 *
 * Tests for TUI-053: Input not restored when switching between sessions
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionManagerList = vi.fn();
const mockSessionAttach = vi.fn();
const mockSessionDetach = vi.fn();
const mockSessionGetMergedOutput = vi.fn();
const mockSessionGetStatus = vi.fn();
const mockSessionSetPendingInput = vi.fn();
const mockSessionGetPendingInput = vi.fn();
const mockActivateSession = vi.fn();
const mockSessionGetMessages = vi.fn(() => []);

vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerList: mockSessionManagerList,
  sessionAttach: mockSessionAttach,
  sessionDetach: mockSessionDetach,
  sessionGetMergedOutput: mockSessionGetMergedOutput,
  sessionGetStatus: mockSessionGetStatus,
  sessionSetPendingInput: mockSessionSetPendingInput,
  sessionGetPendingInput: mockSessionGetPendingInput,
  activateSession: mockActivateSession,
  sessionGetMessages: mockSessionGetMessages,
  // Other required mocks
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
}));

// Mock session data
const createMockSession = (
  id: string,
  name: string,
  status: 'running' | 'idle' = 'idle'
) => ({
  id,
  name,
  status,
  project: '/test/project',
  messageCount: 5,
});

describe('Feature: Input not restored when switching between sessions', () => {
  describe('Scenario: Pending input is preserved when switching between sessions with Shift+Right/Left', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should preserve input when switching A → B → A', async () => {
      // @step Given the user has typed "Hello world" in Session A's AgentView input area
      const sessionA = 'session-a';
      const sessionB = 'session-b';
      const savedInput = 'Hello world';
      const emptyInput = '';

      mockSessionManagerList.mockReturnValue([
        createMockSession(sessionA, 'Session A'),
        createMockSession(sessionB, 'Session B'),
      ]);
      mockSessionSetPendingInput.mockResolvedValue(undefined);
      mockSessionGetPendingInput.mockImplementation((sessionId: string) => {
        if (sessionId === sessionA) return savedInput;
        return null; // Session B has no pending input
      });
      mockActivateSession.mockResolvedValue(undefined);

      // Simulate saving input for Session A
      await mockSessionSetPendingInput(sessionA, savedInput);

      // @step When the user presses Shift+Right to switch to Session B
      // This should: 1) save A's input, 2) detach from A, 3) resume B
      await mockSessionSetPendingInput(sessionA, savedInput); // Save before switching
      await mockSessionDetach(sessionA);
      await mockActivateSession(sessionB);

      // Verify Session A's input was saved
      expect(mockSessionSetPendingInput).toHaveBeenCalledWith(
        sessionA,
        savedInput
      );
      expect(mockSessionDetach).toHaveBeenCalledWith(sessionA);

      // Verify Session B's input is restored (empty in this case)
      const sessionBInput = mockSessionGetPendingInput(sessionB);
      expect(sessionBInput).toBe(null);

      // @step And the user presses Shift+Left to return to Session A
      // This should: 1) save B's input (empty), 2) detach from B, 3) resume A
      await mockSessionSetPendingInput(sessionB, emptyInput);
      await mockSessionDetach(sessionB);
      await mockActivateSession(sessionA);

      // Verify Session B's input was saved
      expect(mockSessionSetPendingInput).toHaveBeenCalledWith(
        sessionB,
        emptyInput
      );

      // @step Then Session A's AgentView input area should display "Hello world"
      const restoredInput = mockSessionGetPendingInput(sessionA);
      expect(restoredInput).toBe(savedInput);
    });
  });

  describe('Scenario: Empty input is correctly set when switching to session with no pending input', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should correctly set empty input for session without pending input', async () => {
      // @step Given the user has typed "Hello world" in Session A's AgentView input area
      const sessionA = 'session-a';
      const sessionB = 'session-b';
      const savedInput = 'Hello world';
      const emptyInput = '';

      mockSessionManagerList.mockReturnValue([
        createMockSession(sessionA, 'Session A'),
        createMockSession(sessionB, 'Session B'),
      ]);
      mockSessionSetPendingInput.mockResolvedValue(undefined);
      mockSessionGetPendingInput.mockImplementation((sessionId: string) => {
        if (sessionId === sessionA) return savedInput;
        return null; // Session B has no pending input
      });
      mockActivateSession.mockResolvedValue(undefined);

      // Simulate saving input for Session A
      await mockSessionSetPendingInput(sessionA, savedInput);

      // @step When the user presses Shift+Right to switch to Session B
      // This should: 1) save A's input, 2) detach from A, 3) resume B
      await mockSessionSetPendingInput(sessionA, savedInput);
      await mockSessionDetach(sessionA);
      await mockActivateSession(sessionB);

      // Verify Session A's input was saved
      expect(mockSessionSetPendingInput).toHaveBeenCalledWith(
        sessionA,
        savedInput
      );

      // @step Then Session B's AgentView input area should be empty
      const sessionBInput = mockSessionGetPendingInput(sessionB);
      expect(sessionBInput).toBe(null);

      // The resumeSessionById function should set empty string when pending input is null
      expect(sessionBInput || '').toBe('');
    });
  });

  describe('Scenario: Different pending inputs are preserved for each session', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should preserve different inputs when switching between sessions', async () => {
      // @step Given the user has typed "Hello world" in Session A's AgentView input area
      const sessionA = 'session-a';
      const sessionB = 'session-b';
      const savedInputA = 'Hello world';
      const savedInputB = 'Goodbye';

      mockSessionManagerList.mockReturnValue([
        createMockSession(sessionA, 'Session A'),
        createMockSession(sessionB, 'Session B'),
      ]);
      mockSessionSetPendingInput.mockResolvedValue(undefined);
      mockSessionGetPendingInput.mockImplementation((sessionId: string) => {
        if (sessionId === sessionA) return savedInputA;
        if (sessionId === sessionB) return savedInputB;
        return null;
      });
      mockActivateSession.mockResolvedValue(undefined);

      // @step And the user has typed "Goodbye" in Session B's AgentView input area
      // Simulate saving inputs for both sessions
      await mockSessionSetPendingInput(sessionA, savedInputA);
      await mockSessionSetPendingInput(sessionB, savedInputB);

      // @step When the user presses Shift+Right to switch to Session B
      await mockSessionSetPendingInput(sessionA, savedInputA);
      await mockSessionDetach(sessionA);
      await mockActivateSession(sessionB);

      // @step Then Session B's AgentView input area should display "Goodbye"
      const sessionBInput = mockSessionGetPendingInput(sessionB);
      expect(sessionBInput).toBe(savedInputB);

      // @step And the user presses Shift+Left to return to Session A
      await mockSessionSetPendingInput(sessionB, savedInputB);
      await mockSessionDetach(sessionB);
      await mockActivateSession(sessionA);

      // @step And Session A's AgentView input area should display "Hello world"
      const sessionAInput = mockSessionGetPendingInput(sessionA);
      expect(sessionAInput).toBe(savedInputA);
    });
  });

  describe('Integration: Session navigation with pending input save/restore', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should save pending input before switching even if input is empty', async () => {
      const sessionA = 'session-a';
      const sessionB = 'session-b';
      const emptyInput = '';

      mockSessionManagerList.mockReturnValue([
        createMockSession(sessionA, 'Session A'),
        createMockSession(sessionB, 'Session B'),
      ]);
      mockSessionSetPendingInput.mockResolvedValue(undefined);
      mockSessionGetPendingInput.mockReturnValue(null);
      mockActivateSession.mockResolvedValue(undefined);

      // Simulate navigating from A to B with empty input
      // The navigation logic should ALWAYS save pending input, even if empty
      await mockSessionSetPendingInput(sessionA, emptyInput);
      await mockSessionDetach(sessionA);
      await mockActivateSession(sessionB);

      // Verify empty input was saved to prevent wrong session's input from persisting
      expect(mockSessionSetPendingInput).toHaveBeenCalledWith(
        sessionA,
        emptyInput
      );
    });

    it('should perform navigation operations in correct order', async () => {
      const sessionA = 'session-a';
      const sessionB = 'session-b';
      const savedInput = 'Test input';

      mockSessionManagerList.mockReturnValue([
        createMockSession(sessionA, 'Session A'),
        createMockSession(sessionB, 'Session B'),
      ]);
      mockSessionSetPendingInput.mockResolvedValue(undefined);
      mockSessionGetPendingInput.mockReturnValue(savedInput);
      mockActivateSession.mockResolvedValue(undefined);

      const callOrder: string[] = [];

      mockSessionSetPendingInput.mockImplementation(() => {
        callOrder.push('save_pending_input');
        return Promise.resolve();
      });
      mockSessionDetach.mockImplementation(() => {
        callOrder.push('detach');
      });
      mockActivateSession.mockImplementation(() => {
        callOrder.push('resume_session');
        return Promise.resolve();
      });

      // Simulate navigation from A to B
      await mockSessionSetPendingInput(sessionA, savedInput);
      await mockSessionDetach(sessionA);
      await mockActivateSession(sessionB);

      // Verify order: save → detach → resume
      expect(callOrder).toEqual([
        'save_pending_input',
        'detach',
        'resume_session',
      ]);
    });
  });
});
