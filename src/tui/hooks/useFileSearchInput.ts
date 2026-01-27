/**
 * useFileSearchInput Hook
 *
 * Complete file search popup management following the EXACT same architecture as useSlashCommandInput:
 * - Visibility and filter state
 * - File searching with Glob tool integration
 * - Keyboard navigation
 * - Input handling with @ symbol detection
 *
 * This hook matches useSlashCommandInput interface for consistency.
 *
 * Work Unit: TUI-055
 */

import { useState, useCallback, useMemo } from 'react';
import type { Key } from 'ink';
import { callGlobTool } from '../../utils/toolIntegration';

export interface FileSearchResult {
  /** File path relative to project root */
  path: string;
  /** Display name (can be truncated path) */
  displayName?: string;
}

/** Maximum width for the dialog */
const MAX_DIALOG_WIDTH = 70;

/** Minimum width for the dialog */
const MIN_DIALOG_WIDTH = 45;

export interface UseFileSearchInputOptions {
  /**
   * Current input value from the text input
   */
  inputValue: string;

  /**
   * Callback to update the input value
   */
  onInputChange: (value: string) => void;

  /**
   * When true, the popup is disabled and will auto-hide
   * Use this when other modes (resume, watcher, model selector) are active
   */
  disabled?: boolean;
}

export interface UseFileSearchInputResult {
  /** Whether the popup is currently visible */
  isVisible: boolean;

  /** Current filter text (after "@") */
  filter: string;

  /** Filtered list of files to display */
  files: FileSearchResult[];

  /** Currently selected file index */
  selectedIndex: number;

  /**
   * Fixed dialog width based on file paths
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
   * Call this when input value changes to show/hide popup
   */
  handleInputChange: (newValue: string) => void;
}

export function useFileSearchInput(
  options: UseFileSearchInputOptions
): UseFileSearchInputResult {
  const { inputValue, onInputChange, disabled = false } = options;

  // Internal state
  const [isVisible, setIsVisible] = useState(false);
  const [filter, setFilter] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [files, setFiles] = useState<FileSearchResult[]>([]);

  // Calculate fixed dialog width based on file paths
  const dialogWidth = useMemo(() => {
    if (files.length === 0) {
      return MIN_DIALOG_WIDTH;
    }

    const maxPathWidth = Math.max(...files.map(f => f.path.length), 20);
    const contentWidth = 4 + maxPathWidth; // "🔍 " + path + padding
    const footerWidth = 43; // "↑↓ Navigate │ Enter Select │ Esc Close"

    const calculatedWidth = Math.max(
      contentWidth,
      footerWidth,
      MIN_DIALOG_WIDTH
    );
    return Math.min(calculatedWidth, MAX_DIALOG_WIDTH);
  }, [files]);

  // Auto-hide when disabled
  const effectiveIsVisible = isVisible && !disabled;

  // Show the popup
  const show = useCallback(() => {
    if (!disabled) {
      setIsVisible(true);
      setFilter('');
      setSelectedIndex(0);
    }
  }, [disabled]);

  // Hide the popup
  const hide = useCallback(() => {
    setIsVisible(false);
    setFilter('');
    setSelectedIndex(0);
    setFiles([]);
  }, []);

  // Update filter and search files
  const updateFilter = useCallback(async (newFilter: string) => {
    setFilter(newFilter);
    setSelectedIndex(0); // Reset selection when filter changes

    if (!newFilter.trim()) {
      setFiles([]);
      return;
    }

    try {
      // Use Glob tool to search for files
      const pattern = `**/*${newFilter}*`;
      const result = await callGlobTool(pattern);

      if (result.success && result.data) {
        const filePaths = result.data.split('\n').filter(Boolean);
        const fileResults: FileSearchResult[] = filePaths.map(path => ({
          path,
          displayName: path.split('/').pop() || path,
        }));
        setFiles(fileResults);
      } else {
        setFiles([]);
      }
    } catch (error) {
      console.error('File search error:', error);
      setFiles([]);
    }
  }, []);

  // Move selection up (with wrap-around)
  const moveUp = useCallback(() => {
    setSelectedIndex(prev => {
      if (prev <= 0) {
        return files.length - 1;
      }
      return prev - 1;
    });
  }, [files.length]);

  // Move selection down (with wrap-around)
  const moveDown = useCallback(() => {
    setSelectedIndex(prev => {
      if (prev >= files.length - 1) {
        return 0;
      }
      return prev + 1;
    });
  }, [files.length]);

  // Get currently selected file
  const getSelectedFile = useCallback((): FileSearchResult | undefined => {
    return files[selectedIndex];
  }, [files, selectedIndex]);

  // Handle input text changes (show/hide popup based on input)
  const handleInputChange = useCallback(
    (newValue: string) => {
      onInputChange(newValue);

      if (disabled) {
        if (isVisible) hide();
        return;
      }

      // Hide popup if input is cleared
      if (!newValue || newValue === '') {
        if (isVisible) hide();
        return;
      }

      // Find @ symbol position (can be anywhere in the string, unlike slash commands)
      const atIndex = newValue.lastIndexOf('@');

      if (atIndex === -1) {
        // No @ symbol - hide popup
        if (isVisible) hide();
        return;
      }

      // Extract filter text after @
      const afterAt = newValue.slice(atIndex + 1);

      // Show popup when @ is typed, hide when space is typed after @ (end of file reference)
      if (atIndex >= 0 && !afterAt.includes(' ')) {
        if (!isVisible) {
          show();
        }
        updateFilter(afterAt);
      } else if (isVisible) {
        // Hide if space typed after @ (end of file reference)
        hide();
      }
    },
    [onInputChange, disabled, isVisible, show, hide, updateFilter]
  );

  // Handle keyboard input - returns true if handled
  const handleInput = useCallback(
    (input: string, key: Key): boolean => {
      // Don't handle if disabled or not visible or no files to select
      if (disabled || !effectiveIsVisible || files.length === 0) {
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
        const selected = getSelectedFile();
        if (selected) {
          // Fill in file path (no trailing space for Tab, unlike Enter)
          const atIndex = inputValue.lastIndexOf('@');
          if (atIndex >= 0) {
            const beforeAt = inputValue.slice(0, atIndex);
            const newValue = `${beforeAt}@${selected.path}`;
            onInputChange(newValue);
          }
          hide();
        }
        return true;
      }

      if (key.return) {
        const selected = getSelectedFile();
        if (selected) {
          // Find the @ position and replace the filter with the full file path
          const atIndex = inputValue.lastIndexOf('@');
          if (atIndex >= 0) {
            const beforeAt = inputValue.slice(0, atIndex);
            const newValue = `${beforeAt}@${selected.path} `;
            onInputChange(newValue);
          }
          hide();
        }
        return true;
      }

      // Don't capture other keys - let them go to the text input
      return false;
    },
    [
      disabled,
      effectiveIsVisible,
      files.length,
      hide,
      moveUp,
      moveDown,
      getSelectedFile,
      onInputChange,
      inputValue,
    ]
  );

  return {
    isVisible: effectiveIsVisible,
    filter,
    files,
    selectedIndex,
    dialogWidth,
    handleInput,
    handleInputChange,
  };
}
