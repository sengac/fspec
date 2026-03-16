// Feature: spec/features/sibling-watcher-navigation.feature
// Tests for WATCH-013: Sibling Supervisor Navigation

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerList: vi.fn(),
  sessionGetSubordinate: vi.fn(),
  sessionGetMergedOutput: vi.fn(),
  sessionGetStatus: vi.fn(),
  sessionSetPendingInput: vi.fn(),
  sessionGetPendingInput: vi.fn(),
}));

import {
  sessionManagerList,
  sessionGetSubordinate,
  sessionSetPendingInput,
  sessionGetPendingInput,
} from '@sengac/codelet-napi';

// Mock session data
interface MockSession {
  id: string;
  name: string;
  status: 'running' | 'idle';
  project: string;
  messageCount: number;
}

const createMockSession = (
  id: string,
  name: string,
  status: 'running' | 'idle' = 'idle'
): MockSession => ({
  id,
  name,
  status,
  project: '/test/project',
  messageCount: 5,
});

// Helper to simulate switchToSession logic with sibling filtering
function simulateSwitchToSession(
  direction: 'prev' | 'next',
  currentSessionId: string,
  allSessions: MockSession[],
  getSubordinate: (id: string) => string | null
): MockSession | null {
  // Get current session's subordinate
  const currentSubordinateId = getSubordinate(currentSessionId);

  let sessionsToNavigate: MockSession[];

  if (currentSubordinateId !== null) {
    // In a supervisor session - filter to only sibling supervisors (same subordinate)
    sessionsToNavigate = allSessions.filter(
      s => getSubordinate(s.id) === currentSubordinateId
    );
  } else {
    // Regular session - navigate through all sessions
    sessionsToNavigate = allSessions;
  }

  // Need at least 2 sessions to switch
  if (sessionsToNavigate.length < 2) {
    return null;
  }

  // Find current session index in filtered list
  const currentIndex = sessionsToNavigate.findIndex(
    s => s.id === currentSessionId
  );
  if (currentIndex === -1) {
    return sessionsToNavigate[
      direction === 'next' ? 0 : sessionsToNavigate.length - 1
    ];
  }

  // Calculate target index with wrap-around
  const targetIndex =
    direction === 'next'
      ? (currentIndex + 1) % sessionsToNavigate.length
      : (currentIndex - 1 + sessionsToNavigate.length) %
        sessionsToNavigate.length;

  return sessionsToNavigate[targetIndex];
}

describe('Sibling Supervisor Navigation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Navigate forward through sibling supervisors', () => {
    it('should navigate only through supervisors of the same subordinate', () => {
      // @step Given a subordinate session "Main Dev" exists
      const subordinateSession = createMockSession(
        'subordinate-main-dev',
        'Main Dev'
      );

      // @step And three supervisor sessions exist for "Main Dev": "W1", "W2", "W3"
      const w1 = createMockSession('supervisor-w1', 'W1');
      const w2 = createMockSession('supervisor-w2', 'W2');
      const w3 = createMockSession('supervisor-w3', 'W3');

      const allSessions = [subordinateSession, w1, w2, w3];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);

      // Setup subordinate relationships
      vi.mocked(sessionGetSubordinate).mockImplementation((id: string) => {
        if (
          id === 'supervisor-w1' ||
          id === 'supervisor-w2' ||
          id === 'supervisor-w3'
        ) {
          return 'subordinate-main-dev';
        }
        return null; // Subordinate session has no subordinate
      });

      // @step And I am viewing supervisor session "W1"
      let currentSessionId = 'supervisor-w1';

      // @step When I press Shift+Right
      let targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (
            id === 'supervisor-w1' ||
            id === 'supervisor-w2' ||
            id === 'supervisor-w3'
          ) {
            return 'subordinate-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing supervisor session "W2"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('supervisor-w2');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Right
      targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (
            id === 'supervisor-w1' ||
            id === 'supervisor-w2' ||
            id === 'supervisor-w3'
          ) {
            return 'subordinate-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing supervisor session "W3"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('supervisor-w3');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Right
      targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (
            id === 'supervisor-w1' ||
            id === 'supervisor-w2' ||
            id === 'supervisor-w3'
          ) {
            return 'subordinate-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing supervisor session "W1"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('supervisor-w1');
    });
  });

  describe('Scenario: Regular session navigates through all sessions', () => {
    it('should navigate through all sessions when in a non-supervisor session', () => {
      // @step Given three sessions exist: "Session A", "Session B", "Session C"
      const sessionA = createMockSession('session-a', 'Session A');
      const sessionB = createMockSession('session-b', 'Session B');
      const sessionC = createMockSession('session-c', 'Session C');

      // @step And none of the sessions are supervisors
      const allSessions = [sessionA, sessionB, sessionC];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetSubordinate).mockReturnValue(null); // No subordinates

      // @step And I am viewing session "Session B"
      let currentSessionId = 'session-b';

      // @step When I press Shift+Right
      let targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        () => null
      );

      // @step Then I should be viewing session "Session C"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('session-c');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Right
      targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        () => null
      );

      // @step Then I should be viewing session "Session A"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('session-a');
    });
  });

  describe('Scenario: Single supervisor has no siblings to navigate', () => {
    it('should not navigate when only one supervisor exists for a subordinate', () => {
      // @step Given a subordinate session "Main Dev" exists
      const subordinateSession = createMockSession(
        'subordinate-main-dev',
        'Main Dev'
      );

      // @step And one supervisor session "W1" exists for "Main Dev"
      const w1 = createMockSession('supervisor-w1', 'W1');

      const allSessions = [subordinateSession, w1];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetSubordinate).mockImplementation((id: string) => {
        if (id === 'supervisor-w1') {
          return 'subordinate-main-dev';
        }
        return null;
      });

      // @step And I am viewing supervisor session "W1"
      const currentSessionId = 'supervisor-w1';

      // @step When I press Shift+Right
      const targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'supervisor-w1') {
            return 'subordinate-main-dev';
          }
          return null;
        }
      );

      // @step Then I should remain viewing supervisor session "W1"
      expect(targetSession).toBeNull(); // No navigation should occur
    });
  });

  describe('Scenario: Navigate backward through sibling supervisors', () => {
    it('should navigate backward with wrap-around', () => {
      // @step Given a subordinate session "Main Dev" exists
      const subordinateSession = createMockSession(
        'subordinate-main-dev',
        'Main Dev'
      );

      // @step And two supervisor sessions exist for "Main Dev": "W1", "W2"
      const w1 = createMockSession('supervisor-w1', 'W1');
      const w2 = createMockSession('supervisor-w2', 'W2');

      const allSessions = [subordinateSession, w1, w2];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetSubordinate).mockImplementation((id: string) => {
        if (id === 'supervisor-w1' || id === 'supervisor-w2') {
          return 'subordinate-main-dev';
        }
        return null;
      });

      // @step And I am viewing supervisor session "W1"
      let currentSessionId = 'supervisor-w1';

      // @step When I press Shift+Left
      let targetSession = simulateSwitchToSession(
        'prev',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'supervisor-w1' || id === 'supervisor-w2') {
            return 'subordinate-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing supervisor session "W2"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('supervisor-w2');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Left
      targetSession = simulateSwitchToSession(
        'prev',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'supervisor-w1' || id === 'supervisor-w2') {
            return 'subordinate-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing supervisor session "W1"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('supervisor-w1');
    });
  });

  describe('Scenario: Supervisors of different subordinates are isolated', () => {
    it('should only navigate between supervisors of the same subordinate', () => {
      // @step Given two subordinate sessions exist: "Subordinate A" and "Subordinate B"
      const subA = createMockSession('subordinate-a', 'Subordinate A');
      const subB = createMockSession('subordinate-b', 'Subordinate B');

      // @step And two supervisor sessions exist for "Subordinate A": "W1", "W2"
      const w1 = createMockSession('supervisor-w1', 'W1');
      const w2 = createMockSession('supervisor-w2', 'W2');

      // @step And two supervisor sessions exist for "Subordinate B": "W3", "W4"
      const w3 = createMockSession('supervisor-w3', 'W3');
      const w4 = createMockSession('supervisor-w4', 'W4');

      const allSessions = [subA, subB, w1, w2, w3, w4];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetSubordinate).mockImplementation((id: string) => {
        if (id === 'supervisor-w1' || id === 'supervisor-w2') {
          return 'subordinate-a';
        }
        if (id === 'supervisor-w3' || id === 'supervisor-w4') {
          return 'subordinate-b';
        }
        return null; // Subordinate sessions have no subordinate
      });

      // @step And I am viewing supervisor session "W1"
      const currentSessionId = 'supervisor-w1';

      // @step When I press Shift+Right
      const targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'supervisor-w1' || id === 'supervisor-w2') {
            return 'subordinate-a';
          }
          if (id === 'supervisor-w3' || id === 'supervisor-w4') {
            return 'subordinate-b';
          }
          return null;
        }
      );

      // @step Then I should be viewing supervisor session "W2"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('supervisor-w2');

      // @step And I should not navigate to "W3" or "W4"
      expect(targetSession!.id).not.toBe('supervisor-w3');
      expect(targetSession!.id).not.toBe('supervisor-w4');
    });
  });
});
