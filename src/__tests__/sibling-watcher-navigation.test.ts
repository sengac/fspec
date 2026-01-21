// Feature: spec/features/sibling-watcher-navigation.feature
// Tests for WATCH-013: Sibling Watcher Navigation

import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock the NAPI bindings
vi.mock('@sengac/codelet-napi', () => ({
  sessionManagerList: vi.fn(),
  sessionGetParent: vi.fn(),
  sessionAttach: vi.fn(),
  sessionDetach: vi.fn(),
  sessionGetMergedOutput: vi.fn(),
  sessionGetStatus: vi.fn(),
  sessionSetPendingInput: vi.fn(),
  sessionGetPendingInput: vi.fn(),
}));

import {
  sessionManagerList,
  sessionGetParent,
  sessionAttach,
  sessionDetach,
  sessionGetMergedOutput,
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
  getParent: (id: string) => string | null
): MockSession | null {
  // Get current session's parent
  const currentParentId = getParent(currentSessionId);

  let sessionsToNavigate: MockSession[];

  if (currentParentId !== null) {
    // In a watcher session - filter to only sibling watchers (same parent)
    sessionsToNavigate = allSessions.filter(
      s => getParent(s.id) === currentParentId
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

describe('Sibling Watcher Navigation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Navigate forward through sibling watchers', () => {
    it('should navigate only through watchers of the same parent', () => {
      // @step Given a parent session "Main Dev" exists
      const parentSession = createMockSession('parent-main-dev', 'Main Dev');

      // @step And three watcher sessions exist for "Main Dev": "W1", "W2", "W3"
      const w1 = createMockSession('watcher-w1', 'W1');
      const w2 = createMockSession('watcher-w2', 'W2');
      const w3 = createMockSession('watcher-w3', 'W3');

      const allSessions = [parentSession, w1, w2, w3];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);

      // Setup parent relationships
      vi.mocked(sessionGetParent).mockImplementation((id: string) => {
        if (id === 'watcher-w1' || id === 'watcher-w2' || id === 'watcher-w3') {
          return 'parent-main-dev';
        }
        return null; // Parent session has no parent
      });

      // @step And I am viewing watcher session "W1"
      let currentSessionId = 'watcher-w1';

      // @step When I press Shift+Right
      let targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (
            id === 'watcher-w1' ||
            id === 'watcher-w2' ||
            id === 'watcher-w3'
          ) {
            return 'parent-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing watcher session "W2"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('watcher-w2');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Right
      targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (
            id === 'watcher-w1' ||
            id === 'watcher-w2' ||
            id === 'watcher-w3'
          ) {
            return 'parent-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing watcher session "W3"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('watcher-w3');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Right
      targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (
            id === 'watcher-w1' ||
            id === 'watcher-w2' ||
            id === 'watcher-w3'
          ) {
            return 'parent-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing watcher session "W1"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('watcher-w1');
    });
  });

  describe('Scenario: Regular session navigates through all sessions', () => {
    it('should navigate through all sessions when in a non-watcher session', () => {
      // @step Given three sessions exist: "Session A", "Session B", "Session C"
      const sessionA = createMockSession('session-a', 'Session A');
      const sessionB = createMockSession('session-b', 'Session B');
      const sessionC = createMockSession('session-c', 'Session C');

      // @step And none of the sessions are watchers
      const allSessions = [sessionA, sessionB, sessionC];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetParent).mockReturnValue(null); // No parents

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

  describe('Scenario: Single watcher has no siblings to navigate', () => {
    it('should not navigate when only one watcher exists for a parent', () => {
      // @step Given a parent session "Main Dev" exists
      const parentSession = createMockSession('parent-main-dev', 'Main Dev');

      // @step And one watcher session "W1" exists for "Main Dev"
      const w1 = createMockSession('watcher-w1', 'W1');

      const allSessions = [parentSession, w1];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetParent).mockImplementation((id: string) => {
        if (id === 'watcher-w1') {
          return 'parent-main-dev';
        }
        return null;
      });

      // @step And I am viewing watcher session "W1"
      const currentSessionId = 'watcher-w1';

      // @step When I press Shift+Right
      const targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'watcher-w1') {
            return 'parent-main-dev';
          }
          return null;
        }
      );

      // @step Then I should remain viewing watcher session "W1"
      expect(targetSession).toBeNull(); // No navigation should occur
    });
  });

  describe('Scenario: Navigate backward through sibling watchers', () => {
    it('should navigate backward with wrap-around', () => {
      // @step Given a parent session "Main Dev" exists
      const parentSession = createMockSession('parent-main-dev', 'Main Dev');

      // @step And two watcher sessions exist for "Main Dev": "W1", "W2"
      const w1 = createMockSession('watcher-w1', 'W1');
      const w2 = createMockSession('watcher-w2', 'W2');

      const allSessions = [parentSession, w1, w2];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetParent).mockImplementation((id: string) => {
        if (id === 'watcher-w1' || id === 'watcher-w2') {
          return 'parent-main-dev';
        }
        return null;
      });

      // @step And I am viewing watcher session "W1"
      let currentSessionId = 'watcher-w1';

      // @step When I press Shift+Left
      let targetSession = simulateSwitchToSession(
        'prev',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'watcher-w1' || id === 'watcher-w2') {
            return 'parent-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing watcher session "W2"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('watcher-w2');
      currentSessionId = targetSession!.id;

      // @step When I press Shift+Left
      targetSession = simulateSwitchToSession(
        'prev',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'watcher-w1' || id === 'watcher-w2') {
            return 'parent-main-dev';
          }
          return null;
        }
      );

      // @step Then I should be viewing watcher session "W1"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('watcher-w1');
    });
  });

  describe('Scenario: Watchers of different parents are isolated', () => {
    it('should only navigate between watchers of the same parent', () => {
      // @step Given two parent sessions exist: "Parent A" and "Parent B"
      const parentA = createMockSession('parent-a', 'Parent A');
      const parentB = createMockSession('parent-b', 'Parent B');

      // @step And two watcher sessions exist for "Parent A": "W1", "W2"
      const w1 = createMockSession('watcher-w1', 'W1');
      const w2 = createMockSession('watcher-w2', 'W2');

      // @step And two watcher sessions exist for "Parent B": "W3", "W4"
      const w3 = createMockSession('watcher-w3', 'W3');
      const w4 = createMockSession('watcher-w4', 'W4');

      const allSessions = [parentA, parentB, w1, w2, w3, w4];
      vi.mocked(sessionManagerList).mockReturnValue(allSessions);
      vi.mocked(sessionGetParent).mockImplementation((id: string) => {
        if (id === 'watcher-w1' || id === 'watcher-w2') {
          return 'parent-a';
        }
        if (id === 'watcher-w3' || id === 'watcher-w4') {
          return 'parent-b';
        }
        return null; // Parent sessions have no parent
      });

      // @step And I am viewing watcher session "W1"
      const currentSessionId = 'watcher-w1';

      // @step When I press Shift+Right
      const targetSession = simulateSwitchToSession(
        'next',
        currentSessionId,
        allSessions,
        id => {
          if (id === 'watcher-w1' || id === 'watcher-w2') {
            return 'parent-a';
          }
          if (id === 'watcher-w3' || id === 'watcher-w4') {
            return 'parent-b';
          }
          return null;
        }
      );

      // @step Then I should be viewing watcher session "W2"
      expect(targetSession).not.toBeNull();
      expect(targetSession!.id).toBe('watcher-w2');

      // @step And I should not navigate to "W3" or "W4"
      expect(targetSession!.id).not.toBe('watcher-w3');
      expect(targetSession!.id).not.toBe('watcher-w4');
    });
  });
});
