/**
 * Tests for input priority propagation behavior
 *
 * Verifies the core principle of the priority-based input system:
 * - When a handler returns `true`, propagation STOPS (input is consumed)
 * - When a handler returns `false`, propagation CONTINUES to lower priority
 *
 * This is critical for preventing issues like:
 * - Slash command Up/Down arrows also scrolling VirtualList
 * - Dialog input leaking to underlying views
 *
 * INPUT-001: Priority-based input handling
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { InputManager } from '../../input/InputManager';
import { useInputCompat, InputPriority } from '../../input';
import { SGR_BUTTON, parseSgrMouse } from '../../utils/mouseProtocol';

describe('Input priority propagation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('return true stops propagation', () => {
    it('should NOT call lower priority handlers when higher returns true', async () => {
      const highHandler = vi.fn(() => true); // Consumes input
      const lowHandler = vi.fn(() => true);
      const bgHandler = vi.fn(() => true);

      function HighPriority() {
        useInputCompat({
          id: 'high',
          priority: InputPriority.HIGH,
          isActive: true,
          handler: highHandler,
        });
        return null;
      }

      function LowPriority() {
        useInputCompat({
          id: 'low',
          priority: InputPriority.LOW,
          isActive: true,
          handler: lowHandler,
        });
        return null;
      }

      function Background() {
        useInputCompat({
          id: 'bg',
          priority: InputPriority.BACKGROUND,
          isActive: true,
          handler: bgHandler,
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <HighPriority />
          <LowPriority />
          <Background />
          <Text>Test</Text>
        </InputManager>
      );

      stdin.write('x');
      await new Promise(r => setTimeout(r, 20));

      expect(highHandler).toHaveBeenCalledTimes(1);
      expect(lowHandler).not.toHaveBeenCalled();
      expect(bgHandler).not.toHaveBeenCalled();
    });

    it('should stop at LOW when it returns true (simulates AgentView fix)', async () => {
      const mediumHandler = vi.fn(() => false); // Propagates (like MultiLineInput for Up/Down)
      const lowHandler = vi.fn(() => true); // Consumes (like AgentView with slash command)
      const bgHandler = vi.fn(() => true); // Should NOT be called (like VirtualList)

      function Medium() {
        useInputCompat({
          id: 'medium',
          priority: InputPriority.MEDIUM,
          isActive: true,
          handler: mediumHandler,
        });
        return null;
      }

      function Low() {
        useInputCompat({
          id: 'low',
          priority: InputPriority.LOW,
          isActive: true,
          handler: lowHandler,
        });
        return null;
      }

      function Background() {
        useInputCompat({
          id: 'bg',
          priority: InputPriority.BACKGROUND,
          isActive: true,
          handler: bgHandler,
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <Medium />
          <Low />
          <Background />
          <Text>Test</Text>
        </InputManager>
      );

      stdin.write('\x1b[A'); // Up arrow
      await new Promise(r => setTimeout(r, 20));

      expect(mediumHandler).toHaveBeenCalledTimes(1);
      expect(lowHandler).toHaveBeenCalledTimes(1);
      expect(bgHandler).not.toHaveBeenCalled(); // Critical: should NOT reach background
    });
  });

  describe('return false continues propagation', () => {
    it('should call all handlers when all return false', async () => {
      const highHandler = vi.fn(() => false);
      const lowHandler = vi.fn(() => false);
      const bgHandler = vi.fn(() => false);

      function HighPriority() {
        useInputCompat({
          id: 'high',
          priority: InputPriority.HIGH,
          isActive: true,
          handler: highHandler,
        });
        return null;
      }

      function LowPriority() {
        useInputCompat({
          id: 'low',
          priority: InputPriority.LOW,
          isActive: true,
          handler: lowHandler,
        });
        return null;
      }

      function Background() {
        useInputCompat({
          id: 'bg',
          priority: InputPriority.BACKGROUND,
          isActive: true,
          handler: bgHandler,
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <HighPriority />
          <LowPriority />
          <Background />
          <Text>Test</Text>
        </InputManager>
      );

      stdin.write('x');
      await new Promise(r => setTimeout(r, 20));

      expect(highHandler).toHaveBeenCalledTimes(1);
      expect(lowHandler).toHaveBeenCalledTimes(1);
      expect(bgHandler).toHaveBeenCalledTimes(1);
    });
  });

  describe('real-world scenario: slash command with VirtualList', () => {
    it('should NOT scroll VirtualList when slash command handles Up/Down', async () => {
      // Simulates the real scenario:
      // - MultiLineInput (MEDIUM) propagates Up/Down for single-line input
      // - AgentView (LOW) has slash command visible, handles Up/Down
      // - VirtualList (BACKGROUND) should NOT receive Up/Down

      let slashCommandVisible = true;
      const virtualListScroll = vi.fn();

      // Simulates MultiLineInput behavior for single-line input
      function MultiLineInputSim() {
        useInputCompat({
          id: 'multi-line-input',
          priority: InputPriority.MEDIUM,
          isActive: true,
          handler: (input, key) => {
            // Single-line input: Up/Down have nowhere to go, propagate
            if (key.upArrow || key.downArrow) {
              return false; // Propagate
            }
            return false;
          },
        });
        return null;
      }

      // Simulates AgentView with slash command handling
      function AgentViewSim() {
        useInputCompat({
          id: 'agent-view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            // Slash command palette handles Up/Down when visible
            if (slashCommandVisible && (key.upArrow || key.downArrow)) {
              // This is the fix: return true to stop propagation
              return true;
            }
            return false;
          },
        });
        return null;
      }

      // Simulates VirtualList scroll handling
      function VirtualListSim() {
        useInputCompat({
          id: 'virtual-list',
          priority: InputPriority.BACKGROUND,
          isActive: true,
          handler: (input, key) => {
            if (key.upArrow || key.downArrow) {
              virtualListScroll();
              return true;
            }
            return false;
          },
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <MultiLineInputSim />
          <AgentViewSim />
          <VirtualListSim />
          <Text>Test</Text>
        </InputManager>
      );

      // Press Up arrow - should be handled by slash command, NOT VirtualList
      stdin.write('\x1b[A');
      await new Promise(r => setTimeout(r, 20));

      expect(virtualListScroll).not.toHaveBeenCalled();

      // Press Down arrow - same behavior
      stdin.write('\x1b[B');
      await new Promise(r => setTimeout(r, 20));

      expect(virtualListScroll).not.toHaveBeenCalled();

      // Now hide slash command - VirtualList should receive arrows
      slashCommandVisible = false;

      stdin.write('\x1b[A');
      await new Promise(r => setTimeout(r, 20));

      expect(virtualListScroll).toHaveBeenCalledTimes(1);
    });
  });

  describe('mouse event propagation', () => {
    it('should propagate mouse scroll to VirtualList when not handled by overlay', async () => {
      // Simulates the real scenario:
      // - AgentView (LOW) doesn't handle scroll when no modal is open
      // - VirtualList (BACKGROUND) should receive scroll for conversation

      let modalOpen = false;
      const virtualListScroll = vi.fn();

      // Simulates AgentView mouse handling (BUG-131: uses SGR protocol)
      function AgentViewSim() {
        useInputCompat({
          id: 'agent-view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input) => {
            // Only handle mouse scroll when modal is open
            const mouseEvent = parseSgrMouse(input);
            if (mouseEvent) {
              if (modalOpen && (mouseEvent.button === SGR_BUTTON.SCROLL_UP || mouseEvent.button === SGR_BUTTON.SCROLL_DOWN)) {
                return true; // Handle scroll in modal
              }
              // Let unhandled mouse events propagate to VirtualList
              return false;
            }
            return false;
          },
        });
        return null;
      }

      // Simulates VirtualList scroll handling (BUG-131: uses SGR protocol)
      function VirtualListSim() {
        useInputCompat({
          id: 'virtual-list',
          priority: InputPriority.BACKGROUND,
          isActive: true,
          handler: (input) => {
            const mouseEvent = parseSgrMouse(input);
            if (mouseEvent && (mouseEvent.button === SGR_BUTTON.SCROLL_UP || mouseEvent.button === SGR_BUTTON.SCROLL_DOWN)) {
              virtualListScroll();
              return true;
            }
            return false;
          },
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <AgentViewSim />
          <VirtualListSim />
          <Text>Test</Text>
        </InputManager>
      );

      // With no modal open, mouse scroll should reach VirtualList
      modalOpen = false;
      
      // BUG-131: Send SGR scroll-up event (post-ESC-strip format)
      stdin.write('[<64;10;20M');

      // VirtualList should have received the scroll event
      expect(virtualListScroll).toHaveBeenCalled();

      // When modal IS open, scroll should be consumed by AgentView
      virtualListScroll.mockClear();
      modalOpen = true;
      stdin.write('[<64;10;20M');
      // AgentView returns true, VirtualList shouldn't be called
      expect(virtualListScroll).not.toHaveBeenCalled();
    });
  });

  describe('undefined/void return is treated as false', () => {
    it('should propagate when handler returns undefined (the bug we fixed)', async () => {
      // This tests the BUG that existed before: `return;` (undefined) caused propagation
      const highHandler = vi.fn(() => {
        // BUG: `return;` without a value returns undefined
        return; // This should be `return true;` to stop propagation
      });
      const lowHandler = vi.fn(() => true);

      function HighPriority() {
        useInputCompat({
          id: 'high',
          priority: InputPriority.HIGH,
          isActive: true,
          handler: highHandler,
        });
        return null;
      }

      function LowPriority() {
        useInputCompat({
          id: 'low',
          priority: InputPriority.LOW,
          isActive: true,
          handler: lowHandler,
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <HighPriority />
          <LowPriority />
          <Text>Test</Text>
        </InputManager>
      );

      stdin.write('x');
      await new Promise(r => setTimeout(r, 20));

      // Both handlers called because high returned undefined (falsy)
      expect(highHandler).toHaveBeenCalledTimes(1);
      expect(lowHandler).toHaveBeenCalledTimes(1); // BUG: this gets called when it shouldn't
    });
  });
});
