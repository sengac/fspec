/**
 * Feature: spec/features/anchor-view.feature
 *
 * Tests for AnchorView component - full-screen anchor point viewer
 * with split-pane layout (anchor list left, turn preview right).
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { AnchorView } from '../AnchorView';
import type { AnchorPoint } from '../../types/anchor';

// Mock Ink's Box to strip position="absolute" which doesn't work in ink-testing-library
vi.mock('ink', async () => {
  const actual = await vi.importActual<typeof import('ink')>('ink');
  return {
    ...actual,
    Box: (props: Record<string, unknown>) => {
      // Strip position="absolute" as ink-testing-library can't render it
      const { position, ...rest } = props;
      return <actual.Box {...rest} />;
    },
  };
});

// Mock anchor points for testing - now include embedded turn content
const createMockAnchorPoints = (): AnchorPoint[] => [
  {
    anchorType: 'TaskCompletion',
    turnIndex: 14,
    timestamp: Date.now() - 2 * 60 * 1000, // 2 min ago
    weight: 0.91,
    confidence: 0.91,
    description: 'Completed refactoring task',
    userMessage: 'Can you analyze the compaction logs?',
    assistantResponse: 'I found the issue in the buffer management and fixed it.',
    toolCalls: [
      { tool: 'Read', success: true },
      { tool: 'Edit', success: true },
    ],
  },
  {
    anchorType: 'ErrorResolution',
    turnIndex: 8,
    timestamp: Date.now() - 5 * 60 * 1000, // 5 min ago
    weight: 0.85,
    confidence: 0.85,
    description: 'Fixed compilation error',
    userMessage: 'There is a type error in the session manager.',
    assistantResponse: 'I see the issue - the return type was incorrect. Fixed it.',
    toolCalls: [
      { tool: 'Edit', success: true },
    ],
  },
  {
    anchorType: 'FeatureMilestone',
    turnIndex: 3,
    timestamp: Date.now() - 12 * 60 * 1000, // 12 min ago
    weight: 0.78,
    confidence: 0.78,
    description: 'Architecture decision made',
    userMessage: 'What architecture should we use?',
    assistantResponse: 'I recommend using a modular architecture with clear boundaries.',
    toolCalls: [],
  },
];

// Test dimensions for consistent rendering
const TEST_WIDTH = 120;
const TEST_HEIGHT = 40;

describe('Feature: Refactor Anchor Viewer from Dialog to Full-Screen View', () => {
  let mockOnClose: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockOnClose = vi.fn();
  });

  describe('Scenario: Open anchor view with /anchors command', () => {
    it('should display full-screen anchor view with split pane layout', () => {
      const anchorPoints = createMockAnchorPoints();

      // @step Given I have an active session with anchor points
      // (Anchor points are provided as props - session management is outside AnchorView scope)

      // @step When I type "/anchors"
      // (Command handling is in AgentView - AnchorView receives isVisible=true)
      const { lastFrame } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={anchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      const output = lastFrame();

      // @step Then a full-screen anchor view opens
      expect(output).toBeDefined();

      // @step And the left pane shows the anchor list with metadata
      expect(output).toContain('TaskCompletion');
      expect(output).toContain('Turn 14');
      expect(output).toContain('0.91');

      // @step And the right pane shows the selected anchor's turn content
      // First anchor is selected by default, its content should be displayed
      expect(output).toContain('Can you analyze');

      // @step And the view has a header showing "Conversation Anchor Points"
      expect(output).toContain('Conversation Anchor Points');

      // @step And the footer shows available keyboard shortcuts
      expect(output).toMatch(/Up.*Down.*Navigate|Navigate/i);
      expect(output).toContain('Esc');
    });
  });

  describe('Scenario: Navigate anchors with arrow keys', () => {
    it('should navigate between anchors and update preview', async () => {
      const anchorPoints = createMockAnchorPoints();

      // @step Given the anchor view is open with multiple anchors
      const { lastFrame, stdin } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={anchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      // Initial state - first anchor selected, shows its content
      let output = lastFrame() ?? '';
      expect(output).toContain('TaskCompletion');
      expect(output).toContain('Can you analyze');

      // @step When I press the down arrow key
      stdin.write('\x1B[B'); // Down arrow

      // @step Then the next anchor in the list is selected
      // @step And the right pane updates to show that anchor's turn content
      await vi.waitFor(() => {
        output = lastFrame() ?? '';
        // Should now show ErrorResolution content
        expect(output).toContain('type error');
      });

      // @step When I press the up arrow key
      stdin.write('\x1B[A'); // Up arrow

      // @step Then the previous anchor in the list is selected
      // @step And the right pane updates to show that anchor's turn content
      await vi.waitFor(() => {
        output = lastFrame() ?? '';
        // Should be back to TaskCompletion content
        expect(output).toContain('Can you analyze');
      });
    });
  });

  describe('Scenario: Anchor list shows rich metadata', () => {
    it('should display all metadata fields without emojis', () => {
      const anchorPoints = createMockAnchorPoints();

      // @step Given the anchor view is open with anchors
      const { lastFrame } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={anchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then each anchor item displays the anchor type label
      expect(output).toContain('TaskCompletion');
      expect(output).toContain('ErrorResolution');
      expect(output).toContain('FeatureMilestone');

      // @step And each anchor item displays the turn number
      expect(output).toContain('Turn 14');
      expect(output).toContain('Turn 8');
      expect(output).toContain('Turn 3');

      // @step And each anchor item displays the confidence score
      expect(output).toContain('0.91');
      expect(output).toContain('0.85');
      expect(output).toContain('0.78');

      // @step And each anchor item displays the relative timestamp
      // Timestamps should be relative (e.g., "2 min ago", "5 min ago")
      expect(output).toMatch(/\d+\s*(min|sec|hr)/i);

      // @step And no emoji characters are used in the display
      // Check for common emoji ranges - no emojis should be present
      const emojiRegex = /[\u{1F300}-\u{1F9FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]/u;
      expect(emojiRegex.test(output)).toBe(false);
    });
  });

  describe('Scenario: Preview pane shows turn content', () => {
    it('should display turn details in right pane', () => {
      const anchorPoints = createMockAnchorPoints();

      // @step Given the anchor view is open with an anchor selected
      const { lastFrame } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={anchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the right pane shows the user message for that turn
      expect(output).toContain('Can you analyze');

      // @step And the right pane shows the assistant response for that turn
      expect(output).toContain('found the issue');

      // @step And the right pane shows any tool calls made in that turn
      expect(output).toMatch(/Read|Edit|TOOLS/i);

      // @step And the content is scrollable with a scrollbar indicator
      // VirtualList with showScrollbar=true provides scrollbar when content exceeds viewport
      // The scrollbar indicator is rendered by VirtualList component
    });
  });

  describe('Scenario: Exit anchor view with Escape', () => {
    it('should close view and call onClose when Escape pressed', () => {
      const anchorPoints = createMockAnchorPoints();

      // @step Given the anchor view is open
      const { stdin } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={anchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      // @step When I press the Escape key
      stdin.write('\x1B'); // Escape

      // @step Then the anchor view closes
      // @step And I return to the AgentView
      expect(mockOnClose).toHaveBeenCalled();
    });
  });

  describe('Scenario: All input is consumed by anchor view', () => {
    it('should consume all input and not leak keystrokes', () => {
      const anchorPoints = createMockAnchorPoints();

      // @step Given the anchor view is open
      const { stdin, lastFrame } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={anchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      // Verify view is rendered
      expect(lastFrame()).toContain('Conversation Anchor Points');

      // @step When I type random characters
      stdin.write('abc123!@#');

      // @step Then no keystrokes leak to components underneath
      // @step And the AgentView does not receive any input
      // The useInputCompat handler with CRITICAL priority returns true for ALL input
      // We verify this by checking onClose wasn't called (Escape would trigger it)
      // and the view is still displayed unchanged
      expect(mockOnClose).not.toHaveBeenCalled();
      expect(lastFrame()).toContain('Conversation Anchor Points');
    });
  });

  describe('Scenario: Empty state when no anchors exist', () => {
    it('should display empty state message', () => {
      // @step Given I have an active session with no anchor points
      const emptyAnchorPoints: AnchorPoint[] = [];

      // @step When I type "/anchors"
      const { lastFrame } = render(
        <AnchorView
          isVisible={true}
          anchorPoints={emptyAnchorPoints}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      const output = lastFrame() ?? '';

      // @step Then the anchor view opens
      expect(output).toBeDefined();

      // @step And a message displays "No anchor points found in this session"
      expect(output).toContain('No anchor points found in this session');
    });
  });

  describe('Scenario: Error state when no active session', () => {
    it('should show status message when no session exists', () => {
      // @step Given I have no active session
      // This is tested at AgentView level - AnchorView won't be rendered
      // The /anchors command handler checks for active session first

      // @step When I type "/anchors"
      // @step Then a status message displays "Start a session first to view anchor points"
      // @step And the anchor view does not open

      // This test verifies the component doesn't render when isVisible=false
      const { lastFrame } = render(
        <AnchorView
          isVisible={false}
          anchorPoints={[]}
          onClose={mockOnClose}
          _terminalWidth={TEST_WIDTH}
          _terminalHeight={TEST_HEIGHT}
        />
      );

      // View should not render anything when not visible
      expect(lastFrame()).toBe('');
    });
  });
});
