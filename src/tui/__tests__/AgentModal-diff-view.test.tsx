/**
 * Feature: spec/features/diff-view-for-write-edit-tool-output.feature
 *
 * Tests for Diff View for Write/Edit Tool Output (TUI-038)
 *
 * These tests verify that Edit and Write tool results display with colored
 * diff output showing removed lines in red and added lines in green.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { computeLineDiff, changesToDiffLines, type DiffLine } from '../../git/diff-parser';

// Color constants matching FileDiffViewer
const DIFF_COLORS = {
  removed: '#8B0000', // Dark red
  added: '#006400',   // Dark green
};

// Helper to format tool header in Claude Code style
function formatToolHeader(toolName: string, args: string): string {
  return `● ${toolName}(${args})`;
}

// Helper to format with tree connectors: L on first line, indent on rest
function formatWithTreeConnectors(content: string): string {
  const lines = content.split('\n');
  return lines.map((line, i) => {
    if (i === 0) return `L ${line}`;  // First line gets L prefix
    return `  ${line}`;                // Subsequent lines get indent
  }).join('\n');
}

// Helper to format collapsed output
function formatCollapsedOutput(
  lines: string[],
  visibleLines: number = 4
): string {
  if (lines.length <= visibleLines) {
    return lines.join('\n');
  }
  const visible = lines.slice(0, visibleLines);
  const remaining = lines.length - visibleLines;
  return `${visible.join('\n')}\n... +${remaining} lines (ctrl+o to expand)`;
}

/**
 * Format diff lines for tool output display
 * This is the function that will be implemented in AgentModal.tsx
 */
function formatDiffOutput(diffLines: DiffLine[]): { content: string; colors: Map<number, string> } {
  const colors = new Map<number, string>();
  const contentLines: string[] = [];

  for (let i = 0; i < diffLines.length; i++) {
    const line = diffLines[i];
    contentLines.push(line.content);

    if (line.type === 'removed') {
      colors.set(i, DIFF_COLORS.removed);
    } else if (line.type === 'added') {
      colors.set(i, DIFF_COLORS.added);
    }
  }

  return { content: contentLines.join('\n'), colors };
}

/**
 * Generate diff for Edit tool (old_string -> new_string replacement)
 */
function generateEditDiff(oldString: string, newString: string): DiffLine[] {
  const changes = computeLineDiff(oldString, newString);
  return changesToDiffLines(changes);
}

/**
 * Generate diff for Write tool (new file = all additions)
 */
function generateWriteDiff(content: string): DiffLine[] {
  const lines = content.split('\n');
  return lines.map(line => ({
    content: `+${line}`,
    type: 'added' as const,
    changeGroup: 'addition' as const,
  }));
}

describe('Feature: Diff View for Write/Edit Tool Output', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Scenario: Edit tool shows single line replacement with diff colors', () => {
    it('should show removed line with red background and added line with green background', () => {
      // @step Given the agent executes an Edit tool replacing 'const x = 1' with 'const x = 2'
      const oldString = 'const x = 1';
      const newString = 'const x = 2';

      // @step When the tool result is displayed in the TUI
      const diffLines = generateEditDiff(oldString, newString);
      const { content, colors } = formatDiffOutput(diffLines);

      // @step Then the removed line shows '-const x = 1' with red background and the added line shows '+const x = 2' with green background
      expect(content).toContain('-const x = 1');
      expect(content).toContain('+const x = 2');

      // Find indices of removed and added lines
      const lines = content.split('\n');
      const removedIdx = lines.findIndex(l => l.startsWith('-'));
      const addedIdx = lines.findIndex(l => l.startsWith('+'));

      expect(colors.get(removedIdx)).toBe(DIFF_COLORS.removed);
      expect(colors.get(addedIdx)).toBe(DIFF_COLORS.added);
    });
  });

  describe('Scenario: Write tool shows new file content as additions', () => {
    it('should show all lines with + prefix and green background', () => {
      // @step Given the agent executes a Write tool creating a new file with 3 lines of content
      const newFileContent = 'line 1\nline 2\nline 3';

      // @step When the tool result is displayed in the TUI
      const diffLines = generateWriteDiff(newFileContent);
      const { content, colors } = formatDiffOutput(diffLines);

      // @step Then all 3 lines are shown with '+' prefix and green background
      const lines = content.split('\n');
      expect(lines.length).toBe(3);
      expect(lines[0]).toBe('+line 1');
      expect(lines[1]).toBe('+line 2');
      expect(lines[2]).toBe('+line 3');

      // All lines should have green background
      expect(colors.get(0)).toBe(DIFF_COLORS.added);
      expect(colors.get(1)).toBe(DIFF_COLORS.added);
      expect(colors.get(2)).toBe(DIFF_COLORS.added);
    });
  });

  describe('Scenario: Edit tool shows multi-line replacement with grouped changes', () => {
    it('should group removed lines together followed by added lines', () => {
      // @step Given the agent executes an Edit tool replacing 3 lines with 2 new lines
      const oldString = 'line A\nline B\nline C';
      const newString = 'new line 1\nnew line 2';

      // @step When the tool result is displayed in the TUI
      const diffLines = generateEditDiff(oldString, newString);
      const { content, colors } = formatDiffOutput(diffLines);

      // @step Then the 3 removed lines are grouped together with red background followed by the 2 added lines grouped together with green background
      const lines = content.split('\n');

      // Find all removed and added lines
      const removedLines = lines.filter(l => l.startsWith('-'));
      const addedLines = lines.filter(l => l.startsWith('+'));

      expect(removedLines.length).toBe(3);
      expect(addedLines.length).toBe(2);

      // Verify removed lines have red background
      let removedCount = 0;
      let addedCount = 0;
      lines.forEach((line, idx) => {
        if (line.startsWith('-')) {
          expect(colors.get(idx)).toBe(DIFF_COLORS.removed);
          removedCount++;
        }
        if (line.startsWith('+')) {
          expect(colors.get(idx)).toBe(DIFF_COLORS.added);
          addedCount++;
        }
      });

      // Confirm all removed and added lines have correct colors
      expect(removedCount).toBe(3);
      expect(addedCount).toBe(2);
    });
  });

  describe('Scenario: Long diff output is collapsed with expand indicator', () => {
    it('should show first 4 lines and collapse indicator for long diff', () => {
      // @step Given the agent executes an Edit tool with a 100+ line diff
      const oldLines = Array.from({ length: 100 }, (_, i) => `old line ${i + 1}`);
      const newLines = Array.from({ length: 100 }, (_, i) => `new line ${i + 1}`);
      const oldString = oldLines.join('\n');
      const newString = newLines.join('\n');

      // @step When the tool result is displayed in the TUI
      const diffLines = generateEditDiff(oldString, newString);
      const { content } = formatDiffOutput(diffLines);
      const allLines = content.split('\n');
      const collapsed = formatCollapsedOutput(allLines, 4);

      // @step Then the first 4 lines of the diff are visible followed by '... +96 lines (ctrl+o to expand)' indicator
      const collapsedLines = collapsed.split('\n');
      expect(collapsedLines.length).toBe(5); // 4 visible + 1 indicator

      // First 4 should be diff lines (either - or + prefixed)
      expect(collapsedLines[0]).toMatch(/^[-+]/);
      expect(collapsedLines[1]).toMatch(/^[-+]/);
      expect(collapsedLines[2]).toMatch(/^[-+]/);
      expect(collapsedLines[3]).toMatch(/^[-+]/);

      // Total diff lines minus 4 visible
      const remaining = allLines.length - 4;
      expect(collapsed).toContain(`... +${remaining} lines (ctrl+o to expand)`);
    });
  });

  describe('Scenario: Diff output uses tree connector pattern', () => {
    it('should use L prefix on first line and indent on subsequent lines', () => {
      // @step Given the agent executes an Edit tool with a multi-line diff
      const oldString = 'const x = 1';
      const newString = 'const x = 2';

      // @step When the tool result is displayed in the TUI
      const diffLines = generateEditDiff(oldString, newString);
      const { content } = formatDiffOutput(diffLines);
      const formatted = formatWithTreeConnectors(content);

      // @step Then the first diff line has 'L ' prefix and subsequent lines have two-space indent
      const lines = formatted.split('\n');
      expect(lines[0]).toMatch(/^L /);

      for (let i = 1; i < lines.length; i++) {
        expect(lines[i]).toMatch(/^  /);
      }
    });
  });
});
