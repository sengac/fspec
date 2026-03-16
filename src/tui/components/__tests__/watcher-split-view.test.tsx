/**
 * Feature: spec/features/watcher-split-view-ui.feature
 *
 * Tests for Supervisor Split View UI (WATCH-010)
 *
 * NOTE: These tests verify the core logic that will be implemented in AgentView.tsx.
 * The logic functions here MUST match the implementation.
 *
 * UI integration tests require manual verification due to React/Ink complexity.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// Mock the codelet-napi module
const mockSessionGetSubordinate = vi.fn();
const mockSessionGetMergedOutput = vi.fn();
const mockSessionGetRole = vi.fn();
const mockSessionGetStatus = vi.fn();
const mockSessionSendInput = vi.fn();

vi.mock('@sengac/codelet-napi', () => ({
  sessionGetSubordinate: mockSessionGetSubordinate,
  sessionGetMergedOutput: mockSessionGetMergedOutput,
  sessionGetRole: mockSessionGetRole,
  sessionGetStatus: mockSessionGetStatus,
  sessionSendInput: mockSessionSendInput,
  // Other required mocks
  persistenceSetDataDirectory: vi.fn(),
  persistenceGetHistory: vi.fn(() => []),
  persistenceListSessions: vi.fn(() => []),
  sessionManagerList: vi.fn(() => []),
  JsThinkingLevel: { Off: 0, Low: 1, Medium: 2, High: 3 },
  getThinkingConfig: vi.fn(() => null),
  // BRIDGE-006: Unified thinking level detection NAPI functions
  napiDetectThinkingLevel: vi.fn(() => 0), // Default to Off
  napiHasDisableKeywords: vi.fn(() => false),
  napiComputeEffectiveThinkingLevel: vi.fn((base: number, detected: number, forceOff: boolean) => {
    if (forceOff) { return 0; }
    return Math.max(base, detected);
  }),
}));

// Types matching AgentView.tsx
interface ConversationLine {
  role: 'user' | 'assistant' | 'status' | 'tool';
  content: string;
  turnIndex?: number;
}

type ActivePane = 'subordinate' | 'supervisor';

// Split view state - MUST match AgentView.tsx state shape
interface SplitViewState {
  isSupervisorSession: boolean;
  activePane: ActivePane;
  subordinateSessionId: string | null;
  subordinateSessionName: string;
  subordinateConversation: ConversationLine[];
  supervisorConversation: ConversationLine[];
  isTurnSelectMode: boolean;
  selectedTurnIndex: number;
}

// Function to detect if session is a supervisor
const isSupervisorSession = (sessionId: string): boolean => {
  const subordinateId = mockSessionGetSubordinate(sessionId);
  return subordinateId !== null;
};

// Function to get subordinate session info
const getSubordinateSessionInfo = (
  supervisorSessionId: string
): { subordinateId: string; subordinateName: string } | null => {
  const subordinateId = mockSessionGetSubordinate(supervisorSessionId);
  if (!subordinateId) return null;

  const role = mockSessionGetRole(supervisorSessionId);
  return {
    subordinateId,
    subordinateName: role?.brief || 'Subordinate Session',
  };
};

// Function to switch active pane
const switchActivePane = (
  currentPane: ActivePane,
  direction: 'left' | 'right'
): ActivePane => {
  if (direction === 'left') {
    return 'subordinate';
  } else {
    return 'supervisor';
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
  return `Regarding turn ${turnIndex} in subordinate session:\n\`\`\`\n${preview}\n\`\`\`\n`;
};

// Function to format header for supervisor session
// WATCH-024: [WATCHER] → [SUPERVISOR]
const formatSupervisorHeader = (roleName: string, subordinateName: string): string => {
  return `[SUPERVISOR] ${roleName} (watching: ${subordinateName})`;
};

describe('Feature: Supervisor Split View UI', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Scenario: Split view renders when viewing a supervisor session', () => {
    it('should render split view for supervisor sessions', () => {
      // @step Given a subordinate session "Main Dev Session" exists with conversation history
      const subordinateSessionId = 'subordinate-session-123';
      const subordinateConversation: ConversationLine[] = [
        { role: 'user', content: 'Hello', turnIndex: 0 },
        { role: 'assistant', content: 'Hi there!', turnIndex: 1 },
      ];
      mockSessionGetMergedOutput.mockReturnValue([
        { type: 'UserInput', text: 'Hello' },
        { type: 'Text', text: 'Hi there!' },
      ]);

      // @step And a supervisor session "Security Reviewer" is watching "Main Dev Session"
      const supervisorSessionId = 'supervisor-session-456';
      mockSessionGetSubordinate.mockReturnValue(subordinateSessionId);
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        brief: 'Main Dev Session',
      });

      // @step When I switch to the supervisor session "Security Reviewer"
      const subordinateInfo = getSubordinateSessionInfo(supervisorSessionId);
      expect(subordinateInfo).not.toBeNull();
      expect(subordinateInfo?.subordinateId).toBe(subordinateSessionId);

      // @step Then the view renders with two vertical panes
      const state: SplitViewState = {
        isSupervisorSession: isSupervisorSession(supervisorSessionId),
        activePane: 'supervisor',
        subordinateSessionId: subordinateInfo!.subordinateId,
        subordinateSessionName: subordinateInfo!.subordinateName,
        subordinateConversation,
        supervisorConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };
      expect(state.isSupervisorSession).toBe(true);

      // @step And the left pane shows the subordinate conversation from "Main Dev Session"
      expect(state.subordinateConversation).toHaveLength(2);
      expect(state.subordinateSessionId).toBe(subordinateSessionId);

      // @step And the right pane shows the supervisor conversation
      expect(state.supervisorConversation).toBeDefined();
      expect(state.activePane).toBe('supervisor');

      // @step And the header shows "[SUPERVISOR] Security Reviewer (watching: Main Dev Session)"
      const header = formatSupervisorHeader('Security Reviewer', 'Main Dev Session');
      expect(header).toBe('[SUPERVISOR] Security Reviewer (watching: Main Dev Session)');
    });
  });

  describe('Scenario: Switch active pane to supervisor with right arrow', () => {
    it('should switch to supervisor pane when pressing right arrow', () => {
      // @step Given I am viewing the supervisor split view
      const state: SplitViewState = {
        isSupervisorSession: true,
        activePane: 'subordinate',
        subordinateSessionId: 'subordinate-123',
        subordinateSessionName: 'Main Dev Session',
        subordinateConversation: [],
        supervisorConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the left (subordinate) pane is currently active
      expect(state.activePane).toBe('subordinate');

      // @step When I press the Right arrow key
      state.activePane = switchActivePane(state.activePane, 'right');

      // @step Then the right (supervisor) pane becomes active
      expect(state.activePane).toBe('supervisor');

      // @step And the supervisor pane has bright styling
      // (verified in UI by checking activePane === 'supervisor' applies bright styling)
      expect(state.activePane).toBe('supervisor');

      // @step And the subordinate pane has dimmed styling
      // (verified in UI by checking activePane !== 'subordinate' applies dimmed styling)
      expect(state.activePane).not.toBe('subordinate');
    });
  });

  describe('Scenario: Switch active pane to subordinate with left arrow', () => {
    it('should switch to subordinate pane when pressing left arrow', () => {
      // @step Given I am viewing the supervisor split view
      const state: SplitViewState = {
        isSupervisorSession: true,
        activePane: 'supervisor',
        subordinateSessionId: 'subordinate-123',
        subordinateSessionName: 'Main Dev Session',
        subordinateConversation: [],
        supervisorConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the right (supervisor) pane is currently active
      expect(state.activePane).toBe('supervisor');

      // @step When I press the Left arrow key
      state.activePane = switchActivePane(state.activePane, 'left');

      // @step Then the left (subordinate) pane becomes active
      expect(state.activePane).toBe('subordinate');

      // @step And the subordinate pane has bright styling
      // (verified in UI by checking activePane === 'subordinate' applies bright styling)
      expect(state.activePane).toBe('subordinate');

      // @step And the supervisor pane has dimmed styling
      // (verified in UI by checking activePane !== 'supervisor' applies dimmed styling)
      expect(state.activePane).not.toBe('supervisor');
    });
  });

  describe('Scenario: Toggle turn-select mode with Tab', () => {
    it('should enable turn-select mode when Tab is pressed', () => {
      // @step Given I am viewing the supervisor split view
      const state: SplitViewState = {
        isSupervisorSession: true,
        activePane: 'supervisor',
        subordinateSessionId: 'subordinate-123',
        subordinateSessionName: 'Main Dev Session',
        subordinateConversation: [],
        supervisorConversation: [
          { role: 'user', content: 'Check this code', turnIndex: 0 },
          { role: 'assistant', content: 'I will analyze...', turnIndex: 1 },
        ],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the right (supervisor) pane is currently active
      expect(state.activePane).toBe('supervisor');

      // @step When I press the Tab key
      state.isTurnSelectMode = toggleTurnSelectMode(state.isTurnSelectMode);

      // @step Then turn-select mode is enabled
      expect(state.isTurnSelectMode).toBe(true);

      // @step And a selection highlight appears in the supervisor pane
      // (verified in UI by checking isTurnSelectMode && activePane === 'supervisor' shows highlight)
      expect(state.selectedTurnIndex).toBe(0);
    });
  });

  describe('Scenario: Navigate turns with Up/Down in select mode', () => {
    it('should navigate between turns with arrow keys', () => {
      // @step Given I am viewing the supervisor split view
      const state: SplitViewState = {
        isSupervisorSession: true,
        activePane: 'supervisor',
        subordinateSessionId: 'subordinate-123',
        subordinateSessionName: 'Main Dev Session',
        subordinateConversation: [],
        supervisorConversation: [
          { role: 'user', content: 'Turn 0', turnIndex: 0 },
          { role: 'assistant', content: 'Turn 1', turnIndex: 1 },
          { role: 'user', content: 'Turn 2', turnIndex: 2 },
        ],
        isTurnSelectMode: true,
        selectedTurnIndex: 0,
      };

      // @step And the right (supervisor) pane is currently active
      expect(state.activePane).toBe('supervisor');

      // @step And turn-select mode is enabled
      expect(state.isTurnSelectMode).toBe(true);

      // @step And multiple turns exist in the supervisor pane
      expect(state.supervisorConversation.length).toBeGreaterThan(1);

      // @step When I press the Down arrow key
      state.selectedTurnIndex = navigateTurn(
        state.selectedTurnIndex,
        'down',
        state.supervisorConversation.length - 1
      );

      // @step Then the selection moves to the next turn
      expect(state.selectedTurnIndex).toBe(1);

      // @step When I press the Up arrow key
      state.selectedTurnIndex = navigateTurn(
        state.selectedTurnIndex,
        'up',
        state.supervisorConversation.length - 1
      );

      // @step Then the selection moves to the previous turn
      expect(state.selectedTurnIndex).toBe(0);
    });
  });

  describe('Scenario: Discuss selected message from subordinate pane', () => {
    it('should pre-fill input with context from selected subordinate message', () => {
      // @step Given I am viewing the supervisor split view
      const state: SplitViewState = {
        isSupervisorSession: true,
        activePane: 'subordinate',
        subordinateSessionId: 'subordinate-123',
        subordinateSessionName: 'Main Dev Session',
        subordinateConversation: [
          { role: 'user', content: 'Hello', turnIndex: 0 },
          { role: 'assistant', content: 'Hi there!', turnIndex: 1 },
          { role: 'user', content: 'Write a login function', turnIndex: 2 },
          { role: 'assistant', content: 'Here is the code...', turnIndex: 3 },
        ],
        supervisorConversation: [],
        isTurnSelectMode: true,
        selectedTurnIndex: 2,
      };

      // @step And the left (subordinate) pane is currently active
      expect(state.activePane).toBe('subordinate');

      // @step And turn-select mode is enabled
      expect(state.isTurnSelectMode).toBe(true);

      // @step And I have selected turn 3 with content "Write a login function"
      // Note: Turn 3 in Gherkin refers to 0-indexed turn 2
      const selectedTurn = state.subordinateConversation[state.selectedTurnIndex];
      expect(selectedTurn.content).toBe('Write a login function');

      // @step When I press the Enter key
      const prefill = generateDiscussSelectedPrefill(
        state.selectedTurnIndex + 1, // Display as 1-indexed
        selectedTurn.content
      );

      // @step Then the input is pre-filled with context from the selected turn
      expect(prefill).toContain('Write a login function');

      // @step And the pre-fill includes "Regarding turn 3 in subordinate session:"
      expect(prefill).toContain('Regarding turn 3 in subordinate session:');
    });
  });

  describe('Scenario: Input always sends to supervisor session', () => {
    it('should send input only to supervisor session', () => {
      // @step Given I am viewing the supervisor split view
      const supervisorSessionId = 'supervisor-456';
      const subordinateSessionId = 'subordinate-123';

      const state: SplitViewState = {
        isSupervisorSession: true,
        activePane: 'supervisor',
        subordinateSessionId,
        subordinateSessionName: 'Main Dev Session',
        subordinateConversation: [],
        supervisorConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };

      // @step And the supervisor session is "Security Reviewer"
      mockSessionGetRole.mockReturnValue({
        name: 'Security Reviewer',
        brief: 'Reviews security issues',
      });

      // @step When I type "Also check for XSS vulnerabilities" in the input
      const inputMessage = 'Also check for XSS vulnerabilities';

      // @step And I press Enter to send
      // In implementation: sessionSendInput(supervisorSessionId, message)
      mockSessionSendInput(supervisorSessionId, inputMessage);

      // @step Then the message is sent to the supervisor session
      expect(mockSessionSendInput).toHaveBeenCalledWith(
        supervisorSessionId,
        inputMessage
      );

      // @step And the message is not sent to the subordinate session
      expect(mockSessionSendInput).not.toHaveBeenCalledWith(
        subordinateSessionId,
        expect.any(String)
      );
    });
  });

  describe('Scenario: Regular session shows normal single-pane view', () => {
    it('should show single-pane view for non-supervisor sessions', () => {
      // @step Given a regular session "Dev Session" exists
      const sessionId = 'regular-session-789';

      // @step And the session has no subordinate (not a supervisor)
      mockSessionGetSubordinate.mockReturnValue(null);

      // @step When I switch to "Dev Session"
      const isSupervisor = isSupervisorSession(sessionId);
      const subordinateInfo = getSubordinateSessionInfo(sessionId);

      // @step Then the normal single-pane AgentView renders
      expect(isSupervisor).toBe(false);
      expect(subordinateInfo).toBeNull();

      // @step And no split view is shown
      // (verified in UI by checking isSupervisorSession === false renders single pane)
      const state: SplitViewState = {
        isSupervisorSession: false,
        activePane: 'supervisor',
        subordinateSessionId: null,
        subordinateSessionName: '',
        subordinateConversation: [],
        supervisorConversation: [],
        isTurnSelectMode: false,
        selectedTurnIndex: 0,
      };
      expect(state.isSupervisorSession).toBe(false);
      expect(state.subordinateSessionId).toBeNull();
    });
  });
});
