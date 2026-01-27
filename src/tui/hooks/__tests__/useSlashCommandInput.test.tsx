/**
 * Tests for useSlashCommandInput hook
 *
 * Verifies slash command palette behavior including:
 * - Keyboard navigation return values (Up/Down/Tab/Enter/Escape)  
 * - Enter key propagation behavior based on visibility
 * - Command execution and Tab completion side effects
 *
 * TUI-050: Slash command autocomplete feature
 * 
 * KEY BEHAVIOR TESTED:
 * - handleInput returns true = key consumed, stops propagation
 * - handleInput returns false = key propagates to next handler (e.g., MultiLineInput)
 * 
 * This is critical for the suppressEnter/Enter propagation fix:
 * When slash palette is visible → returns true → MultiLineInput doesn't submit
 * When slash palette is hidden → returns false → MultiLineInput can submit
 */

import React from 'react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from 'ink-testing-library';
import { Text } from 'ink';
import { useSlashCommandInput, type UseSlashCommandInputResult } from '../useSlashCommandInput';
import type { Key } from 'ink';

// Mock Key object factory
const createKey = (overrides: Partial<Key> = {}): Key => ({
  upArrow: false,
  downArrow: false,
  leftArrow: false,
  rightArrow: false,
  pageDown: false,
  pageUp: false,
  return: false,
  escape: false,
  ctrl: false,
  shift: false,
  tab: false,
  backspace: false,
  delete: false,
  meta: false,
  ...overrides,
});

// Shared state ref for testing
let hookState: UseSlashCommandInputResult | null = null;
let lastOnInputChange: ReturnType<typeof vi.fn> | null = null;
let lastOnExecuteCommand: ReturnType<typeof vi.fn> | null = null;

// Test component with controlled visibility state
interface TestComponentProps {
  showPalette: boolean;
  filter?: string;
  disabled?: boolean;
}

function TestComponent({ showPalette, filter = '', disabled = false }: TestComponentProps) {
  const onInputChange = vi.fn();
  const onExecuteCommand = vi.fn();
  
  lastOnInputChange = onInputChange;
  lastOnExecuteCommand = onExecuteCommand;
  
  // Construct inputValue to trigger show/filter
  const inputValue = showPalette ? `/${filter}` : filter;
  
  const slashCommand = useSlashCommandInput({
    inputValue,
    onInputChange,
    onExecuteCommand,
    disabled,
  });

  // Manually trigger show if needed based on props
  React.useEffect(() => {
    if (showPalette && !disabled) {
      slashCommand.handleInputChange(`/${filter}`);
    } else {
      slashCommand.handleInputChange(filter);
    }
  }, [showPalette, filter, disabled]);

  // Store current state
  hookState = slashCommand;

  return (
    <Text>
      V:{String(slashCommand.isVisible)}|C:{slashCommand.filteredCommands.length}|I:{slashCommand.selectedIndex}
    </Text>
  );
}

describe('useSlashCommandInput', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hookState = null;
    lastOnInputChange = null;
    lastOnExecuteCommand = null;
  });

  describe('handleInput return values - NOT visible (key propagation)', () => {
    it('should return false for Enter when not visible', async () => {
      render(<TestComponent showPalette={false} />);
      await new Promise(r => setTimeout(r, 10));
      
      const handled = hookState?.handleInput('', createKey({ return: true }));
      expect(handled).toBe(false);
    });

    it('should return false for navigation keys when not visible', async () => {
      render(<TestComponent showPalette={false} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('', createKey({ upArrow: true }))).toBe(false);
      expect(hookState?.handleInput('', createKey({ downArrow: true }))).toBe(false);
      expect(hookState?.handleInput('', createKey({ tab: true }))).toBe(false);
      expect(hookState?.handleInput('', createKey({ escape: true }))).toBe(false);
    });

    it('should return false for regular characters when not visible', async () => {
      render(<TestComponent showPalette={false} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('a', createKey())).toBe(false);
    });
  });

  describe('handleInput return values - visible with commands (key consumption)', () => {
    it('should return true for Enter when visible with commands', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.isVisible).toBe(true);
      expect(hookState?.filteredCommands.length).toBeGreaterThan(0);
      
      const handled = hookState?.handleInput('', createKey({ return: true }));
      expect(handled).toBe(true);
    });

    it('should return true for Up arrow when visible', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('', createKey({ upArrow: true }))).toBe(true);
    });

    it('should return true for Down arrow when visible', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('', createKey({ downArrow: true }))).toBe(true);
    });

    it('should return true for Tab when visible', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('', createKey({ tab: true }))).toBe(true);
    });

    it('should return true for Escape when visible', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('', createKey({ escape: true }))).toBe(true);
    });

    it('should return false for regular characters when visible (propagate to input)', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.handleInput('a', createKey())).toBe(false);
    });
  });

  describe('handleInput return values - visible but no matching commands', () => {
    it('should return false for Enter when no commands match filter', async () => {
      // Filter that won't match any commands
      render(<TestComponent showPalette={true} filter="xyznonexistent123" />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.filteredCommands.length).toBe(0);
      
      const handled = hookState?.handleInput('', createKey({ return: true }));
      expect(handled).toBe(false);
    });
  });

  describe('handleInput return values - disabled', () => {
    it('should return false for all keys when disabled', async () => {
      render(<TestComponent showPalette={true} disabled={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.isVisible).toBe(false);
      expect(hookState?.handleInput('', createKey({ return: true }))).toBe(false);
      expect(hookState?.handleInput('', createKey({ upArrow: true }))).toBe(false);
    });
  });

  describe('command execution side effects', () => {
    it('should call onExecuteCommand on Enter when visible', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      const firstCommand = hookState?.filteredCommands[0]?.name;
      hookState?.handleInput('', createKey({ return: true }));
      
      expect(lastOnExecuteCommand).toHaveBeenCalledWith(`/${firstCommand}`);
    });

    it('should call onInputChange with empty string on Enter (clear input)', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      hookState?.handleInput('', createKey({ return: true }));
      
      expect(lastOnInputChange).toHaveBeenCalledWith('');
    });

    it('should NOT call onExecuteCommand when disabled', async () => {
      render(<TestComponent showPalette={true} disabled={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      hookState?.handleInput('', createKey({ return: true }));
      
      expect(lastOnExecuteCommand).not.toHaveBeenCalled();
    });
  });

  describe('Tab completion side effects', () => {
    it('should call onInputChange with command name (no trailing space) on Tab', async () => {
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      const firstCommand = hookState?.filteredCommands[0]?.name;
      hookState?.handleInput('', createKey({ tab: true }));
      
      expect(lastOnInputChange).toHaveBeenCalledWith(`/${firstCommand}`);
    });
  });

  describe('integration: suppressEnter scenario', () => {
    it('CRITICAL: returns true when visible = MultiLineInput suppressEnter prevents submit', async () => {
      // This is the key fix: when slash command palette is visible,
      // pressing Enter should be handled by slash command (returns true),
      // which means Enter doesn't propagate to MultiLineInput
      
      render(<TestComponent showPalette={true} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.isVisible).toBe(true);
      const handled = hookState?.handleInput('', createKey({ return: true }));
      expect(handled).toBe(true);
    });

    it('CRITICAL: returns false when not visible = Enter propagates for normal submit', async () => {
      // When palette is not visible, Enter should propagate to MultiLineInput
      // which will call onSubmit (since suppressEnter would be false)
      
      render(<TestComponent showPalette={false} />);
      await new Promise(r => setTimeout(r, 10));
      
      expect(hookState?.isVisible).toBe(false);
      const handled = hookState?.handleInput('', createKey({ return: true }));
      expect(handled).toBe(false);
    });
  });
});
