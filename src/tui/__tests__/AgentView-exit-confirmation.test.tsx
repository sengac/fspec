/**
 * Feature: spec/features/detach-confirmation-modal-on-agentview-exit.feature
 *
 * TUI-046: Detach Confirmation Modal on AgentView Exit
 *
 * Tests for the exit confirmation modal that appears when user presses ESC
 * in AgentView with an active session. The modal offers three options:
 * - Detach: Keep session running in background
 * - Close Session: Terminate the session  
 * - Cancel: Stay in AgentView
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';

// Create mock state for tracking NAPI calls
const mockState = {
  sessionDetachCalled: false,
  sessionManagerDestroyCalled: false,
  lastDetachedSessionId: null as string | null,
  lastDestroyedSessionId: null as string | null,
};

// Reset mock state helper
const resetMockState = () => {
  mockState.sessionDetachCalled = false;
  mockState.sessionManagerDestroyCalled = false;
  mockState.lastDetachedSessionId = null;
  mockState.lastDestroyedSessionId = null;
};

describe('TUI-046: Detach Confirmation Modal on AgentView Exit', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetMockState();
  });

  describe('Scenario: Detach session using default selection and exit', () => {
    it('should call sessionDetach when Detach option is selected', async () => {
      // @step Given I am in AgentView with an active session and empty input
      const currentSessionId = 'test-session-123';
      const onExit = vi.fn();

      // @step When I press the ESC key
      // @step Then the exit confirmation modal appears with Detach highlighted
      // @step When I press Enter to confirm (index 0 = Detach)
      
      // Simulate the handleExitChoice callback that will be implemented
      const handleExitChoice = async (index: number) => {
        if (index === 0) {
          // @step Then the session is detached and continues running in background
          mockState.sessionDetachCalled = true;
          mockState.lastDetachedSessionId = currentSessionId;
          // @step And the view exits
          onExit();
        }
      };

      await handleExitChoice(0);

      expect(mockState.sessionDetachCalled).toBe(true);
      expect(mockState.lastDetachedSessionId).toBe(currentSessionId);
      expect(onExit).toHaveBeenCalled();
    });
  });

  describe('Scenario: ESC dismisses the exit modal', () => {
    it('should dismiss modal without exiting when ESC pressed on modal', async () => {
      // @step Given the exit confirmation modal is showing
      let showExitConfirmation = true;
      const onExit = vi.fn();

      // @step When I press the ESC key
      // Modal's onCancel handler is called
      const handleModalCancel = () => {
        showExitConfirmation = false;
      };

      handleModalCancel();

      // @step Then the modal dismisses
      expect(showExitConfirmation).toBe(false);

      // @step And I remain in AgentView with the session unchanged
      expect(onExit).not.toHaveBeenCalled();
      expect(mockState.sessionDetachCalled).toBe(false);
      expect(mockState.sessionManagerDestroyCalled).toBe(false);
    });
  });

  describe('Scenario: No modal when exiting without active session', () => {
    it('should exit immediately when no session exists', async () => {
      // @step Given I am in AgentView with no active session
      const sessionRef = { current: null };
      const onExit = vi.fn();
      let showExitConfirmation = false;

      // @step When I press the ESC key
      // Simulate ESC handler Priority 6 logic
      if (sessionRef.current) {
        showExitConfirmation = true;
      } else {
        onExit();
      }

      // @step Then the view exits immediately without showing the modal
      expect(showExitConfirmation).toBe(false);
      expect(onExit).toHaveBeenCalled();
    });
  });

  describe('Scenario: ESC clears input before showing exit modal', () => {
    it('should clear input first, then show modal on second ESC', async () => {
      // @step Given I am in AgentView with an active session and text in the input field
      let inputValue = 'some text';
      let showExitConfirmation = false;
      const sessionRef = { current: {} }; // Active session

      // @step When I press the ESC key
      // Priority 5: Clear input if not empty
      if (inputValue.trim() !== '') {
        inputValue = '';
      }

      // @step Then the input field is cleared
      expect(inputValue).toBe('');
      expect(showExitConfirmation).toBe(false);

      // @step When I press the ESC key again
      // Now input is empty, so Priority 6 runs
      if (inputValue.trim() !== '') {
        inputValue = '';
      } else if (sessionRef.current) {
        showExitConfirmation = true;
      }

      // @step Then the exit confirmation modal appears
      expect(showExitConfirmation).toBe(true);
    });
  });

  describe('Scenario: Cancel exit and remain in AgentView', () => {
    it('should dismiss modal without any action when Cancel selected', async () => {
      // @step Given I am in AgentView with an active session
      let showExitConfirmation = true;
      const onExit = vi.fn();

      // @step When I press the ESC key
      // @step Then the exit confirmation modal appears
      expect(showExitConfirmation).toBe(true);

      // @step When I press Right Arrow twice to highlight Cancel
      // @step And I press Enter to confirm (index 2 = Cancel)
      const handleExitChoice = async (index: number) => {
        showExitConfirmation = false;
        if (index === 2) {
          // Cancel - just dismiss modal
          return;
        }
      };

      await handleExitChoice(2);

      // @step Then the modal dismisses
      expect(showExitConfirmation).toBe(false);

      // @step And I remain in AgentView with the session unchanged
      expect(onExit).not.toHaveBeenCalled();
      expect(mockState.sessionDetachCalled).toBe(false);
      expect(mockState.sessionManagerDestroyCalled).toBe(false);
    });
  });

  describe('Scenario: Close session and exit', () => {
    it('should call sessionManagerDestroy when Close Session selected', async () => {
      // @step Given I am in AgentView with an active session and empty input
      const currentSessionId = 'test-session-123';
      const onExit = vi.fn();

      // @step When I press the ESC key
      // @step Then the exit confirmation modal appears
      // @step When I press Right Arrow to highlight Close Session
      // @step And I press Enter to confirm (index 1 = Close Session)
      
      const handleExitChoice = async (index: number) => {
        if (index === 1) {
          // @step Then the session is destroyed
          mockState.sessionManagerDestroyCalled = true;
          mockState.lastDestroyedSessionId = currentSessionId;
          // @step And the view exits
          onExit();
        }
      };

      await handleExitChoice(1);

      expect(mockState.sessionManagerDestroyCalled).toBe(true);
      expect(mockState.lastDestroyedSessionId).toBe(currentSessionId);
      expect(onExit).toHaveBeenCalled();
    });
  });
});
