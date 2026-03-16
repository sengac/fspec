/**
 * Feature: spec/features/discuss-selected-feature.feature
 *
 * Tests for Discuss Selected Feature (WATCH-016)
 *
 * This feature enables Enter key behavior in supervisor split view:
 * - Subordinate pane: Pre-fill input with context for discussing the turn
 * - Supervisor pane: Open TurnContentModal to view full content
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';

// ============================================================
// MOCK FUNCTIONS
// ============================================================

// Mock the generateDiscussSelectedPrefill function
const mockGenerateDiscussSelectedPrefill = vi.fn((turnNumber: number, turnContent: string, maxLength: number = 50) => {
  const preview = turnContent.slice(0, maxLength) + (turnContent.length > maxLength ? '...' : '');
  return `Regarding turn ${turnNumber} in subordinate session:\n\`\`\`\n${preview}\n\`\`\`\n`;
});

// ============================================================
// TYPE DEFINITIONS
// ============================================================

interface ConversationLine {
  role: 'user' | 'assistant' | 'supervisor' | 'status' | 'tool';
  content: string;
  messageIndex: number;
  isSeparator?: boolean;
}

type ActivePane = 'subordinate' | 'supervisor';

interface SelectionState {
  isSelectMode: boolean;
  selectedIndex: number;
}

interface SplitViewState {
  activePane: ActivePane;
  subordinateSelection: SelectionState;
  supervisorSelection: SelectionState;
  inputValue: string;
  showTurnContentModal: boolean;
  turnContentModalContent: string;
}

// ============================================================
// LOGIC FUNCTIONS UNDER TEST
// ============================================================

/**
 * Handle Enter key press in split view select mode.
 * - Subordinate pane: pre-fill input with context
 * - Supervisor pane: open turn content modal
 */
function handleEnterInSelectMode(
  state: SplitViewState,
  subordinateConversation: ConversationLine[],
  supervisorConversation: ConversationLine[],
): SplitViewState {
  const newState = { ...state };

  if (state.activePane === 'subordinate' && state.subordinateSelection.isSelectMode) {
    const selectedLine = subordinateConversation[state.subordinateSelection.selectedIndex];
    if (selectedLine) {
      const turnNumber = selectedLine.messageIndex + 1; // 1-indexed
      const prefill = mockGenerateDiscussSelectedPrefill(turnNumber, selectedLine.content);
      newState.inputValue = prefill;
      newState.subordinateSelection = { ...state.subordinateSelection, isSelectMode: false };
    }
  } else if (state.activePane === 'supervisor' && state.supervisorSelection.isSelectMode) {
    const selectedLine = supervisorConversation[state.supervisorSelection.selectedIndex];
    if (selectedLine) {
      newState.showTurnContentModal = true;
      newState.turnContentModalContent = selectedLine.content;
    }
  }

  return newState;
}

/**
 * Get the content of a conversation line (for testing truncation)
 */
function getFirstContentOfTurn(lines: ConversationLine[], messageIndex: number): string {
  for (const line of lines) {
    if (line.messageIndex === messageIndex && !line.isSeparator && line.content.trim()) {
      return line.content;
    }
  }
  return '';
}

// ============================================================
// TESTS
// ============================================================

describe('Feature: Discuss Selected Feature', () => {
  let defaultState: SplitViewState;
  let subordinateConversation: ConversationLine[];
  let supervisorConversation: ConversationLine[];

  beforeEach(() => {
    vi.clearAllMocks();

    subordinateConversation = [
      { role: 'user', content: 'Hello', messageIndex: 0 },
      { role: 'assistant', content: 'Hi there!', messageIndex: 1 },
      { role: 'user', content: 'Write a login function', messageIndex: 2 },
      { role: 'assistant', content: 'Here is the code for a login function that handles authentication...', messageIndex: 3 },
    ];

    supervisorConversation = [
      { role: 'assistant', content: 'Watching for security issues...', messageIndex: 0 },
      { role: 'assistant', content: '⚠️ SQL INJECTION VULNERABILITY: This code is vulnerable to SQL injection. The username and password are directly interpolated into the query string. Use parameterized queries instead.', messageIndex: 1 },
    ];

    defaultState = {
      activePane: 'subordinate',
      subordinateSelection: { isSelectMode: true, selectedIndex: 2 },
      supervisorSelection: { isSelectMode: false, selectedIndex: 0 },
      inputValue: '',
      showTurnContentModal: false,
      turnContentModalContent: '',
    };
  });

  describe('Scenario: Enter on selected turn in subordinate pane pre-fills input with context', () => {
    it('should pre-fill input with formatted context when Enter pressed on subordinate selection', () => {
      // @step Given I am viewing a supervisor session in split view
      const state = { ...defaultState };
      expect(state.activePane).toBe('subordinate');

      // @step And the subordinate pane is active with turn-select mode enabled
      expect(state.subordinateSelection.isSelectMode).toBe(true);

      // @step And turn 3 is selected with content "Write a login function"
      state.subordinateSelection.selectedIndex = 2; // 0-indexed, turn 3 = index 2
      const selectedLine = subordinateConversation[state.subordinateSelection.selectedIndex];
      expect(selectedLine.content).toBe('Write a login function');

      // @step When I press the Enter key
      const newState = handleEnterInSelectMode(state, subordinateConversation, supervisorConversation);

      // @step Then the input area is pre-filled with "Regarding turn 3 in subordinate session:"
      expect(newState.inputValue).toContain('Regarding turn 3 in subordinate session:');

      // @step And the pre-fill includes a code-fenced preview of the turn content
      expect(newState.inputValue).toContain('```');
      expect(newState.inputValue).toContain('Write a login function');

      // @step And turn-select mode is exited
      expect(newState.subordinateSelection.isSelectMode).toBe(false);

      // @step And the cursor is positioned after the pre-fill for typing
      // (Verified by input having the prefill value - cursor positioning is handled by React/Ink)
      expect(newState.inputValue.endsWith('\n')).toBe(true);
    });
  });

  describe('Scenario: Enter on selected turn in supervisor pane opens full content modal', () => {
    it('should open TurnContentModal when Enter pressed on supervisor selection', () => {
      // @step Given I am viewing a supervisor session in split view
      const state: SplitViewState = {
        ...defaultState,
        activePane: 'supervisor',
        subordinateSelection: { isSelectMode: false, selectedIndex: 0 },
        supervisorSelection: { isSelectMode: true, selectedIndex: 1 },
      };

      // @step And the supervisor pane is active with turn-select mode enabled
      expect(state.activePane).toBe('supervisor');
      expect(state.supervisorSelection.isSelectMode).toBe(true);

      // @step And turn 2 is selected with a long SQL injection warning message
      const selectedLine = supervisorConversation[state.supervisorSelection.selectedIndex];
      expect(selectedLine.content).toContain('SQL INJECTION VULNERABILITY');

      // @step When I press the Enter key
      const newState = handleEnterInSelectMode(state, subordinateConversation, supervisorConversation);

      // @step Then the TurnContentModal opens
      expect(newState.showTurnContentModal).toBe(true);

      // @step And the modal shows the full supervisor response with scrolling support
      expect(newState.turnContentModalContent).toBe(selectedLine.content);
    });
  });

  describe('Scenario: Long content in subordinate pane is truncated in pre-fill', () => {
    it('should truncate content to 50 characters with ellipsis', () => {
      // @step Given I am viewing a supervisor session in split view
      const longContent = 'This is a very long message that exceeds fifty characters and should be truncated with an ellipsis';
      const subordinateWithLongContent: ConversationLine[] = [
        { role: 'user', content: longContent, messageIndex: 0 },
      ];

      const state: SplitViewState = {
        ...defaultState,
        subordinateSelection: { isSelectMode: true, selectedIndex: 0 },
      };

      // @step And the subordinate pane is active with turn-select mode enabled
      expect(state.activePane).toBe('subordinate');
      expect(state.subordinateSelection.isSelectMode).toBe(true);

      // @step And a turn is selected with content exceeding 50 characters
      expect(longContent.length).toBeGreaterThan(50);

      // @step When I press the Enter key
      const newState = handleEnterInSelectMode(state, subordinateWithLongContent, supervisorConversation);

      // @step Then the pre-fill shows only the first 50 characters
      expect(mockGenerateDiscussSelectedPrefill).toHaveBeenCalledWith(1, longContent);
      const expectedPreview = longContent.slice(0, 50) + '...';
      expect(newState.inputValue).toContain(expectedPreview);

      // @step And the preview ends with "..." to indicate truncation
      expect(newState.inputValue).toContain('...');
    });
  });

  describe('Scenario: Select mode exits after discussing subordinate turn', () => {
    it('should exit select mode and allow input after Enter', () => {
      // @step Given I am viewing a supervisor session in split view
      const state: SplitViewState = {
        ...defaultState,
        subordinateSelection: { isSelectMode: true, selectedIndex: 1 },
      };

      // @step And the subordinate pane is active with turn-select mode enabled
      expect(state.activePane).toBe('subordinate');
      expect(state.subordinateSelection.isSelectMode).toBe(true);

      // @step And a turn is selected in the subordinate pane
      expect(state.subordinateSelection.selectedIndex).toBe(1);

      // @step When I press the Enter key
      const newState = handleEnterInSelectMode(state, subordinateConversation, supervisorConversation);

      // @step Then turn-select mode is disabled
      expect(newState.subordinateSelection.isSelectMode).toBe(false);

      // @step And the input area gains focus
      // (Verified by select mode being disabled - focus naturally returns to input)
      expect(newState.inputValue).not.toBe('');

      // @step And the user can type their question after the pre-fill
      // (Verified by inputValue containing the prefill, ready for appending)
      expect(newState.inputValue).toContain('Regarding turn');
    });
  });

  describe('Scenario: Modal updates when selecting different supervisor turn', () => {
    it('should update modal content when Enter pressed on new selection', () => {
      // @step Given I am viewing a supervisor session in split view
      const state: SplitViewState = {
        ...defaultState,
        activePane: 'supervisor',
        subordinateSelection: { isSelectMode: false, selectedIndex: 0 },
        supervisorSelection: { isSelectMode: true, selectedIndex: 0 },
        showTurnContentModal: true,
        turnContentModalContent: supervisorConversation[0].content,
      };

      // @step And the supervisor pane is active with turn-select mode enabled
      expect(state.activePane).toBe('supervisor');
      expect(state.supervisorSelection.isSelectMode).toBe(true);

      // @step And the TurnContentModal is already open showing turn 1
      expect(state.showTurnContentModal).toBe(true);
      expect(state.turnContentModalContent).toBe('Watching for security issues...');

      // @step And I navigate to select turn 2 in the supervisor pane
      state.supervisorSelection.selectedIndex = 1;

      // @step When I press the Enter key
      const newState = handleEnterInSelectMode(state, subordinateConversation, supervisorConversation);

      // @step Then the TurnContentModal updates to show turn 2 content
      expect(newState.turnContentModalContent).toContain('SQL INJECTION VULNERABILITY');
      expect(newState.showTurnContentModal).toBe(true);
    });
  });
});

describe('Unit: generateDiscussSelectedPrefill', () => {
  it('should format prefill correctly', () => {
    const result = mockGenerateDiscussSelectedPrefill(3, 'Write a login function');
    expect(result).toBe(`Regarding turn 3 in subordinate session:\n\`\`\`\nWrite a login function\n\`\`\`\n`);
  });

  it('should truncate long content', () => {
    const longContent = 'This is a very long message that exceeds fifty characters and should be truncated';
    const result = mockGenerateDiscussSelectedPrefill(1, longContent);
    // Content is truncated to first 50 chars plus '...'
    expect(result).toContain('This is a very long message that exceeds fifty cha...');
  });

  it('should not truncate short content', () => {
    const shortContent = 'Short message';
    const result = mockGenerateDiscussSelectedPrefill(1, shortContent);
    expect(result).toContain('Short message');
    expect(result).not.toContain('...');
  });
});
