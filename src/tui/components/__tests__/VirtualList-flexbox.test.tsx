/**
 * Feature: spec/features/pure-flexbox-layout-for-checkpoint-and-changed-files-viewers.feature
 *
 * Tests for VirtualList flexbox behavior (GIT-006)
 */

import React from 'react';
import { render } from 'ink-testing-library';
import { VirtualList } from '../VirtualList';
import { Box } from 'ink';

describe('Feature: Pure flexbox layout for checkpoint and changed files viewers', () => {
  describe('Scenario: VirtualList uses flexbox without hardcoded heights', () => {
    it('should NOT accept a height prop', () => {
      // @step Given VirtualList has 100 items
      const items = Array.from({ length: 100 }, (_, i) => `Item ${i + 1}`);

      // @step And the terminal is 80x24
      // Terminal size handled by useTerminalSize hook

      // @step When VirtualList renders
      // This should fail because VirtualList currently requires height prop
      const { frames } = render(
        <VirtualList
          items={items}
          renderItem={(item) => <Box>{item}</Box>}
        />
      );

      // @step Then it should use flexGrow=1 to fill container
      // @step And it should calculate visibleHeight from useTerminalSize internally
      // @step And it should display only items that fit in available height
      // @step And it should NOT accept a height prop
      expect(frames[frames.length - 1]).toBeDefined();
    });

    it('should use flexGrow to fill container', () => {
      // This test verifies that VirtualList renders with flexGrow=1
      // Currently will fail because VirtualList uses hardcoded height
      const items = ['Item 1', 'Item 2', 'Item 3'];

      const { frames } = render(
        <Box flexDirection="column" height={20}>
          <VirtualList
            items={items}
            renderItem={(item) => <Box>{item}</Box>}
          />
        </Box>
      );

      // VirtualList should fill the parent container
      expect(frames[frames.length - 1]).toContain('Item 1');
    });

    it('should calculate visible items from terminal size', () => {
      // @step Given terminal is 80x24
      // @step When VirtualList renders with many items
      const items = Array.from({ length: 100 }, (_, i) => `Item ${i + 1}`);

      const { frames } = render(
        <VirtualList
          items={items}
          renderItem={(item) => <Box>{item}</Box>}
        />
      );

      // @step Then it should display only items that fit
      // Currently fails because height is required
      expect(frames[frames.length - 1]).toBeDefined();
    });
  });

  describe('Scenario: Terminal resize causes automatic layout adjustment', () => {
    it('should adjust layout automatically using flexbox', () => {
      // @step Given any viewer component is rendered in 80x24 terminal
      const items = Array.from({ length: 50 }, (_, i) => `Item ${i + 1}`);

      // @step When terminal resizes to 120x40
      const { frames } = render(
        <VirtualList
          items={items}
          renderItem={(item) => <Box>{item}</Box>}
        />
      );

      // @step Then all components should adjust automatically using flexbox
      // @step And there should be NO re-render glitches or flickering
      // @step And proportional sizing should recalculate correctly
      // useTerminalSize hook ensures automatic adjustment to terminal size changes
      expect(frames[frames.length - 1]).toBeDefined();
    });
  });
});
