/**
 * Tests for ConversationInputArea component
 *
 * Verifies the input area correctly handles:
 * - suppressEnter prop for slash command integration
 * - isActive prop for input focus control
 * - The correct pattern: isActive=true + suppressEnter=true when slash command visible
 *
 * TUI-050: Slash command autocomplete integration
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { ConversationInputArea } from '../ConversationInputArea';
import { InputManager } from '../../input/InputManager';
import { useInputCompat, InputPriority } from '../../input';

describe('ConversationInputArea', () => {
  const defaultProps = {
    value: '',
    onChange: vi.fn(),
    onSubmit: vi.fn(),
    isLoading: false,
    isActive: true,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('suppressEnter prop', () => {
    it('should call onSubmit when Enter pressed and suppressEnter=false', async () => {
      const onSubmit = vi.fn();

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            onSubmit={onSubmit}
            suppressEnter={false}
          />
        </InputManager>
      );

      stdin.write('\r'); // Enter
      await new Promise(r => setTimeout(r, 50));

      expect(onSubmit).toHaveBeenCalledTimes(1);
    });

    it('should NOT call onSubmit when Enter pressed and suppressEnter=true', async () => {
      const onSubmit = vi.fn();

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            onSubmit={onSubmit}
            suppressEnter={true}
          />
        </InputManager>
      );

      stdin.write('\r'); // Enter
      await new Promise(r => setTimeout(r, 50));

      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('should propagate Enter to view-level handler when suppressEnter=true', async () => {
      const onSubmit = vi.fn();
      const viewHandler = vi.fn(() => true);

      // Simulates view-level slash command handler
      function ViewHandler() {
        useInputCompat({
          id: 'view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            if (key.return) {
              viewHandler();
              return true;
            }
            return false;
          },
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            onSubmit={onSubmit}
            suppressEnter={true}
          />
          <ViewHandler />
        </InputManager>
      );

      stdin.write('\r'); // Enter
      await new Promise(r => setTimeout(r, 50));

      // Input should NOT call onSubmit
      expect(onSubmit).not.toHaveBeenCalled();
      // View handler SHOULD receive Enter
      expect(viewHandler).toHaveBeenCalledTimes(1);
    });
  });

  describe('isActive prop', () => {
    it('should handle typing when isActive=true', async () => {
      const onChange = vi.fn();

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            onChange={onChange}
            isActive={true}
          />
        </InputManager>
      );

      stdin.write('a');
      await new Promise(r => setTimeout(r, 50));

      expect(onChange).toHaveBeenCalled();
    });

    it('should NOT handle typing when isActive=false', async () => {
      const onChange = vi.fn();
      const viewHandler = vi.fn(() => true);

      function ViewHandler() {
        useInputCompat({
          id: 'view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: viewHandler,
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            onChange={onChange}
            isActive={false}
          />
          <ViewHandler />
        </InputManager>
      );

      stdin.write('a');
      await new Promise(r => setTimeout(r, 50));

      // onChange should NOT be called when inactive
      expect(onChange).not.toHaveBeenCalled();
      // Input should propagate to view
      expect(viewHandler).toHaveBeenCalled();
    });
  });

  describe('slash command integration pattern', () => {
    /**
     * CRITICAL: This test documents the correct pattern for slash command integration.
     * 
     * When slash command palette is visible:
     * - isActive should be TRUE (so typing works for filtering)
     * - suppressEnter should be TRUE (so Enter goes to slash command handler)
     * 
     * The WRONG pattern was: isActive=false when slash visible (breaks typing!)
     */
    it('should allow typing but suppress Enter when slash command visible', async () => {
      const onChange = vi.fn();
      const onSubmit = vi.fn();
      const slashCommandEnterHandler = vi.fn(() => true);

      // Simulates slash command handler in view (AgentView/SplitSessionView)
      function SlashCommandHandler() {
        useInputCompat({
          id: 'slash-command',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            if (key.return) {
              slashCommandEnterHandler();
              return true;
            }
            return false;
          },
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            onChange={onChange}
            onSubmit={onSubmit}
            isActive={true}              // CORRECT: stays active for typing
            suppressEnter={true}          // CORRECT: suppresses Enter for slash command
          />
          <SlashCommandHandler />
        </InputManager>
      );

      // Typing should work (for slash command filtering)
      stdin.write('d');
      await new Promise(r => setTimeout(r, 50));
      expect(onChange).toHaveBeenCalled();

      // Enter should go to slash command handler, not onSubmit
      stdin.write('\r');
      await new Promise(r => setTimeout(r, 50));
      expect(onSubmit).not.toHaveBeenCalled();
      expect(slashCommandEnterHandler).toHaveBeenCalledTimes(1);
    });

    it('should handle backspace for filtering when slash command visible', async () => {
      const onChange = vi.fn();

      const { stdin } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            value="/de"
            onChange={onChange}
            isActive={true}
            suppressEnter={true}
          />
        </InputManager>
      );

      // Backspace should work for editing the filter
      stdin.write('\x7f'); // Backspace
      await new Promise(r => setTimeout(r, 50));

      expect(onChange).toHaveBeenCalled();
    });
  });

  describe('loading state', () => {
    it('should show thinking indicator when isLoading=true', () => {
      const { lastFrame } = render(
        <InputManager>
          <ConversationInputArea
            {...defaultProps}
            isLoading={true}
          />
        </InputManager>
      );

      expect(lastFrame()).toContain('Thinking');
    });
  });
});
