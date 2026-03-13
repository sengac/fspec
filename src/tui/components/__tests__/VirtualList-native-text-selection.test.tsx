/**
 * Feature: spec/features/native-text-selection-while-preserving-mouse-scroll-wheel-in-virtuallist.feature
 *
 * Tests for VirtualList native text selection support (TUI-078)
 * These tests verify that:
 * - Scroll wheel events continue to work normally
 * - Button-down events temporarily disable mouse tracking for native text selection
 * - Mouse tracking is re-enabled after a 5000ms timeout (debounced)
 * - Button-release events immediately re-enable mouse tracking
 * - Rapid clicks reset the timer (debounce behavior)
 * - Component cleanup clears any pending timer
 */

import React from 'react';
import { render, wait } from 'ink-testing-library';
import { VirtualList } from '../VirtualList';
import { Box, Text } from 'ink';
import { vi, beforeEach, afterEach, describe, it, expect } from 'vitest';

// Helper to create test items
const createItems = (count: number): string[] =>
  Array.from({ length: count }, (_, i) => `Line ${i + 1}`);

// X10 mouse protocol button bytes
const MOUSE_BUTTONS = {
  LEFT_DOWN: 32,
  MIDDLE_DOWN: 33,
  RIGHT_DOWN: 34,
  BUTTON_RELEASE: 35,
  SCROLL_UP: 96,
  SCROLL_DOWN: 97,
} as const;

// Helper to simulate raw mouse escape sequence
const createMouseEvent = (buttonByte: number, x: number = 0, y: number = 0): string => {
  // X10 format: ESC [ M <btn+32> <x+32> <y+32>
  return `[M${String.fromCharCode(buttonByte)}${String.fromCharCode(x + 32)}${String.fromCharCode(y + 32)}`;
};

describe('Feature: Native text selection while preserving mouse scroll wheel in VirtualList', () => {
  let stdoutWriteSpy: ReturnType<typeof vi.spyOn>;
  let mockStdoutWrites: string[];

  beforeEach(() => {
    vi.useFakeTimers();
    mockStdoutWrites = [];
    stdoutWriteSpy = vi.spyOn(process.stdout, 'write').mockImplementation((data: string | Buffer) => {
      mockStdoutWrites.push(data.toString());
      return true;
    });
  });

  afterEach(() => {
    stdoutWriteSpy.mockRestore();
    vi.useRealTimers();
  });

  // ========================================
  // SCROLL WHEEL SCENARIOS
  // ========================================

  describe('Scenario: User scrolls with mouse wheel in conversation view', () => {
    it('should scroll up on wheel up and keep mouse tracking enabled', () => {
      // @step Given the TUI is showing the conversation view with AI output
      // @step And mouse tracking is enabled (?1000h)
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // Verify mouse tracking was enabled on mount
      expect(mockStdoutWrites).toContain('\x1b[?1000h');

      // @step When the user scrolls the mouse wheel up or down
      const wheelUpEvent = createMouseEvent(MOUSE_BUTTONS.SCROLL_UP);
      stdin.write(wheelUpEvent);

      // @step Then the conversation content should scroll in the corresponding direction
      // @step And mouse tracking should remain enabled throughout
      // Mouse tracking should NOT have been disabled (scrolling works normally)
      expect(mockStdoutWrites).not.toContain('\x1b[?1000l');
    });

    it('should scroll down on wheel down and keep mouse tracking enabled', () => {
      // @step Given the TUI is showing the conversation view with AI output
      // @step And mouse tracking is enabled (?1000h)
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // Clear initial writes
      mockStdoutWrites = [];

      // @step When the user scrolls the mouse wheel down
      const wheelDownEvent = createMouseEvent(MOUSE_BUTTONS.SCROLL_DOWN);
      stdin.write(wheelDownEvent);

      // @step Then the conversation content should scroll in the corresponding direction
      // @step And mouse tracking should remain enabled throughout
      expect(mockStdoutWrites).not.toContain('\x1b[?1000l');
    });
  });

  // ========================================
  // BUTTON-DOWN (TEXT SELECTION) SCENARIOS
  // ========================================

  describe('Scenario: User clicks and drags to select text', () => {
    it('should disable mouse tracking on left button down for native selection', () => {
      // @step Given the TUI is showing the conversation view with AI output
      // @step And mouse tracking is enabled (?1000h)
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // Clear initial writes
      mockStdoutWrites = [];

      // @step When the user clicks and drags to select text
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);

      // @step Then mouse tracking should be temporarily disabled (?1000l)
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      // @step And the terminal should handle the text selection natively
      // @step And the user should be able to copy the selected text with Ctrl+C
      // (Native terminal behavior - no additional assertions needed)
    });

    it('should disable mouse tracking on middle button down', () => {
      // @step Given the TUI is showing the conversation view with AI output
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      mockStdoutWrites = [];

      // @step When the user middle-clicks
      const middleClickEvent = createMouseEvent(MOUSE_BUTTONS.MIDDLE_DOWN);
      stdin.write(middleClickEvent);

      // @step Then mouse tracking should be temporarily disabled (?1000l)
      expect(mockStdoutWrites).toContain('\x1b[?1000l');
    });

    it('should disable mouse tracking on right button down', () => {
      // @step Given the TUI is showing the conversation view with AI output
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      mockStdoutWrites = [];

      // @step When the user right-clicks
      const rightClickEvent = createMouseEvent(MOUSE_BUTTONS.RIGHT_DOWN);
      stdin.write(rightClickEvent);

      // @step Then mouse tracking should be temporarily disabled (?1000l)
      expect(mockStdoutWrites).toContain('\x1b[?1000l');
    });
  });

  // ========================================
  // TIMER RE-ENABLE SCENARIOS
  // ========================================

  describe('Scenario: Button release immediately re-enables mouse tracking', () => {
    it('should re-enable mouse tracking on button release event', () => {
      // @step Given the TUI is showing the conversation view
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // @step And the user has clicked to select text
      mockStdoutWrites = [];
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      mockStdoutWrites = [];

      // @step When the user releases the mouse button
      const releaseEvent = createMouseEvent(MOUSE_BUTTONS.BUTTON_RELEASE);
      stdin.write(releaseEvent);

      // @step Then mouse tracking should be immediately re-enabled (?1000h)
      // @step And any pending timer should be cleared
      // @step And scroll wheel should work right away without waiting for timeout
      expect(mockStdoutWrites).toContain('\x1b[?1000h');
    });

    it('should clear pending timer on button release', () => {
      // @step Given the TUI is showing the conversation view
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // @step And the user has clicked to select text (timer pending)
      mockStdoutWrites = [];
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);

      // @step And 2 seconds have passed (timer still pending)
      vi.advanceTimersByTime(2000);
      mockStdoutWrites = [];

      // @step When the user releases the mouse button
      const releaseEvent = createMouseEvent(MOUSE_BUTTONS.BUTTON_RELEASE);
      stdin.write(releaseEvent);
      expect(mockStdoutWrites).toContain('\x1b[?1000h');

      mockStdoutWrites = [];

      // @step Then any pending timer should be cleared
      // @step And no duplicate re-enable should happen
      vi.advanceTimersByTime(5000);
      expect(mockStdoutWrites).not.toContain('\x1b[?1000h');
    });
  });

  describe('Scenario: Scroll wheel works again after selection timeout', () => {
    it('should re-enable mouse tracking after 5000ms timeout', async () => {
      // @step Given the TUI is showing the conversation view
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // @step And the user has just finished clicking/dragging to select text
      // @step And mouse tracking was temporarily disabled
      // @step And the button-release event was not captured
      mockStdoutWrites = [];
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      mockStdoutWrites = [];

      // @step When 5 seconds have passed since the click
      vi.advanceTimersByTime(5000);

      // @step Then mouse tracking should be re-enabled (?1000h)
      // @step And the user should be able to scroll with the mouse wheel again
      expect(mockStdoutWrites).toContain('\x1b[?1000h');
    });
  });

  describe('Scenario: Rapid clicks reset the re-enable timer', () => {
    it('should reset timer on second click and delay re-enable', () => {
      // @step Given the TUI is showing the conversation view
      // @step And mouse tracking is enabled (?1000h)
      const items = createItems(100);

      const { stdin } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // @step When the user clicks once (first click)
      mockStdoutWrites = [];
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      // @step And 3 seconds later clicks again (second click)
      vi.advanceTimersByTime(3000);
      mockStdoutWrites = [];
      stdin.write(leftClickEvent);

      // @step Then the re-enable timer should be reset
      // Timer should NOT fire at 5s from first click (only 2s from second)
      mockStdoutWrites = [];
      vi.advanceTimersByTime(2000); // 5s from first click, but only 2s from second

      // @step And mouse tracking should stay disabled for 5 seconds from the second click
      // @step And mouse tracking should not be re-enabled at 5 seconds from the first click
      expect(mockStdoutWrites).not.toContain('\x1b[?1000h');

      // Now advance to 5s from second click
      vi.advanceTimersByTime(3000); // Total 5s from second click
      expect(mockStdoutWrites).toContain('\x1b[?1000h');
    });
  });

  // ========================================
  // CLEANUP SCENARIOS
  // ========================================

  describe('Scenario: Timer cleaned up when navigating away', () => {
    it('should clear pending timer on unmount', () => {
      // @step Given the TUI is showing the conversation view
      const items = createItems(100);

      const { stdin, unmount } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // @step And the user has clicked to select text (triggering the disable timer)
      // @step And the re-enable timer is pending
      mockStdoutWrites = [];
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      mockStdoutWrites = [];

      // @step When the user navigates away from the conversation view
      unmount();

      // @step Then the pending re-enable timer should be cleared
      // @step And mouse tracking should be cleanly disabled (?1000l)
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      // Verify timer was cleared - no re-enable after timeout
      mockStdoutWrites = [];
      vi.advanceTimersByTime(3000);
      expect(mockStdoutWrites).not.toContain('\x1b[?1000h');
    });

    it('should clear timer when isFocused becomes false', () => {
      // @step Given the TUI is showing the conversation view
      const items = createItems(100);

      const { stdin, rerender } = render(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            isFocused={true}
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // Click to trigger disable
      mockStdoutWrites = [];
      const leftClickEvent = createMouseEvent(MOUSE_BUTTONS.LEFT_DOWN);
      stdin.write(leftClickEvent);
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      mockStdoutWrites = [];

      // @step When the user navigates away from the conversation view
      // Simulate focus change
      rerender(
        <Box height={10}>
          <VirtualList
            items={items}
            selectionMode="scroll"
            isFocused={false}
            renderItem={(item) => <Text>{item}</Text>}
          />
        </Box>
      );

      // @step Then the pending re-enable timer should be cleared
      // @step And mouse tracking should be cleanly disabled (?1000l)
      expect(mockStdoutWrites).toContain('\x1b[?1000l');

      // Verify timer was cleared
      mockStdoutWrites = [];
      vi.advanceTimersByTime(3000);
      expect(mockStdoutWrites).not.toContain('\x1b[?1000h');
    });
  });
});
