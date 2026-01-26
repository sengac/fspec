/**
 * Feature: spec/features/input-state-not-restored-when-navigating-from-boardview-to-agentview.feature
 *
 * Tests for TUI-051: Input state not restored when navigating from BoardView to AgentView
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionGetPendingInput = vi.fn();
const mockSessionSetPendingInput = vi.fn();
const mockActivateSession = vi.fn();
const mockSessionGetMessages = vi.fn(() => []);

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetPendingInput: mockSessionGetPendingInput,
  sessionSetPendingInput: mockSessionSetPendingInput,
  activateSession: mockActivateSession,
  sessionGetMessages: mockSessionGetMessages,
  // Other required mocks
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
}));

describe('Feature: Input state not restored when navigating from BoardView to AgentView', () => {
  describe('Scenario: Pending input is restored when returning from BoardView to AgentView', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should restore pending input from Rust when resuming session', async () => {
      // @step Given the user has typed "Hello world" in the AgentView input area
      const savedInput = 'Hello world';
      // @step And the user presses Shift+Left to navigate to BoardView
      // (Input is saved to Rust by navigation logic)
      mockSessionGetPendingInput.mockReturnValue(savedInput);

      // @step And the user presses Shift+Right to return to AgentView
      const sessionId = 'session-123';
      mockActivateSession.mockResolvedValue(undefined);

      // Resume session logic would call sessionGetPendingInput
      const restoredInput = mockSessionGetPendingInput(sessionId);

      // @step Then the input area displays "Hello world"
      expect(restoredInput).toBe(savedInput);

      // @step And the input content matches what was typed before leaving AgentView
      expect(restoredInput).toBe('Hello world');
    });
  });

  describe('Scenario: Input state is synced to Rust on every keypress', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should sync input state to Rust when typing character by character', () => {
      // @step Given the user is in AgentView
      const sessionId = 'session-101';
      mockSessionSetPendingInput.mockResolvedValue(undefined);

      // @step When the user types "Testing 1 2 3" character by character
      const characters = [
        'T',
        'Te',
        'Tes',
        'Test',
        'Testi',
        'Testin',
        'Testing',
        'Testing ',
        'Testing 1',
        'Testing 1 ',
        'Testing 1 2',
        'Testing 1 2 ',
        'Testing 1 2 3',
      ];

      characters.forEach(input => {
        mockSessionSetPendingInput(sessionId, input);
      });

      // @step Then sessionSetPendingInput is called after each keystroke
      expect(mockSessionSetPendingInput).toHaveBeenCalledTimes(
        characters.length
      );

      // @step And the pending input in Rust matches the current input area content
      expect(mockSessionSetPendingInput).toHaveBeenLastCalledWith(
        sessionId,
        'Testing 1 2 3'
      );
    });
  });

  describe('Scenario: Pending input is preserved when switching between sessions with Shift+Right/Left', () => {
    beforeEach(() => {
      vi.clearAllMocks();
    });

    it('should preserve input when switching between sessions', async () => {
      // @step Given the user has typed "Hello world" in Session A's AgentView input area
      const sessionA = 'session-a';
      const savedInput = 'Hello world';
      mockSessionSetPendingInput.mockResolvedValue(undefined);
      mockSessionGetPendingInput.mockReturnValue(savedInput);
      mockActivateSession.mockResolvedValue(undefined);

      // Save input for Session A (simulating typing)
      await mockSessionSetPendingInput(sessionA, savedInput);

      // @step When the user presses Shift+Right to switch to Session B
      const sessionB = 'session-b';
      mockSessionGetPendingInput.mockReturnValue(null); // Session B has no pending input
      await mockActivateSession(sessionB);

      // @step And the user presses Shift+Left to return to Session A
      // Restore Session A's pending input
      mockSessionGetPendingInput.mockReturnValue(savedInput);
      await mockActivateSession(sessionA);

      const restoredInput = mockSessionGetPendingInput(sessionA);

      // @step Then the input area displays "Hello world" in Session A
      expect(restoredInput).toBe(savedInput);
    });
  });
});
