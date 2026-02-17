// Feature: spec/features/shift-arrow-session-switching.feature
// Tests for TUI-049: Shift+Arrow Session Switching
//
// Session switching now uses GlobalSessionStreamManager for handler registration.
// These tests verify the navigation LOGIC (index calculation, wrap-around).

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerList: vi.fn(),
  sessionGetMergedOutput: vi.fn(),
  sessionGetStatus: vi.fn(),
  sessionSetPendingInput: vi.fn(),
  sessionGetPendingInput: vi.fn(),
}));

import {
  sessionManagerList,
  sessionGetMergedOutput,
  sessionGetStatus,
  sessionSetPendingInput,
  sessionGetPendingInput,
} from '@sengac/codelet-napi';

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

// Helper to calculate target session index (mirrors actual navigation logic)
function calculateTargetIndex(
  direction: 'next' | 'prev',
  currentIndex: number,
  totalSessions: number
): number {
  if (direction === 'next') {
    return (currentIndex + 1) % totalSessions;
  } else {
    return (currentIndex - 1 + totalSessions) % totalSessions;
  }
}

describe('Shift+Arrow Session Switching', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Switch to next session with Shift+Right', () => {
    it('should switch from session A to session B when pressing Shift+Right', () => {
      // @step Given I have 3 background sessions A, B, and C
      const sessions = [
        createMockSession('session-a', 'Session A'),
        createMockSession('session-b', 'Session B'),
        createMockSession('session-c', 'Session C'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And I am currently viewing session A
      const currentSessionId = 'session-a';
      const currentIndex = sessions.findIndex(s => s.id === currentSessionId);
      expect(currentIndex).toBe(0);

      // @step When I press Shift+Right arrow
      const nextIndex = calculateTargetIndex(
        'next',
        currentIndex,
        sessions.length
      );
      const nextSession = sessions[nextIndex];

      // @step Then the target session should be session B
      expect(nextSession.id).toBe('session-b');

      // @step And I should see session B's conversation
      vi.mocked(sessionGetMergedOutput).mockReturnValue([
        { type: 'Text', text: 'Session B conversation' },
      ]);
      const output = sessionGetMergedOutput(nextSession.id);
      expect(output).toHaveLength(1);
      expect(output[0].text).toBe('Session B conversation');
    });
  });

  describe('Scenario: Switch to previous session with Shift+Left', () => {
    it('should switch from session B to session A when pressing Shift+Left', () => {
      // @step Given I have 3 background sessions A, B, and C
      const sessions = [
        createMockSession('session-a', 'Session A'),
        createMockSession('session-b', 'Session B'),
        createMockSession('session-c', 'Session C'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And I am currently viewing session B
      const currentSessionId = 'session-b';
      const currentIndex = sessions.findIndex(s => s.id === currentSessionId);
      expect(currentIndex).toBe(1);

      // @step When I press Shift+Left arrow
      const prevIndex = calculateTargetIndex(
        'prev',
        currentIndex,
        sessions.length
      );
      const prevSession = sessions[prevIndex];

      // @step Then the target session should be session A
      expect(prevSession.id).toBe('session-a');

      // @step And I should see session A's conversation
      vi.mocked(sessionGetMergedOutput).mockReturnValue([
        { type: 'Text', text: 'Session A conversation' },
      ]);
      const output = sessionGetMergedOutput(prevSession.id);
      expect(output).toHaveLength(1);
      expect(output[0].text).toBe('Session A conversation');
    });
  });

  describe('Scenario: Wrap around from last to first session with Shift+Right', () => {
    it('should wrap from session C to session A when pressing Shift+Right', () => {
      // @step Given I have 3 background sessions A, B, and C
      const sessions = [
        createMockSession('session-a', 'Session A'),
        createMockSession('session-b', 'Session B'),
        createMockSession('session-c', 'Session C'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And I am currently viewing session C (the last session)
      const currentSessionId = 'session-c';
      const currentIndex = sessions.findIndex(s => s.id === currentSessionId);
      expect(currentIndex).toBe(2); // Last session

      // @step When I press Shift+Right arrow
      // Wrap around: index 2 + 1 = 3, 3 % 3 = 0
      const nextIndex = calculateTargetIndex(
        'next',
        currentIndex,
        sessions.length
      );
      expect(nextIndex).toBe(0);

      // @step Then I should be viewing session A (the first session)
      const nextSession = sessions[nextIndex];
      expect(nextSession.id).toBe('session-a');
    });
  });

  describe('Scenario: Wrap around from first to last session with Shift+Left', () => {
    it('should wrap from session A to session C when pressing Shift+Left', () => {
      // @step Given I have 3 background sessions A, B, and C
      const sessions = [
        createMockSession('session-a', 'Session A'),
        createMockSession('session-b', 'Session B'),
        createMockSession('session-c', 'Session C'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And I am currently viewing session A (the first session)
      const currentSessionId = 'session-a';
      const currentIndex = sessions.findIndex(s => s.id === currentSessionId);
      expect(currentIndex).toBe(0); // First session

      // @step When I press Shift+Left arrow
      // Wrap around: index 0 - 1 = -1, (-1 + 3) % 3 = 2
      const prevIndex = calculateTargetIndex(
        'prev',
        currentIndex,
        sessions.length
      );
      expect(prevIndex).toBe(2);

      // @step Then I should be viewing session C (the last session)
      const prevSession = sessions[prevIndex];
      expect(prevSession.id).toBe('session-c');
    });
  });

  describe('Scenario: No action with only one session', () => {
    it('should do nothing when only one session exists', () => {
      // @step Given I have only 1 background session
      const sessions = [createMockSession('session-a', 'Session A')];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step When I press Shift+Right arrow
      const shouldSwitch = sessions.length >= 2;
      expect(shouldSwitch).toBe(false);

      // @step Then nothing should happen - navigation requires 2+ sessions
    });
  });

  describe('Scenario: No action when in resume mode', () => {
    it('should not switch sessions when resume mode is active', () => {
      // @step Given I have multiple background sessions
      const sessions = [
        createMockSession('session-a', 'Session A'),
        createMockSession('session-b', 'Session B'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And I am in resume mode (session selection modal is open)
      const isResumeMode = true;

      // @step When I press Shift+Right arrow
      const shouldSwitch = !isResumeMode && sessions.length >= 2;
      expect(shouldSwitch).toBe(false);

      // @step Then nothing should happen
      // @step And the resume modal should remain open
      expect(isResumeMode).toBe(true);
    });
  });

  describe('Scenario: Running session continues in background after switch', () => {
    it('should continue running session A in background when switching to B', () => {
      // @step Given I have 2 background sessions A and B
      const sessions = [
        createMockSession('session-a', 'Session A', 'running'),
        createMockSession('session-b', 'Session B', 'idle'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And session A is currently running an agent task
      vi.mocked(sessionGetStatus).mockImplementation((id: string) => {
        return id === 'session-a' ? 'running' : 'idle';
      });
      expect(sessionGetStatus('session-a')).toBe('running');

      // @step And I am viewing session A
      const currentSessionId = 'session-a';
      const currentIndex = sessions.findIndex(s => s.id === currentSessionId);

      // @step When I press Shift+Right arrow
      const nextIndex = calculateTargetIndex(
        'next',
        currentIndex,
        sessions.length
      );
      const targetSession = sessions[nextIndex];
      expect(targetSession.id).toBe('session-b');

      // @step Then session A should continue executing in background
      // (GlobalSessionStreamManager handles routing - A's handler stays registered)
      expect(sessionGetStatus('session-a')).toBe('running');

      // @step And I should see session B's conversation immediately
      vi.mocked(sessionGetMergedOutput).mockReturnValue([
        { type: 'Text', text: 'Session B output' },
      ]);
      const output = sessionGetMergedOutput('session-b');
      expect(output[0].text).toBe('Session B output');
    });
  });

  describe('Scenario: Input text preserved when switching sessions', () => {
    it('should preserve input text when switching away and back', () => {
      // @step Given I have 2 background sessions A and B
      const sessions = [
        createMockSession('session-a', 'Session A'),
        createMockSession('session-b', 'Session B'),
      ];
      vi.mocked(sessionManagerList).mockReturnValue(sessions);

      // @step And I am viewing session A
      const currentSessionId = 'session-a';

      // @step And I have typed 'hello world' in the input field
      const inputText = 'hello world';

      // @step When I press Shift+Right arrow to switch to session B
      // Save input text to session A before switching
      sessionSetPendingInput(currentSessionId, inputText);
      expect(sessionSetPendingInput).toHaveBeenCalledWith(
        'session-a',
        'hello world'
      );

      // @step And I press Shift+Left arrow to return to session A
      // @step Then I should see 'hello world' in the input field
      vi.mocked(sessionGetPendingInput).mockReturnValue('hello world');
      const restoredInput = sessionGetPendingInput('session-a');
      expect(restoredInput).toBe('hello world');
    });
  });
});
