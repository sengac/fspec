/**
 * useSlashCommandInput Hook
 *
 * Complete slash command palette management including:
 * - Visibility and filter state
 * - Command filtering with three-tier matching
 * - Keyboard navigation
 * - Input handling with proper priority
 *
 * This hook owns ALL slash command behavior following Single Responsibility Principle.
 * It returns an input handler function that should be called from the parent's useInputCompat handler.
 *
 * Work Unit: TUI-050
 */

import { useState, useCallback, useMemo } from 'react';
import type { Key } from 'ink';
import {
  SLASH_COMMANDS,
  filterCommands,
  type SlashCommand,
} from '../utils/slashCommands';

/** Maximum width for the dialog */
const MAX_DIALOG_WIDTH = 70;

/** Minimum width for the dialog */
const MIN_DIALOG_WIDTH = 45;

export interface UseSlashCommandInputOptions {
  /**
   * Current input value from the text input
   */
  inputValue: string;

  /**
   * Callback to update the input value
   */
  onInputChange: (value: string) => void;

  /**
   * Callback to execute a slash command
   * Called when user presses Enter on a selected command
   */
  onExecuteCommand: (command: string) => void;

  /**
   * When true, the palette is disabled and will auto-hide
   * Use this when other modes (resume, watcher, model selector) are active
   */
  disabled?: boolean;
}

export interface UseSlashCommandInputResult {
  /** Whether the palette is currently visible */
  isVisible: boolean;

  /** Current filter text (after "/") */
  filter: string;

  /** Filtered list of commands to display */
  filteredCommands: SlashCommand[];

  /** Currently selected command index */
  selectedIndex: number;

  /**
   * Fixed dialog width based on all commands
   * Prevents dialog from expanding/contracting during scroll
   */
  dialogWidth: number;

  /**
   * Handle keyboard input
   * Returns true if the input was handled (should stop propagation)
   * Call this from your useInputCompat handler
   */
  handleInput: (input: string, key: Key) => boolean;

  /**
   * Handle input text changes
   * Call this when input value changes to show/hide palette
   */
  handleInputChange: (newValue: string) => void;
}

export function useSlashCommandInput(
  options: UseSlashCommandInputOptions
): UseSlashCommandInputResult {
  const {
    inputValue,
    onInputChange,
    onExecuteCommand,
    disabled = false,
  } = options;

  // Internal state
  const [isVisible, setIsVisible] = useState(false);
  const [filter, setFilter] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Filter commands based on current filter
  const filteredCommands = useMemo(
    () => filterCommands(SLASH_COMMANDS, filter),
    [filter]
  );

  // Calculate fixed dialog width based on ALL commands (not just filtered)
  const dialogWidth = useMemo(() => {
    const allCommands = SLASH_COMMANDS;
    if (allCommands.length === 0) {
      return MIN_DIALOG_WIDTH;
    }

    const maxNameWidth = Math.max(...allCommands.map(c => c.name.length), 8);
    const maxDescriptionWidth = Math.max(
      ...allCommands.map(c => c.description.length),
      10
    );
    const contentWidth = 2 + 1 + maxNameWidth + 1 + maxDescriptionWidth;
    const footerWidth = 43; // "↑↓ Navigate │ Tab/Enter Select │ Esc Close"

    const calculatedWidth = Math.max(
      contentWidth,
      footerWidth,
      MIN_DIALOG_WIDTH
    );
    return Math.min(calculatedWidth, MAX_DIALOG_WIDTH);
  }, []);

  // Auto-hide when disabled
  const effectiveIsVisible = isVisible && !disabled;

  // Show the palette
  const show = useCallback(() => {
    if (!disabled) {
      setIsVisible(true);
      setFilter('');
      setSelectedIndex(0);
    }
  }, [disabled]);

  // Hide the palette
  const hide = useCallback(() => {
    setIsVisible(false);
    setFilter('');
    setSelectedIndex(0);
  }, []);

  // Update filter
  const updateFilter = useCallback((newFilter: string) => {
    setFilter(newFilter);
    setSelectedIndex(0); // Reset selection when filter changes
  }, []);

  // Move selection up (with wrap-around)
  const moveUp = useCallback(() => {
    setSelectedIndex(prev => {
      if (prev <= 0) {
        return filteredCommands.length - 1;
      }
      return prev - 1;
    });
  }, [filteredCommands.length]);

  // Move selection down (with wrap-around)
  const moveDown = useCallback(() => {
    setSelectedIndex(prev => {
      if (prev >= filteredCommands.length - 1) {
        return 0;
      }
      return prev + 1;
    });
  }, [filteredCommands.length]);

  // Get currently selected command
  const getSelectedCommand = useCallback((): SlashCommand | undefined => {
    return filteredCommands[selectedIndex];
  }, [filteredCommands, selectedIndex]);

  // Handle input text changes (show/hide palette based on input)
  const handleInputChange = useCallback(
    (newValue: string) => {
      onInputChange(newValue);

      if (disabled) {
        if (isVisible) hide();
        return;
      }

      // Hide palette if input is cleared (command was executed)
      if (!newValue || newValue === '') {
        if (isVisible) hide();
        return;
      }

      const filterText = newValue.slice(1);

      // Show palette when "/" is typed at position 0, hide when space is typed (arguments)
      if (newValue.startsWith('/') && !filterText.includes(' ')) {
        if (!isVisible) {
          show();
        }
        updateFilter(filterText);
      } else if (isVisible) {
        // Hide if "/" removed or space typed (arguments mode)
        hide();
      }
    },
    [onInputChange, disabled, isVisible, show, hide, updateFilter]
  );

  // Handle keyboard input - returns true if handled
  const handleInput = useCallback(
    (input: string, key: Key): boolean => {
      // Don't handle if disabled or not visible or no commands to select
      if (disabled || !effectiveIsVisible || filteredCommands.length === 0) {
        return false;
      }

      if (key.escape) {
        hide();
        return true;
      }

      if (key.upArrow) {
        moveUp();
        return true;
      }

      if (key.downArrow) {
        moveDown();
        return true;
      }

      if (key.tab) {
        const selected = getSelectedCommand();
        if (selected) {
          // Fill in command name (no trailing space)
          onInputChange(`/${selected.name}`);
          hide();
        }
        return true;
      }

      if (key.return) {
        const selected = getSelectedCommand();
        if (selected) {
          // Execute the command
          const commandText = `/${selected.name}`;
          // IMPORTANT: Hide FIRST, then clear input, then execute
          // This ensures the palette is hidden before command execution triggers mode changes
          hide();
          onInputChange(''); // Clear input immediately
          onExecuteCommand(commandText);
        }
        return true;
      }

      // Don't capture other keys - let them go to the text input
      return false;
    },
    [
      disabled,
      effectiveIsVisible,
      filteredCommands.length,
      hide,
      moveUp,
      moveDown,
      getSelectedCommand,
      onInputChange,
      onExecuteCommand,
    ]
  );

  return {
    isVisible: effectiveIsVisible,
    filter,
    filteredCommands,
    selectedIndex,
    dialogWidth,
    handleInput,
    handleInputChange,
  };
}
