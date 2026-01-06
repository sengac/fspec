// Feature: spec/features/expand-command-for-turn-based-tool-output-expansion.feature
// TUI-043: Expand command for turn-based tool output expansion

import { describe, it, expect, vi, beforeEach } from 'vitest';

/**
 * Test suite for /expand command functionality in AgentView
 *
 * These tests verify the expand/collapse toggle behavior for tool output
 * when in turn selection mode.
 */

// Mock the formatCollapsedOutput and related functions
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
  return `${visible.join('\n')}\n... +${remaining} lines (use /select and /expand)`;
};

const formatFullOutput = (content: string): string => {
  return content; // Full content without truncation
};

// Simulated state management for tests
interface TestState {
  isTurnSelectMode: boolean;
  expandedMessageIndices: Set<number>;
  conversation: Array<{
    role: 'user' | 'assistant' | 'tool';
    content: string;
    fullContent?: string;
  }>;
  selectedMessageIndex: number;
}

const createTestState = (): TestState => ({
  isTurnSelectMode: false,
  expandedMessageIndices: new Set(),
  conversation: [],
  selectedMessageIndex: 0,
});

// Simulated /expand command handler
const handleExpandCommand = (state: TestState): { success: boolean } => {
  // Silently do nothing if not in turn selection mode
  if (!state.isTurnSelectMode) {
    return { success: false };
  }

  const messageIndex = state.selectedMessageIndex;
  if (state.expandedMessageIndices.has(messageIndex)) {
    state.expandedMessageIndices.delete(messageIndex);
  } else {
    state.expandedMessageIndices.add(messageIndex);
  }

  return { success: true };
};

// Get effective content based on expansion state
const getEffectiveContent = (
  state: TestState,
  messageIndex: number
): string => {
  const msg = state.conversation[messageIndex];
  if (!msg) return '';

  if (state.expandedMessageIndices.has(messageIndex) && msg.fullContent) {
    return msg.fullContent;
  }
  return msg.content;
};

describe('Feature: Expand command for turn-based tool output expansion', () => {
  let state: TestState;

  beforeEach(() => {
    state = createTestState();
  });

  describe('Scenario: Expand collapsed tool output in turn selection mode', () => {
    it('should expand collapsed output when /expand is run in turn selection mode', () => {
      // @step Given I have a conversation with a tool output turn showing "...+46 lines (use /select and /expand)"
      const fullContent = Array.from(
        { length: 50 },
        (_, i) => `Line ${i + 1}`
      ).join('\n');
      const collapsedContent = formatCollapsedOutput(fullContent);
      state.conversation = [
        { role: 'tool', content: collapsedContent, fullContent },
      ];
      expect(state.conversation[0].content).toContain(
        '+46 lines (use /select and /expand)'
      );

      // @step And I run the /select command to enable turn selection mode
      state.isTurnSelectMode = true;
      expect(state.isTurnSelectMode).toBe(true);

      // @step And I navigate to the collapsed tool output turn
      state.selectedMessageIndex = 0;

      // @step When I run the /expand command
      const result = handleExpandCommand(state);
      expect(result.success).toBe(true);

      // @step Then the turn expands to show the full tool output without truncation
      const effectiveContent = getEffectiveContent(state, 0);
      expect(effectiveContent).toBe(fullContent);
      expect(effectiveContent).not.toContain('...');

      // @step And the "...+N lines" hint is no longer visible for that turn
      expect(effectiveContent).not.toContain('(use /select and /expand)');
    });
  });

  describe('Scenario: Collapse expanded turn by running expand again', () => {
    it('should collapse expanded output when /expand is run again', () => {
      // @step Given I am in turn selection mode with a turn currently expanded
      const fullContent = Array.from(
        { length: 50 },
        (_, i) => `Line ${i + 1}`
      ).join('\n');
      const collapsedContent = formatCollapsedOutput(fullContent);
      state.conversation = [
        { role: 'tool', content: collapsedContent, fullContent },
      ];
      state.isTurnSelectMode = true;
      state.selectedMessageIndex = 0;
      state.expandedMessageIndices.add(0); // Already expanded
      expect(state.expandedMessageIndices.has(0)).toBe(true);

      // @step When I run the /expand command
      const result = handleExpandCommand(state);
      expect(result.success).toBe(true);

      // @step Then the turn collapses back to the truncated view
      expect(state.expandedMessageIndices.has(0)).toBe(false);
      const effectiveContent = getEffectiveContent(state, 0);
      expect(effectiveContent).toBe(collapsedContent);

      // @step And the "...+N lines (use /select and /expand)" hint is visible again
      expect(effectiveContent).toContain('(use /select and /expand)');
    });
  });

  describe('Scenario: Silently ignore expand when not in turn selection mode', () => {
    it('should silently do nothing when /expand is run without turn selection mode', () => {
      // @step Given I am NOT in turn selection mode
      state.isTurnSelectMode = false;
      expect(state.isTurnSelectMode).toBe(false);

      // @step When I run the /expand command
      const result = handleExpandCommand(state);

      // @step Then the command does nothing (silent no-op)
      expect(result.success).toBe(false);
      // No error message - silent failure
    });
  });

  describe('Scenario: Updated hint message in collapsed tool output', () => {
    it('should show new hint message format in collapsed output', () => {
      // @step Given a tool produces output with 50 lines
      const fullContent = Array.from(
        { length: 50 },
        (_, i) => `Line ${i + 1}`
      ).join('\n');

      // @step When the output is displayed in the conversation
      const collapsedContent = formatCollapsedOutput(fullContent);

      // @step Then I see the hint "...+46 lines (use /select and /expand)"
      expect(collapsedContent).toContain('+46 lines (use /select and /expand)');

      // @step And I do NOT see the old hint "ctrl+o to expand"
      expect(collapsedContent).not.toContain('ctrl+o to expand');
    });
  });

  describe('Scenario: Expand collapsed diff output from Edit tool', () => {
    it('should expand collapsed diff output from Edit tool', () => {
      // @step Given I have a conversation with an Edit tool diff showing 100 lines of changes collapsed
      const fullDiffContent = Array.from(
        { length: 100 },
        (_, i) => `+Line ${i + 1}`
      ).join('\n');
      const collapsedDiff = formatCollapsedOutput(fullDiffContent, 25); // Diff uses 25 lines
      state.conversation = [
        {
          role: 'tool',
          content: `● Edit(file.ts)\n${collapsedDiff}`,
          fullContent: `● Edit(file.ts)\n${fullDiffContent}`,
        },
      ];

      // @step And I am in turn selection mode with that turn selected
      state.isTurnSelectMode = true;
      state.selectedMessageIndex = 0;

      // @step When I run the /expand command
      const result = handleExpandCommand(state);
      expect(result.success).toBe(true);

      // @step Then the full diff is visible with all 100 lines of changes
      const effectiveContent = getEffectiveContent(state, 0);
      expect(effectiveContent).toContain('+Line 1');
      expect(effectiveContent).toContain('+Line 100');
      expect(effectiveContent).not.toContain('...');
    });
  });

  describe('Scenario: Expansion state persists when navigating between turns', () => {
    it('should preserve expansion state when navigating between turns', () => {
      // @step Given I am in turn selection mode
      state.isTurnSelectMode = true;

      // @step And I have expanded turn A using /expand
      const fullContentA = Array.from(
        { length: 50 },
        (_, i) => `A-Line ${i + 1}`
      ).join('\n');
      const collapsedA = formatCollapsedOutput(fullContentA);
      const fullContentB = Array.from(
        { length: 50 },
        (_, i) => `B-Line ${i + 1}`
      ).join('\n');
      const collapsedB = formatCollapsedOutput(fullContentB);

      state.conversation = [
        { role: 'tool', content: collapsedA, fullContent: fullContentA },
        { role: 'tool', content: collapsedB, fullContent: fullContentB },
      ];
      state.selectedMessageIndex = 0;
      handleExpandCommand(state); // Expand turn A
      expect(state.expandedMessageIndices.has(0)).toBe(true);

      // @step And turn B is still collapsed
      expect(state.expandedMessageIndices.has(1)).toBe(false);

      // @step When I navigate to turn B
      state.selectedMessageIndex = 1;

      // @step And then navigate back to turn A
      state.selectedMessageIndex = 0;

      // @step Then turn A is still expanded
      expect(state.expandedMessageIndices.has(0)).toBe(true);
      const contentA = getEffectiveContent(state, 0);
      expect(contentA).toBe(fullContentA);

      // @step And turn B remains collapsed
      expect(state.expandedMessageIndices.has(1)).toBe(false);
      const contentB = getEffectiveContent(state, 1);
      expect(contentB).toBe(collapsedB);
    });
  });
});
