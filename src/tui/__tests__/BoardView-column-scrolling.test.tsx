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
    id: `TEST-${i + 1}`.padStart(8, '0'),
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

      // @step And I am at item 1
      const { lastFrame, stdin } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0}
          onColumnChange={mockColumnChange}
          onWorkUnitChange={mockWorkUnitChange}
          onEnter={mockEnter}
        />
      );

      // @step When I press down arrow 11 times
      // This test SHOULD FAIL initially because scroll logic doesn't exist yet

      // Simulate down arrow presses by checking output
      // (This will fail because scrolling isn't implemented)
      const output = lastFrame();

      // @step Then the selection should move to item 11
      // @step And the viewport should scroll to show items 2-11
      // @step And item 11 should be visible and selected

      // THIS TEST WILL FAIL - proving we need to implement scrolling
      expect(output).toContain('TEST-011'); // Item 11 should be visible
      expect(output).not.toContain('TEST-001'); // Item 1 should be scrolled out
    });
  });

  describe('Scenario: Scroll up when navigating backward through items', () => {
    it('should scroll viewport up when navigating to earlier items', () => {
      // Given I am viewing a column with 20 work items
      const workUnits = createMockWorkUnits(20, 'backlog');

      // And I am at item 15
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={14} // Item 15 (0-indexed)
          onColumnChange={vi.fn()}
          onWorkUnitChange={vi.fn()}
          onEnter={vi.fn()}
        />
      );

      // When I press up arrow 10 times to reach item 5
      // Then the viewport should scroll up to show items 1-10
      // And item 5 should be visible and selected

      const output = lastFrame();

      // THIS TEST WILL FAIL - need to implement scroll-up logic
      expect(output).toContain('TEST-005'); // Item 5 should be visible
      expect(output).toContain('TEST-001'); // Item 1 should be visible (scrolled to top)
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

      // Then no scroll indicators should be displayed
      expect(output).not.toContain('↑');
      expect(output).not.toContain('↓');

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

  describe('Scenario: Wrap-around navigation at column boundaries', () => {
    it('should wrap to first item when pressing down at last item', () => {
      // Given I am viewing a column with 10 work units
      const workUnits = createMockWorkUnits(10, 'backlog');

      // And I am at the last item (item 10)
      const mockWorkUnitChange = vi.fn();
      render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={9} // Last item (0-indexed)
          onColumnChange={vi.fn()}
          onWorkUnitChange={mockWorkUnitChange}
          onEnter={vi.fn()}
        />
      );

      // When I press down arrow
      // (Test for wrap-around logic)

      // Then the selection should wrap to the first item (item 1)
      // THIS TEST WILL FAIL - wrap-around not implemented in BoardView yet
      expect(mockWorkUnitChange).toHaveBeenCalledWith(1); // Should wrap
    });

    it('should wrap to last item when pressing up at first item', () => {
      // Given I am at the first item (item 1)
      const workUnits = createMockWorkUnits(10, 'backlog');
      const mockWorkUnitChange = vi.fn();

      render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={0} // First item
          onColumnChange={vi.fn()}
          onWorkUnitChange={mockWorkUnitChange}
          onEnter={vi.fn()}
        />
      );

      // When I press up arrow
      // Then the selection should wrap to the last item (item 10)
      // THIS TEST WILL FAIL - wrap-around not implemented
      expect(mockWorkUnitChange).toHaveBeenCalledWith(-1); // Should wrap
    });
  });

  describe('Scenario: Account for scroll indicators when calculating visible items', () => {
    it('should keep selected item visible when arrows consume viewport rows', () => {
      // @step Given I am viewing a column with 15 work items
      const workUnits = createMockWorkUnits(15, 'backlog');

      // @step And the viewport height is 10 items
      // (VIEWPORT_HEIGHT = 10 in UnifiedBoardLayout)

      // @step And I am at item 8 (middle of the list)
      const { lastFrame } = render(
        <UnifiedBoardLayout
          workUnits={workUnits}
          focusedColumnIndex={0}
          selectedWorkUnitIndex={7} // Item 8 (0-indexed)
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

      // THIS TEST WILL FAIL - arrows consume rows but selected item calculation doesn't account for this
      // Selected item (TEST-008) should be visible despite arrows
      expect(output).toContain('TEST-008');

      // Count how many items are actually visible (should be 8 with both arrows)
      const visibleItems = output.match(/TEST-\d+/g) || [];
      expect(visibleItems.length).toBeLessThanOrEqual(8); // Not 10!
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

      const outputAfterReturn = lastFrame();

      // Then the scroll offset should still be 10
      // And item 15 should still be selected
      // THIS TEST WILL FAIL - scroll position not preserved per column yet
      expect(outputAfterReturn).toEqual(outputBeforeSwitch);
    });
  });
});
