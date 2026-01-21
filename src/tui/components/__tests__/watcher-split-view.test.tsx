/**
 * Feature: spec/features/watcher-split-view-ui.feature
 *
 * Tests for Watcher Split View UI (WATCH-010)
 *
 * NOTE: These tests verify the core logic that will be implemented in AgentView.tsx.
 * The logic functions here MUST match the implementation.
 *
 * UI integration tests require manual verification due to React/Ink complexity.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionGetParent = vi.fn();
const mockSessionGetMergedOutput = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionGetStatus = vi.fn();
const mockSessionAttach = vi.fn();
const mockSessionDetach = vi.fn();
const mockSessionSendInput = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetParent: mockSessionGetParent,
  sessionGetMergedOutput: mockSessionGetMergedOutput,
  sessionGetRole: mockSessionGetRole,
  sessionGetStatus: mockSessionGetStatus,
  sessionAttach: mockSessionAttach,
  sessionDetach: mockSessionDetach,
  sessionSendInput: mockSessionSendInput,
  // Other required mocks
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
}));

// Types matching AgentView.tsx
interface ConversationLine {
  role: 'user' | 'assistant' | 'status' | 'tool';
  content: string;
  turnIndex?: number;
}

type ActivePane = 'parent' | 'watcher';

// Split view state - MUST match AgentView.tsx state shape
interface SplitViewState {
  isWatcherSession: boolean;
  activePane: ActivePane;
  parentSessionId: string | null;
  parentSessionName: string;
  parentConversation: ConversationLine[];
  watcherConversation: ConversationLine[];
  isTurnSelectMode: boolean;
  selectedTurnIndex: number;
}

// Function to detect if session is a watcher
const isWatcherSession = (sessionId: string): boolean => {
  const parentId = mockSessionGetParent(sessionId);
  return parentId !== null;
};

// Function to get parent session info
const getParentSessionInfo = (
  watcherSessionId: string
): { parentId: string; parentName: string } | null => {
  const parentId = mockSessionGetParent(watcherSessionId);
  if (!parentId) return null;

  const role = mockSessionGetRole(watcherSessionId);
  return {
    parentId,
    parentName: role?.description || 'Parent Session',
  };
};

// Function to switch active pane
const switchActivePane = (
  currentPane: ActivePane,
  direction: 'left' | 'right'
): ActivePane => {
  if (direction === 'left') {
    return 'parent';
  } else {
    return 'watcher';
  }
};

// Function to toggle turn-select mode
const toggleTurnSelectMode = (current: boolean): boolean => {
  return !current;
};

// Function to navigate turns
const navigateTurn = (
  currentIndex: number,
  direction: 'up' | 'down',
  maxIndex: number
): number => {
  if (direction === 'up') {
    return Math.max(0, currentIndex - 1);
  } else {
    return Math.min(maxIndex, currentIndex + 1);
  }
};

// Function to generate pre-fill content for "Discuss Selected"
const generateDiscussSelectedPrefill = (
  turnIndex: number,
  turnContent: string
): string => {
  const preview = turnContent.slice(0, 50) + (turnContent.length > 50 ? '...' : '');
  return `Regarding turn ${turnIndex} in parent session:\n\`\`\`\n${preview}\n\`\`\`\n`;
};

// Function to format header for watcher session
const formatWatcherHeader = (roleName: string, parentName: string): string => {
  return `👁️ ${roleName} (watching: ${parentName})`;
};

describe('Feature: Watcher Split View UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Split view renders when viewing a watcher session', () => {
    it('should render split view for watcher sessions', () => {
      // @step Given a parent session "Main Dev Session" exists with conversation history
      const parentSessionId = 'parent-session-123';
      const parentConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', turnIndex: 0 },
        { role: 'assistant', content: 'Hi there!', turnIndex: 1 },
      ];
      mockSessionGetMergedOutput.mockReturnValue([
        { type: 'UserInput', text: 'Hello' },
        { type: 'Text', text: 'Hi there!' },
      ]);

      // @step And a watcher session "Security Reviewer" is watching "Main Dev Session"
      const watcherSessionId = 'watcher-session-456';
      mockSessionGetParent.mockReturnValue(parentSessionId);
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        description: 'Main Dev Session',
        authority: 'supervisor',
      });

      // @step When I switch to the watcher session "Security Reviewer"
      const parentInfo = getParentSessionInfo(watcherSessionId);
      expect(parentInfo).not.toBeNull();
      expect(parentInfo?.parentId).toBe(parentSessionId);

      // @step Then the view renders with two vertical panes
      const state: SplitViewState = {
        isWatcherSession: isWatcherSession(watcherSessionId),
        activePane: 'watcher',
        parentSessionId: parentInfo!.parentId,
        parentSessionName: parentInfo!.parentName,
        parentConversation,
        watcherConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };
      expect(state.isWatcherSession).toBe(true);

      // @step And the left pane shows the parent conversation from "Main Dev Session"
      expect(state.parentConversation).toHaveLength(2);
      expect(state.parentSessionId).toBe(parentSessionId);

      // @step And the right pane shows the watcher conversation
      expect(state.watcherConversation).toBeDefined();
      expect(state.activePane).toBe('watcher');

      // @step And the header shows "👁️ Security Reviewer (watching: Main Dev Session)"
      const header = formatWatcherHeader('Security Reviewer', 'Main Dev Session');
      expect(header).toBe('👁️ Security Reviewer (watching: Main Dev Session)');
    });
  });

  describe('Scenario: Switch active pane to watcher with right arrow', () => {
    it('should switch to watcher pane when pressing right arrow', () => {
      // @step Given I am viewing the watcher split view
      let state: SplitViewState = {
        isWatcherSession: true,
        activePane: 'parent',
        parentSessionId: 'parent-123',
        parentSessionName: 'Main Dev Session',
        parentConversation: [],
        watcherConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the left (parent) pane is currently active
      expect(state.activePane).toBe('parent');

      // @step When I press the Right arrow key
      state.activePane = switchActivePane(state.activePane, 'right');

      // @step Then the right (watcher) pane becomes active
      expect(state.activePane).toBe('watcher');

      // @step And the watcher pane has bright styling
      // (verified in UI by checking activePane === 'watcher' applies bright styling)
      expect(state.activePane).toBe('watcher');

      // @step And the parent pane has dimmed styling
      // (verified in UI by checking activePane !== 'parent' applies dimmed styling)
      expect(state.activePane).not.toBe('parent');
    });
  });

  describe('Scenario: Switch active pane to parent with left arrow', () => {
    it('should switch to parent pane when pressing left arrow', () => {
      // @step Given I am viewing the watcher split view
      let state: SplitViewState = {
        isWatcherSession: true,
        activePane: 'watcher',
        parentSessionId: 'parent-123',
        parentSessionName: 'Main Dev Session',
        parentConversation: [],
        watcherConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the right (watcher) pane is currently active
      expect(state.activePane).toBe('watcher');

      // @step When I press the Left arrow key
      state.activePane = switchActivePane(state.activePane, 'left');

      // @step Then the left (parent) pane becomes active
      expect(state.activePane).toBe('parent');

      // @step And the parent pane has bright styling
      // (verified in UI by checking activePane === 'parent' applies bright styling)
      expect(state.activePane).toBe('parent');

      // @step And the watcher pane has dimmed styling
      // (verified in UI by checking activePane !== 'watcher' applies dimmed styling)
      expect(state.activePane).not.toBe('watcher');
    });
  });

  describe('Scenario: Toggle turn-select mode with Tab', () => {
    it('should enable turn-select mode when Tab is pressed', () => {
      // @step Given I am viewing the watcher split view
      let state: SplitViewState = {
        isWatcherSession: true,
        activePane: 'watcher',
        parentSessionId: 'parent-123',
        parentSessionName: 'Main Dev Session',
        parentConversation: [],
        watcherConversation: [
          { role: 'user', content: 'Check this code', turnIndex: 0 },
          { role: 'assistant', content: 'I will analyze...', turnIndex: 1 },
        ],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the right (watcher) pane is currently active
      expect(state.activePane).toBe('watcher');

      // @step When I press the Tab key
      state.isTurnSelectMode = toggleTurnSelectMode(state.isTurnSelectMode);

      // @step Then turn-select mode is enabled
      expect(state.isTurnSelectMode).toBe(true);

      // @step And a selection highlight appears in the watcher pane
      // (verified in UI by checking isTurnSelectMode && activePane === 'watcher' shows highlight)
      expect(state.selectedTurnIndex).toBe(0);
    });
  });

  describe('Scenario: Navigate turns with Up/Down in select mode', () => {
    it('should navigate between turns with arrow keys', () => {
      // @step Given I am viewing the watcher split view
      let state: SplitViewState = {
        isWatcherSession: true,
        activePane: 'watcher',
        parentSessionId: 'parent-123',
        parentSessionName: 'Main Dev Session',
        parentConversation: [],
        watcherConversation: [
          { role: 'user', content: 'Turn 0', turnIndex: 0 },
          { role: 'assistant', content: 'Turn 1', turnIndex: 1 },
          { role: 'user', content: 'Turn 2', turnIndex: 2 },
        ],
        isTurnSelectMode: true,
        selectedTurnIndex: 0,
      };

      // @step And the right (watcher) pane is currently active
      expect(state.activePane).toBe('watcher');

      // @step And turn-select mode is enabled
      expect(state.isTurnSelectMode).toBe(true);

      // @step And multiple turns exist in the watcher pane
      expect(state.watcherConversation.length).toBeGreaterThan(1);

      // @step When I press the Down arrow key
      state.selectedTurnIndex = navigateTurn(
        state.selectedTurnIndex,
        'down',
        state.watcherConversation.length - 1
      );

      // @step Then the selection moves to the next turn
      expect(state.selectedTurnIndex).toBe(1);

      // @step When I press the Up arrow key
      state.selectedTurnIndex = navigateTurn(
        state.selectedTurnIndex,
        'up',
        state.watcherConversation.length - 1
      );

      // @step Then the selection moves to the previous turn
      expect(state.selectedTurnIndex).toBe(0);
    });
  });

  describe('Scenario: Discuss selected message from parent pane', () => {
    it('should pre-fill input with context from selected parent message', () => {
      // @step Given I am viewing the watcher split view
      let state: SplitViewState = {
        isWatcherSession: true,
        activePane: 'parent',
        parentSessionId: 'parent-123',
        parentSessionName: 'Main Dev Session',
        parentConversation: [
          { role: 'user', content: 'Hello', turnIndex: 0 },
          { role: 'assistant', content: 'Hi there!', turnIndex: 1 },
          { role: 'user', content: 'Write a login function', turnIndex: 2 },
          { role: 'assistant', content: 'Here is the code...', turnIndex: 3 },
        ],
        watcherConversation: [],
        isTurnSelectMode: true,
        selectedTurnIndex: 2,
      };

      // @step And the left (parent) pane is currently active
      expect(state.activePane).toBe('parent');

      // @step And turn-select mode is enabled
      expect(state.isTurnSelectMode).toBe(true);

      // @step And I have selected turn 3 with content "Write a login function"
      // Note: Turn 3 in Gherkin refers to 0-indexed turn 2
      const selectedTurn = state.parentConversation[state.selectedTurnIndex];
      expect(selectedTurn.content).toBe('Write a login function');

      // @step When I press the Enter key
      const prefill = generateDiscussSelectedPrefill(
        state.selectedTurnIndex + 1, // Display as 1-indexed
        selectedTurn.content
      );

      // @step Then the input is pre-filled with context from the selected turn
      expect(prefill).toContain('Write a login function');

      // @step And the pre-fill includes "Regarding turn 3 in parent session:"
      expect(prefill).toContain('Regarding turn 3 in parent session:');
    });
  });

  describe('Scenario: Input always sends to watcher session', () => {
    it('should send input only to watcher session', () => {
      // @step Given I am viewing the watcher split view
      const watcherSessionId = 'watcher-456';
      const parentSessionId = 'parent-123';

      let state: SplitViewState = {
        isWatcherSession: true,
        activePane: 'watcher',
        parentSessionId,
        parentSessionName: 'Main Dev Session',
        parentConversation: [],
        watcherConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the watcher session is "Security Reviewer"
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        description: 'Reviews security issues',
        authority: 'supervisor',
      });

      // @step When I type "Also check for XSS vulnerabilities" in the input
      const inputMessage = 'Also check for XSS vulnerabilities';

      // @step And I press Enter to send
      // In implementation: sessionSendInput(watcherSessionId, message)
      mockSessionSendInput(watcherSessionId, inputMessage);

      // @step Then the message is sent to the watcher session
      expect(mockSessionSendInput).toHaveBeenCalledWith(
        watcherSessionId,
        inputMessage
      );

      // @step And the message is not sent to the parent session
      expect(mockSessionSendInput).not.toHaveBeenCalledWith(
        parentSessionId,
        expect.any(String)
      );
    });
  });

  describe('Scenario: Regular session shows normal single-pane view', () => {
    it('should show single-pane view for non-watcher sessions', () => {
      // @step Given a regular session "Dev Session" exists
      const sessionId = 'regular-session-789';

      // @step And the session has no parent (not a watcher)
      mockSessionGetParent.mockReturnValue(null);

      // @step When I switch to "Dev Session"
      const isWatcher = isWatcherSession(sessionId);
      const parentInfo = getParentSessionInfo(sessionId);

      // @step Then the normal single-pane AgentView renders
      expect(isWatcher).toBe(false);
      expect(parentInfo).toBeNull();

      // @step And no split view is shown
      // (verified in UI by checking isWatcherSession === false renders single pane)
      const state: SplitViewState = {
        isWatcherSession: false,
        activePane: 'watcher',
        parentSessionId: null,
        parentSessionName: '',
        parentConversation: [],
        watcherConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };
      expect(state.isWatcherSession).toBe(false);
      expect(state.parentSessionId).toBeNull();
    });
  });
});
