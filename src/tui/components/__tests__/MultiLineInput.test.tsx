/**
 * Tests for MultiLineInput component
 *
 * Verifies keyboard handling behavior including:
 * - Enter key submission and suppressEnter propagation
 * - Shift+Arrow key propagation to view level
 * - Text input and cursor movement
 *
 * INPUT-001: Tests priority-based input handling behavior
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text, Box } from 'ink';
import { MultiLineInput } from '../MultiLineInput';
import { InputManager } from '../../input/InputManager';
import { useInputCompat, InputPriority } from '../../input';

describe('MultiLineInput', () => {
  const defaultProps = {
    value: '',
    onChange: vi.fn(),
    onSubmit: vi.fn(),
    placeholder: 'Type here...',
    isActive: true,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Enter key handling', () => {
    it('should call onSubmit when Enter is pressed', async () => {
      const onSubmit = vi.fn();
      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} onSubmit={onSubmit} />
        </InputManager>
      );

      stdin.write('\r'); // Enter key
      await new Promise(resolve => setTimeout(resolve, 20));

      expect(onSubmit).toHaveBeenCalledTimes(1);
    });

    it('should NOT call onSubmit when suppressEnter is true', async () => {
      const onSubmit = vi.fn();
      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} onSubmit={onSubmit} suppressEnter={true} />
        </InputManager>
      );

      stdin.write('\r'); // Enter key
      await new Promise(resolve => setTimeout(resolve, 20));

      expect(onSubmit).not.toHaveBeenCalled();
    });

    it('should propagate Enter to lower priority handlers when suppressEnter is true', async () => {
      const onSubmit = vi.fn();
      const viewHandler = vi.fn(() => true);

      // Simulates AgentView's LOW priority handler that handles slash commands
      function ViewLevelHandler() {
        useInputCompat({
          id: 'view-handler',
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
          <MultiLineInput {...defaultProps} onSubmit={onSubmit} suppressEnter={true} />
          <ViewLevelHandler />
        </InputManager>
      );

      stdin.write('\r'); // Enter key
      await new Promise(resolve => setTimeout(resolve, 20));

      // MultiLineInput should NOT consume Enter when suppressed
      expect(onSubmit).not.toHaveBeenCalled();
      // View-level handler SHOULD receive the Enter
      expect(viewHandler).toHaveBeenCalledTimes(1);
    });
  });

  describe('Shift+Arrow propagation', () => {
    it('should propagate Shift+Left to view level for session switching', async () => {
      const sessionPrevHandler = vi.fn(() => true);

      // Simulates AgentView's handler for session switching
      function SessionSwitchHandler() {
        useInputCompat({
          id: 'session-handler',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            // Shift+Left escape sequence
            if (input.includes('[1;2D') || (key.shift && key.leftArrow)) {
              sessionPrevHandler();
              return true;
            }
            return false;
          },
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} />
          <SessionSwitchHandler />
        </InputManager>
      );

      // Send Shift+Left escape sequence
      stdin.write('\x1b[1;2D');
      await new Promise(resolve => setTimeout(resolve, 20));

      // Session handler should receive Shift+Left
      expect(sessionPrevHandler).toHaveBeenCalledTimes(1);
    });

    it('should propagate Shift+Right to view level for session switching', async () => {
      const sessionNextHandler = vi.fn(() => true);

      function SessionSwitchHandler() {
        useInputCompat({
          id: 'session-handler',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            // Shift+Right escape sequence
            if (input.includes('[1;2C') || (key.shift && key.rightArrow)) {
              sessionNextHandler();
              return true;
            }
            return false;
          },
        });
        return null;
      }

      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} />
          <SessionSwitchHandler />
        </InputManager>
      );

      // Send Shift+Right escape sequence
      stdin.write('\x1b[1;2C');
      await new Promise(resolve => setTimeout(resolve, 20));

      // Session handler should receive Shift+Right
      expect(sessionNextHandler).toHaveBeenCalledTimes(1);
    });

    it('should handle Shift+Up for history navigation', async () => {
      const onHistoryPrev = vi.fn();

      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} onHistoryPrev={onHistoryPrev} />
        </InputManager>
      );

      // Send Shift+Up escape sequence
      stdin.write('\x1b[1;2A');
      await new Promise(resolve => setTimeout(resolve, 20));

      expect(onHistoryPrev).toHaveBeenCalledTimes(1);
    });

    it('should handle Shift+Down for history navigation', async () => {
      const onHistoryNext = vi.fn();

      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} onHistoryNext={onHistoryNext} />
        </InputManager>
      );

      // Send Shift+Down escape sequence
      stdin.write('\x1b[1;2B');
      await new Promise(resolve => setTimeout(resolve, 20));

      expect(onHistoryNext).toHaveBeenCalledTimes(1);
    });
  });

  describe('regular arrow keys', () => {
    it('should handle Left arrow for cursor movement (not propagate)', async () => {
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
          <MultiLineInput {...defaultProps} value="hello" />
          <ViewHandler />
        </InputManager>
      );

      // Send Left arrow (regular, not shifted)
      stdin.write('\x1b[D');
      await new Promise(resolve => setTimeout(resolve, 20));

      // View handler should NOT be called - MultiLineInput handles it
      expect(viewHandler).not.toHaveBeenCalled();
    });

    it('should handle Right arrow for cursor movement (not propagate)', async () => {
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
          <MultiLineInput {...defaultProps} value="hello" />
          <ViewHandler />
        </InputManager>
      );

      // Send Right arrow (regular, not shifted)
      stdin.write('\x1b[C');
      await new Promise(resolve => setTimeout(resolve, 20));

      // View handler should NOT be called - MultiLineInput handles it
      expect(viewHandler).not.toHaveBeenCalled();
    });

    it('should propagate Up arrow for single-line input (for slash commands, VirtualList)', async () => {
      const viewHandler = vi.fn(() => true);

      function ViewHandler() {
        useInputCompat({
          id: 'view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            if (key.upArrow) {
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
          {/* Single-line value - Up has nowhere to go */}
          <MultiLineInput {...defaultProps} value="hello" />
          <ViewHandler />
        </InputManager>
      );

      // Send Up arrow
      stdin.write('\x1b[A');
      await new Promise(resolve => setTimeout(resolve, 20));

      // View handler SHOULD be called - MultiLineInput propagates Up for single-line
      expect(viewHandler).toHaveBeenCalledTimes(1);
    });

    it('should propagate Down arrow for single-line input (for slash commands, VirtualList)', async () => {
      const viewHandler = vi.fn(() => true);

      function ViewHandler() {
        useInputCompat({
          id: 'view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            if (key.downArrow) {
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
          {/* Single-line value - Down has nowhere to go */}
          <MultiLineInput {...defaultProps} value="hello" />
          <ViewHandler />
        </InputManager>
      );

      // Send Down arrow
      stdin.write('\x1b[B');
      await new Promise(resolve => setTimeout(resolve, 20));

      // View handler SHOULD be called - MultiLineInput propagates Down for single-line
      expect(viewHandler).toHaveBeenCalledTimes(1);
    });

    it('should handle Down arrow for multi-line input when not at bottom', async () => {
      const viewHandler = vi.fn(() => true);

      function ViewHandler() {
        useInputCompat({
          id: 'view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            if (key.downArrow) {
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
          {/* Multi-line value - cursor starts at first line, so Down can move down */}
          {/* Note: Must use JS expression for actual newlines, not JSX string */}
          <MultiLineInput {...defaultProps} value={"line1\nline2\nline3"} />
          <ViewHandler />
        </InputManager>
      );

      // Wait for any effects to settle
      await new Promise(resolve => setTimeout(resolve, 50));

      // Send Down arrow - cursor at top, should move down within multi-line input
      stdin.write('\x1b[B');
      await new Promise(resolve => setTimeout(resolve, 50));

      // View handler should NOT be called - MultiLineInput handles it (moves cursor down)
      expect(viewHandler).not.toHaveBeenCalled();
    });

    it('should propagate Up arrow when cursor is at first line of multi-line input', async () => {
      const viewHandler = vi.fn(() => true);

      function ViewHandler() {
        useInputCompat({
          id: 'view',
          priority: InputPriority.LOW,
          isActive: true,
          handler: (input, key) => {
            if (key.upArrow) {
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
          {/* Multi-line value - cursor starts at first line */}
          {/* Note: Must use JS expression for actual newlines */}
          <MultiLineInput {...defaultProps} value={"line1\nline2\nline3"} />
          <ViewHandler />
        </InputManager>
      );

      // Wait for any effects to settle
      await new Promise(resolve => setTimeout(resolve, 50));

      // Send Up arrow - cursor at top, nowhere to go, should propagate
      stdin.write('\x1b[A');
      await new Promise(resolve => setTimeout(resolve, 50));

      // View handler SHOULD be called - MultiLineInput propagates Up when at top
      expect(viewHandler).toHaveBeenCalledTimes(1);
    });
  });

  describe('text input', () => {
    it('should call onChange when typing characters', async () => {
      const onChange = vi.fn();

      const { stdin } = render(
        <InputManager>
          <MultiLineInput {...defaultProps} onChange={onChange} />
        </InputManager>
      );

      stdin.write('a');
      await new Promise(resolve => setTimeout(resolve, 20));

      expect(onChange).toHaveBeenCalled();
    });
  });

  describe('inactive state', () => {
    it('should not handle input when isActive is false', async () => {
      const onSubmit = vi.fn();
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
          <MultiLineInput {...defaultProps} onSubmit={onSubmit} isActive={false} />
          <ViewHandler />
        </InputManager>
      );

      stdin.write('\r'); // Enter
      await new Promise(resolve => setTimeout(resolve, 20));

      // MultiLineInput should not handle when inactive
      expect(onSubmit).not.toHaveBeenCalled();
      // Input should propagate to view handler
      expect(viewHandler).toHaveBeenCalled();
    });
  });
});
