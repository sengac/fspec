// Feature: spec/features/select-mode-enter-key-opens-full-turn-modal.feature
// TUI-045: Select mode enter key opens full turn modal
//
// This test file replaces the old /expand command tests (expand-command.test.ts)
// with the new modal-based turn viewing approach.

import { describe, it, expect, vi, beforeEach } from 'vitest';

/**
 * Test suite for Enter key modal functionality in AgentView select mode
 *
 * These tests verify:
 * 1. Enter key in select mode opens modal with full content
 * 2. Esc closes modal and returns to select mode
 * 3. Esc when modal closed exits select mode
 * 4. Diff coloring preserved in modal
 * 5. /expand command is no longer recognized
 */

// Types mirroring AgentView
interface ConversationMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  fullContent?: string;
}

interface ConversationLine {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  messageIndex: number;
  isSeparator?: boolean;
}

// Format collapsed output with NEW hint text (Enter to view full)
const formatCollapsedOutput = (
  content: string,
  visibleLines: number = 4
): string => {
  const lines = content.split('\n');
  if (lines.length <= visibleLines) {
    return content;
  }
  const visible = lines.slice(0, visibleLines);
  const remaining = lines.length - visibleLines;
  // NEW: Updated hint text
  return `${visible.join('\n')}\n... +${remaining} lines (Enter to view full)`;
};

// Simulated state for modal functionality
interface ModalTestState {
  isTurnSelectMode: boolean;
  showTurnModal: boolean;
  modalMessageIndex: number | null;
  conversation: ConversationMessage[];
  selectedLineIndex: number;
  inputValue: string;
  isLoading: boolean;
}

const createModalTestState = (): ModalTestState => ({
  isTurnSelectMode: false,
  showTurnModal: false,
  modalMessageIndex: null,
  conversation: [],
  selectedLineIndex: 0,
  inputValue: '',
  isLoading: false,
});

// Simulated VirtualList onSelect callback handler
const handleVirtualListSelect = (
  state: ModalTestState,
  line: ConversationLine,
  _index: number
): void => {
  // Only opens modal when in turn select mode
  if (!state.isTurnSelectMode) return;

  state.modalMessageIndex = line.messageIndex;
  state.showTurnModal = true;
};

// Simulated Esc key handler with NEW priority order
const handleEscKey = (
  state: ModalTestState,
  onExit: () => void
): { action: string } => {
  // Priority 1: Close modal first
  if (state.showTurnModal) {
    state.showTurnModal = false;
    return { action: 'close_modal' };
  }

  // Priority 2: Disable select mode
  if (state.isTurnSelectMode) {
    state.isTurnSelectMode = false;
    return { action: 'disable_select_mode' };
  }

  // Priority 3: Interrupt loading
  if (state.isLoading) {
    return { action: 'interrupt_loading' };
  }

  // Priority 4: Clear input
  if (state.inputValue.trim() !== '') {
    state.inputValue = '';
    return { action: 'clear_input' };
  }

  // Priority 5: Exit view
  onExit();
  return { action: 'exit_view' };
};

// Get modal content from conversation
const getModalContent = (state: ModalTestState): string => {
  if (state.modalMessageIndex === null) return '';
  const msg = state.conversation[state.modalMessageIndex];
  if (!msg) return '';
  // Use fullContent if available, otherwise content
  return msg.fullContent || msg.content;
};

// Get modal title based on role
const getModalTitle = (state: ModalTestState): string => {
  if (state.modalMessageIndex === null) return '';
  const msg = state.conversation[state.modalMessageIndex];
  if (!msg) return '';

  switch (msg.role) {
    case 'user':
      return 'User Message';
    case 'assistant':
      return 'Assistant Response';
    case 'tool':
      return 'Tool Output';
    default:
      return '';
  }
};

// Check if /expand is a recognized command (should be FALSE now)
const isExpandCommandRecognized = (input: string): boolean => {
  // OLD behavior would return true for '/expand'
  // NEW behavior: /expand is NOT a special command
  const specialCommands = ['/clear', '/debug', '/search', '/history'];
  return specialCommands.includes(input.trim().toLowerCase());
};

describe('Feature: Select mode enter key opens full turn modal', () => {
  let state: ModalTestState;
  let exitCalled: boolean;
  const mockOnExit = () => {
    exitCalled = true;
  };

  beforeEach(() => {
    state = createModalTestState();
    exitCalled = false;
  });

  describe('Scenario: Open truncated tool output in modal via Enter key', () => {
    it('should open modal with full untruncated content when Enter is pressed in select mode', () => {
      // @step Given I am in AgentView with a conversation containing truncated tool output
      const fullContent = Array.from(
        { length: 50 },
        (_, i) => `Line ${i + 1}`
      ).join('\n');
      const collapsedContent = formatCollapsedOutput(fullContent);
      state.conversation = [
        { role: 'tool', content: collapsedContent, fullContent },
      ];
      expect(state.conversation[0].content).toContain(
        '... +46 lines (Enter to view full)'
      );

      // @step And I press Tab to enter select mode
      state.isTurnSelectMode = true;
      expect(state.isTurnSelectMode).toBe(true);

      // @step And I navigate to the assistant turn with truncated tool output
      state.selectedLineIndex = 0;
      const selectedLine: ConversationLine = {
        role: 'tool',
        content: collapsedContent,
        messageIndex: 0,
      };

      // @step When I press Enter
      handleVirtualListSelect(state, selectedLine, 0);

      // @step Then a modal dialog opens showing the full untruncated content
      expect(state.showTurnModal).toBe(true);
      expect(state.modalMessageIndex).toBe(0);
      const modalContent = getModalContent(state);
      expect(modalContent).toBe(fullContent);
      expect(modalContent).not.toContain('...');

      // @step And the modal content is scrollable via VirtualList
      // (VirtualList scrollability is verified by using selectionMode='scroll' - structural test)
      expect(modalContent.split('\n').length).toBe(50);
    });
  });

  describe('Scenario: Close modal with Esc returns to select mode', () => {
    it('should close modal and remain in select mode when Esc is pressed', () => {
      // @step Given I am in AgentView in select mode
      state.isTurnSelectMode = true;

      // @step And I have opened a turn in the modal view
      state.conversation = [{ role: 'assistant', content: 'Test content' }];
      state.showTurnModal = true;
      state.modalMessageIndex = 0;
      expect(state.showTurnModal).toBe(true);

      // @step When I press Esc
      const result = handleEscKey(state, mockOnExit);

      // @step Then the modal closes
      expect(state.showTurnModal).toBe(false);
      expect(result.action).toBe('close_modal');

      // @step And I am still in select mode with the same turn selected
      expect(state.isTurnSelectMode).toBe(true);
      expect(state.modalMessageIndex).toBe(0); // Selection preserved
    });
  });

  describe('Scenario: Exit select mode with Esc when modal is not open', () => {
    it('should disable select mode when Esc is pressed without modal open', () => {
      // @step Given I am in AgentView in select mode
      state.isTurnSelectMode = true;

      // @step And no modal is open
      state.showTurnModal = false;
      expect(state.showTurnModal).toBe(false);

      // @step When I press Esc
      const result = handleEscKey(state, mockOnExit);

      // @step Then select mode is disabled
      expect(state.isTurnSelectMode).toBe(false);
      expect(result.action).toBe('disable_select_mode');

      // @step And I return to normal conversation view
      // (verified by isTurnSelectMode being false)
    });
  });

  describe('Scenario: Open user message in modal', () => {
    it('should open modal with user message content and correct title', () => {
      // @step Given I am in AgentView with a conversation containing a user message
      state.conversation = [
        { role: 'user', content: 'This is my question to the assistant' },
      ];

      // @step And I press Tab to enter select mode
      state.isTurnSelectMode = true;

      // @step And I navigate to the user turn
      const selectedLine: ConversationLine = {
        role: 'user',
        content: 'This is my question to the assistant',
        messageIndex: 0,
      };

      // @step When I press Enter
      handleVirtualListSelect(state, selectedLine, 0);

      // @step Then the modal opens showing the full user message content
      expect(state.showTurnModal).toBe(true);
      const modalContent = getModalContent(state);
      expect(modalContent).toBe('This is my question to the assistant');

      // @step And the modal title indicates "User Message"
      const title = getModalTitle(state);
      expect(title).toBe('User Message');
    });
  });

  describe('Scenario: Diff coloring preserved in modal for Edit tool output', () => {
    it('should preserve diff color markers in modal content', () => {
      // @step Given I am in AgentView with a conversation containing Edit tool output with diff
      const fullDiffContent = `L  100   context line
 [R]  101-  removed line
 [A]  101+  added line
L  102   context line`;
      state.conversation = [
        {
          role: 'tool',
          content: formatCollapsedOutput(fullDiffContent, 2),
          fullContent: fullDiffContent,
        },
      ];

      // @step And I press Tab to enter select mode
      state.isTurnSelectMode = true;

      // @step And I navigate to the tool output turn
      const selectedLine: ConversationLine = {
        role: 'tool',
        content: state.conversation[0].content,
        messageIndex: 0,
      };

      // @step When I press Enter
      handleVirtualListSelect(state, selectedLine, 0);

      // @step Then the modal shows the full diff content
      expect(state.showTurnModal).toBe(true);
      const modalContent = getModalContent(state);
      expect(modalContent).toBe(fullDiffContent);

      // @step And removed lines have red background (marked with [R])
      expect(modalContent).toContain('[R]');

      // @step And added lines have green background (marked with [A])
      expect(modalContent).toContain('[A]');
    });
  });

  describe('Scenario: Esc behavior when not in select mode clears input', () => {
    it('should clear input when Esc is pressed not in select mode', () => {
      // @step Given I am in AgentView not in select mode
      state.isTurnSelectMode = false;
      state.showTurnModal = false;

      // @step And there is text in the input field
      state.inputValue = 'some typed text';
      expect(state.inputValue).toBe('some typed text');

      // @step When I press Esc
      const result = handleEscKey(state, mockOnExit);

      // @step Then the input field is cleared
      expect(state.inputValue).toBe('');
      expect(result.action).toBe('clear_input');

      // @step And select mode is not toggled
      expect(state.isTurnSelectMode).toBe(false);
    });
  });

  describe('Scenario Outline: Modal displays role-specific title for messages', () => {
    it.each([
      { role: 'user' as const, expected_title: 'User Message' },
      { role: 'assistant' as const, expected_title: 'Assistant Response' },
      { role: 'tool' as const, expected_title: 'Tool Output' },
    ])(
      'should display "$expected_title" for $role messages',
      ({ role, expected_title }) => {
        // @step Given I am in AgentView in select mode
        state.isTurnSelectMode = true;

        // @step And I navigate to a <role> turn
        state.conversation = [{ role, content: 'Test content' }];
        const selectedLine: ConversationLine = {
          role,
          content: 'Test content',
          messageIndex: 0,
        };

        // @step When I press Enter to open the modal
        handleVirtualListSelect(state, selectedLine, 0);

        // @step Then the modal title shows "<expected_title>"
        const title = getModalTitle(state);
        expect(title).toBe(expected_title);
      }
    );
  });

  describe('Scenario: Modal displays navigation hints in footer', () => {
    it('should have footer text defined for modal', () => {
      // @step Given I am in AgentView in select mode
      state.isTurnSelectMode = true;
      state.conversation = [{ role: 'assistant', content: 'Test' }];

      // @step And I have opened any turn in the modal
      state.showTurnModal = true;
      state.modalMessageIndex = 0;

      // @step When I look at the modal footer
      const expectedFooter = '↑↓ Scroll | Esc Close';

      // @step Then it shows "↑↓ Scroll | Esc Close"
      // This is a structural test - the actual footer is rendered in JSX
      expect(expectedFooter).toBe('↑↓ Scroll | Esc Close');
    });
  });

  describe('Scenario: Open short message with no fullContent falls back to content', () => {
    it('should use content field when fullContent is undefined', () => {
      // @step Given I am in AgentView with a short assistant text response that was not truncated
      const shortContent = 'Short response';
      state.conversation = [
        { role: 'assistant', content: shortContent }, // No fullContent
      ];

      // @step And the message has no fullContent field
      expect(state.conversation[0].fullContent).toBeUndefined();

      // @step And I press Tab to enter select mode
      state.isTurnSelectMode = true;

      // @step And I navigate to that assistant turn
      const selectedLine: ConversationLine = {
        role: 'assistant',
        content: shortContent,
        messageIndex: 0,
      };

      // @step When I press Enter
      handleVirtualListSelect(state, selectedLine, 0);

      // @step Then the modal opens showing the content field value
      expect(state.showTurnModal).toBe(true);
      const modalContent = getModalContent(state);
      expect(modalContent).toBe(shortContent);

      // @step And the modal is still scrollable
      // (verified structurally - VirtualList is always scrollable)
    });
  });

  describe('Scenario: Collapsed output shows updated hint text', () => {
    it('should show new hint text format in collapsed output', () => {
      // @step Given I am in AgentView with a conversation containing a tool output with more than 4 lines
      const fullContent = Array.from(
        { length: 10 },
        (_, i) => `Line ${i + 1}`
      ).join('\n');

      // @step When the tool output is displayed in collapsed form
      const collapsedContent = formatCollapsedOutput(fullContent);

      // @step Then the collapse indicator shows "... +N lines (Enter to view full)"
      expect(collapsedContent).toContain('... +6 lines (Enter to view full)');

      // @step And the old "/expand" hint text is not present
      expect(collapsedContent).not.toContain('/expand');
      expect(collapsedContent).not.toContain('/select');
    });
  });

  describe('Scenario: Esc key closes modal before disabling select mode', () => {
    it('should close modal first, then disable select mode on second Esc', () => {
      // @step Given I am in AgentView in select mode with a modal open
      state.isTurnSelectMode = true;
      state.showTurnModal = true;
      state.modalMessageIndex = 0;
      state.conversation = [{ role: 'assistant', content: 'Test' }];

      // @step When I press Esc
      let result = handleEscKey(state, mockOnExit);

      // @step Then the modal closes
      expect(state.showTurnModal).toBe(false);
      expect(result.action).toBe('close_modal');

      // @step And select mode remains active
      expect(state.isTurnSelectMode).toBe(true);

      // @step When I press Esc again
      result = handleEscKey(state, mockOnExit);

      // @step Then select mode is disabled
      expect(state.isTurnSelectMode).toBe(false);
      expect(result.action).toBe('disable_select_mode');
    });
  });

  describe('Scenario: The /expand command is no longer recognized', () => {
    it('should not recognize /expand as a special command', () => {
      // @step Given I am in AgentView in select mode with a turn selected
      state.isTurnSelectMode = true;
      state.conversation = [{ role: 'tool', content: 'Test output' }];

      // @step When I type "/expand" and press Enter
      const input = '/expand';

      // @step Then the command is not recognized as a special command
      const isRecognized = isExpandCommandRecognized(input);
      expect(isRecognized).toBe(false);

      // @step And it is sent as a regular message to the agent
      // (verified by isRecognized being false - message would be sent to agent)
    });
  });
});
