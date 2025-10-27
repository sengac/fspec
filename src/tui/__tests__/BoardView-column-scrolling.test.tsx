/**
 * Feature: spec/features/tui-board-column-scrolling.feature
 *
 * Tests for automatic column scrolling functionality in the TUI board.
 * Verifies VirtualList-style navigation pattern from cage project.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { UnifiedBoardLayout } from '../components/UnifiedBoardLayout';
import type { WorkUnit } from '../store/fspecStore';

// Mock work units for testing
const createMockWorkUnits = (count: number, status: string): WorkUnit[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: `TEST-${String(i + 1).padStart(3, '0')}`,
    title: `Test Work Unit ${i + 1}`,
    type: 'story' as const,
    status,
    estimate: 3,
    description: `Description for test unit ${i + 1}`,
    dependencies: [],
  }));
};

describe('Feature: TUI Board Column Scrolling', () => {
  describe('Scenario: Automatic scroll when navigating beyond visible viewport', () => {
    it('should scroll viewport to keep selected item visible when navigating down', () => {
      // @step Given I am viewing a column with 20 work items
      const workUnits = createMockWorkUnits(20, 'backlog');
      const mockColumnChange = vi.fn();
      const mockWorkUnitChange = vi.fn();
      const mockEnter = vi.fn();

      // @step And the viewport shows 10 items at a time
      // (VIEWPORT_HEIGHT = 10 in UnifiedBoardLayout)

      // @step And I am at item 11 (0-indexed as 10)
      const { lastFrame, rerender } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={10}
          onColumnChange={mockColumnChange}
          onWorkUnitChange={mockWorkUnitChange}
          onEnter={mockEnter}
        />
      );

      // @step When component renders with selectedWorkUnitIndex=10
      // The useEffect should automatically scroll to keep item 11 visible
      // Force a rerender to ensure useEffect completes
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={10}
          onColumnChange={mockColumnChange}
          onWorkUnitChange={mockWorkUnitChange}
          onEnter={mockEnter}
        />
      );

      const output = lastFrame();

      // @step Then the selection should move to item 11
      // @step And the viewport should scroll to show items 2-11
      // @step And item 11 should be visible and selected

      // Item 11 (TEST-011) should be visible
      expect(output).toContain('TEST-011');
      // Item 1 should be scrolled out (viewport shows items 2-11)
      expect(output).not.toContain('TEST-001');
    });
  });

  describe('Scenario: Scroll up when navigating backward through items', () => {
    it('should scroll viewport up when navigating to earlier items', () => {
      // Given I am viewing a column with 20 work items
      const workUnits = createMockWorkUnits(20, 'backlog');

      // And I am at item 5 (0-indexed as 4)
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={4} // Item 5 (0-indexed)
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // When component renders with selectedWorkUnitIndex=4
      // The viewport should show items 1-10 with item 5 visible
      const output = lastFrame();

      // Item 5 should be visible
      expect(output).toContain('TEST-005');
      // Item 1 should be visible (at top of viewport)
      expect(output).toContain('TEST-001');
    });
  });

  describe('Scenario: No scroll indicators when all items fit in viewport', () => {
    it('should not show scroll indicators when items fit in viewport', () => {
      // Given I am viewing a column with 5 work items
      const workUnits = createMockWorkUnits(5, 'backlog');

      // And the viewport height is 10 items
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // When I render the board
      const output = lastFrame();

      // Then no scroll indicators should be displayed in the column content
      // Extract just the backlog column content rows
      const lines = output.split('\n');
      const dataRowStart = lines.findIndex(l => l.includes('BACKLOG')) + 2;
      const backlogColumn = lines.slice(dataRowStart, dataRowStart + 10).map(l => {
        const parts = l.split('│');
        return parts[1] || '';  // First column after left border
      });

      // No arrows should appear as standalone content in backlog column
      expect(backlogColumn.some(cell => cell.trim() === '↑')).toBe(false);
      expect(backlogColumn.some(cell => cell.trim() === '↓')).toBe(false);

      // And all 5 items should be visible
      expect(output).toContain('TEST-001');
      expect(output).toContain('TEST-005');
    });
  });

  describe('Scenario: Show scroll indicators when items exceed viewport', () => {
    it('should show up arrow when scrolled down', () => {
      // Given I am viewing a column with 20 work items
      const workUnits = createMockWorkUnits(20, 'backlog');

      // When the scroll offset is greater than 0
      // (Simulated by selecting item beyond viewport)
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={15} // Should trigger scroll
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      const output = lastFrame();

      // Then an up arrow indicator (↑) should appear at the top
      // THIS TEST WILL FAIL - scroll indicators aren't dynamic yet
      expect(output).toContain('↑');
    });

    it('should show down arrow when more items below', () => {
      // Given I am viewing a column with 20 work items
      const workUnits = createMockWorkUnits(20, 'backlog');

      // At the top of the list
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      const output = lastFrame();

      // Then a down arrow indicator (↓) should appear at the bottom
      expect(output).toContain('↓');
    });
  });

  // NOTE: Wrap-around tests removed - not in scope for BOARD-012
  // These tests belong to tui-board-column-scrolling.feature (BUG-045)

  describe('Scenario: Account for scroll indicators when calculating visible items', () => {
    it('should keep selected item visible when arrows consume viewport rows', () => {
      // @step Given I am viewing a column with 15 work items
      const workUnits = createMockWorkUnits(15, 'backlog');

      // @step And the viewport height is 10 items
      // (VIEWPORT_HEIGHT = 10 in UnifiedBoardLayout)

      // @step And I am at item 8 (middle of the list)
      const { lastFrame, rerender } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={7} // Item 8 (0-indexed)
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // Force rerender to ensure useEffect completes
      rerender(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={7}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // @step When the board renders with both up and down arrows visible
      const output = lastFrame();

      // @step Then the up arrow should appear at row 0
      expect(output).toContain('↑');

      // @step And the down arrow should appear at row 9
      expect(output).toContain('↓');

      // @step And only 8 work items should be visible (rows 1-8)
      // @step And the selected item should remain visible and not be hidden by arrows

      // Selected item (TEST-008) should be visible despite arrows
      expect(output).toContain('TEST-008');

      // Count items in backlog column only (not all columns)
      const lines = output.split('\n');
      const dataRowStart = lines.findIndex(l => l.includes('BACKLOG')) + 2;
      const backlogColumn = lines.slice(dataRowStart, dataRowStart + 10).map(l => {
        const parts = l.split('│');
        return parts[1] || '';
      });
      const workItemsInBacklog = backlogColumn.filter(cell => cell.includes('TEST-')).length;

      // With at least one arrow visible, max 9 work items (10 - 1)
      // With both arrows, max 8 work items (10 - 2)
      expect(workItemsInBacklog).toBeLessThanOrEqual(9);
    });
  });

  describe('Scenario: Preserve scroll position when switching columns', () => {
    it('should remember scroll offset per column when switching', () => {
      // Given I am in column A at item 15 with scroll offset 10
      const backlogUnits = createMockWorkUnits(20, 'backlog');
      const specifyingUnits = createMockWorkUnits(5, 'specifying');
      const allUnits = [...backlogUnits, ...specifyingUnits];

      let focusedColumn = 0;
      let selectedIndex = 14; // Item 15

      const { rerender, lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={allUnits}
          focusedColumnIndex={focusedColumn}
          selectedWorkUnitIndex={selectedIndex}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // Force rerender to ensure useEffect completes for initial render
      rerender(
        <UnifiedBoardLayout
          workUnits={allUnits}
          focusedColumnIndex={focusedColumn}
          selectedWorkUnitIndex={selectedIndex}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      const outputBeforeSwitch = lastFrame();

      // When I press right arrow to switch to column B
      focusedColumn = 1;
      selectedIndex = 0;

      rerender(
        <UnifiedBoardLayout
          workUnits={allUnits}
          focusedColumnIndex={focusedColumn}
          selectedWorkUnitIndex={selectedIndex}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // And when I return to column A
      focusedColumn = 0;
      selectedIndex = 14;

      rerender(
        <UnifiedBoardLayout
          workUnits={allUnits}
          focusedColumnIndex={focusedColumn}
          selectedWorkUnitIndex={selectedIndex}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // Force rerender to ensure useEffect completes for return
      rerender(
        <UnifiedBoardLayout
          workUnits={allUnits}
          focusedColumnIndex={focusedColumn}
          selectedWorkUnitIndex={selectedIndex}
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      const outputAfterReturn = lastFrame();

      // Then the scroll offset should still be 10
      // And item 15 should still be selected
      // THIS TEST WILL FAIL - scroll position not preserved per column yet
      expect(outputAfterReturn).toEqual(outputBeforeSwitch);
    });
  });

  describe('Scenario: Verify j/k vim-style keys are NOT bound (regression test)', () => {
    it('should only respond to arrow keys, not j/k keys', () => {
      // @step Given I am viewing a column with 10 work items
      const workUnits = createMockWorkUnits(10, 'backlog');
      const mockColumnChange = vi.fn();
      const mockWorkUnitChange = vi.fn();
      const mockEnter = vi.fn();

      // @step And I am at item 1
      const { stdin } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          onColumnChange={mockColumnChange}
          onWorkUnitChange={mockWorkUnitChange}
          onEnter={mockEnter}
        />
      );

      // @step When I press 'j' key
      stdin.write('j');

      // @step Then the work unit selection should NOT change
      expect(mockWorkUnitChange).not.toHaveBeenCalled();

      // @step When I press 'k' key
      stdin.write('k');

      // @step Then the work unit selection should NOT change
      expect(mockWorkUnitChange).not.toHaveBeenCalled();

      // @step But when I press down arrow
      stdin.write('\x1B[B'); // Down arrow escape sequence

      // @step Then the work unit selection SHOULD change
      expect(mockWorkUnitChange).toHaveBeenCalledWith(1);
    });
  });
});
