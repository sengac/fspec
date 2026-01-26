/**
 * useSlashCommand Hook
 *
 * Manages slash command palette state including:
 * - Visibility based on "/" at position 0
 * - Command filtering with three-tier matching
 * - Keyboard navigation (Up/Down)
 * - Selection handling (Tab/Enter)
 *
 * Work Unit: TUI-050
 */

import { useState, useCallback, useMemo } from 'react';
import {
  SLASH_COMMANDS,
  filterCommands,
  type SlashCommand,
} from '../utils/slashCommands';

export interface UseSlashCommandResult {
  /** Whether the palette is visible */
  isVisible: boolean;
  /** Current filter text (after "/") */
  filter: string;
  /** Filtered list of commands */
  filteredCommands: SlashCommand[];
  /** Currently selected command index */
  selectedIndex: number;
  /** Show the palette */
  show: () => void;
  /** Hide the palette */
  hide: () => void;
  /** Move selection up */
  moveUp: () => void;
  /** Move selection down */
  moveDown: () => void;
  /** Update the filter text */
  setFilter: (filter: string) => void;
  /** Reset selection to first item */
  resetSelection: () => void;
  /** Get the currently selected command */
  getSelectedCommand: () => SlashCommand | undefined;
}

export function useSlashCommand(): UseSlashCommandResult {
  const [isVisible, setIsVisible] = useState(false);
  const [filter, setFilterInternal] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);

  const filteredCommands = useMemo(
    () => filterCommands(SLASH_COMMANDS, filter),
    [filter]
  );

  const show = useCallback(() => {
    setIsVisible(true);
    setFilterInternal('');
    setSelectedIndex(0);
  }, []);

  const hide = useCallback(() => {
    setIsVisible(false);
    setFilterInternal('');
    setSelectedIndex(0);
  }, []);

  const setFilter = useCallback((newFilter: string) => {
    setFilterInternal(newFilter);
    setSelectedIndex(0); // Reset selection when filter changes
  }, []);

  const moveUp = useCallback(() => {
    setSelectedIndex(prev => {
      if (prev <= 0) {
        return filteredCommands.length - 1; // Wrap to bottom
      }
      return prev - 1;
    });
  }, [filteredCommands.length]);

  const moveDown = useCallback(() => {
    setSelectedIndex(prev => {
      if (prev >= filteredCommands.length - 1) {
        return 0; // Wrap to top
      }
      return prev + 1;
    });
  }, [filteredCommands.length]);

  const resetSelection = useCallback(() => {
    setSelectedIndex(0);
  }, []);

  const getSelectedCommand = useCallback(() => {
    return filteredCommands[selectedIndex];
  }, [filteredCommands, selectedIndex]);

  return {
    isVisible,
    filter,
    filteredCommands,
    selectedIndex,
    show,
    hide,
    moveUp,
    moveDown,
    setFilter,
    resetSelection,
    getSelectedCommand,
  };
}
