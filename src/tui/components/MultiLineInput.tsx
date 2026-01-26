/**
 * MultiLineInput - Multi-line text input component with cursor navigation
 *
 * Implements TUI-039: Multi-line text input with cursor navigation for AgentView
 * INPUT-001: Uses centralized input handling with MEDIUM priority
 *
 * Features:
 * - Full cursor navigation (Left/Right, Up/Down, Home/End)
 * - Word-level operations (Alt+Left/Right, Alt+Backspace)
 * - Line operations (Shift+Enter for newline, Enter to submit)
 * - Viewport scrolling for content exceeding maxVisibleLines
 * - History navigation (Shift+Up/Down passed to parent)
 * - Visual cursor rendering with inverse style
 */

import React, { useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { useMultiLineInput } from '../hooks/useMultiLineInput.js';
import { useInputCompat, InputPriority } from '../input/index.js';

export interface MultiLineInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
  maxVisibleLines?: number;
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
  /**
   * TUI-050: When true, Enter key is NOT handled by this component.
   * This allows parent components (like slash command palette) to handle Enter.
   * Use this when an overlay is active that needs to capture Enter.
   */
  suppressEnter?: boolean;
}

export const MultiLineInput: React.FC<MultiLineInputProps> = ({
  value,
  onChange,
  onSubmit,
  placeholder = '',
  isActive = true,
  maxVisibleLines = 5,
  onHistoryPrev,
  onHistoryNext,
  suppressEnter = false,
}) => {
  const {
    lines,
    cursorRow,
    cursorCol,
    scrollOffset,
    visibleLines,
    value: hookValue,
    moveCursorLeft,
    moveCursorRight,
    moveCursorUp,
    moveCursorDown,
    moveCursorToLineStart,
    moveCursorToLineEnd,
    moveWordLeft,
    moveWordRight,
    insertChar,
    insertString,
    insertNewline,
    deleteCharBefore,
    deleteCharAt,
    deleteWordBefore,
    setValue,
    handleHistoryPrev,
    handleHistoryNext,
  } = useMultiLineInput({
    initialValue: value,
    maxVisibleLines,
    onHistoryPrev,
    onHistoryNext,
  });

  // Sync external value changes to internal state
  // Use a flag to prevent the reverse sync from overwriting external changes
  const lastExternalValueRef = useRef(value);
  const isSyncingFromExternalRef = useRef(false);

  useEffect(() => {
    if (value !== lastExternalValueRef.current && value !== hookValue) {
      isSyncingFromExternalRef.current = true;
      setValue(value);
      lastExternalValueRef.current = value;
      // Reset the flag after a microtask to allow the setValue to propagate
      Promise.resolve().then(() => {
        isSyncingFromExternalRef.current = false;
      });
    }
  }, [value, hookValue, setValue]);

  // Notify parent of internal value changes (but not when syncing from external)
  useEffect(() => {
    if (!isSyncingFromExternalRef.current && hookValue !== lastExternalValueRef.current) {
      lastExternalValueRef.current = hookValue;
      onChange(hookValue);
    }
  }, [hookValue, onChange]);

  // Handle keyboard input with MEDIUM priority (primary text input)
  useInputCompat({
    id: 'multi-line-input',
    priority: InputPriority.MEDIUM,
    description: 'Multi-line text input keyboard handler',
    isActive,
    handler: (input, key) => {
      // Ignore mouse escape sequences
      if (key.mouse || input.includes('[M') || input.includes('[<')) {
        return false;
      }

      // TUI-050: Enter submits UNLESS suppressed (for slash command palette)
      // When suppressed, return false so Enter propagates to view-level handler for slash commands
      if (key.return) {
        if (suppressEnter) {
          return false; // Let Enter propagate to AgentView for slash command handling
        }
        onSubmit();
        return true;
      }

      // Backspace
      if (key.backspace || key.delete) {
        // Check for Alt+Backspace (word delete)
        // Alt modifier may come through as meta in some terminals
        if (key.meta) {

          deleteWordBefore();
        } else {

          deleteCharBefore();
        }
        return true;
      }

      // Delete key (forward delete)
      if (input === '\x1b[3~') {
        
        deleteCharAt();
        return true;
      }

      // Shift+Arrow for history navigation (check before regular arrow handling)
      if (input.includes('[1;2A') || input.includes('\x1b[1;2A')) {
        
        handleHistoryPrev();
        return true;
      }
      if (input.includes('[1;2B') || input.includes('\x1b[1;2B')) {
        
        handleHistoryNext();
        return true;
      }
      if (key.shift && key.upArrow) {
        
        handleHistoryPrev();
        return true;
      }
      if (key.shift && key.downArrow) {

        handleHistoryNext();
        return true;
      }

      // TUI-049: Shift+Left/Right for session switching - let it propagate to view level
      // These are handled by AgentView/SplitSessionView, not by the input component
      if (input.includes('[1;2D') || input.includes('\x1b[1;2D')) {
        return false; // Let Shift+Left propagate to view level
      }
      if (input.includes('[1;2C') || input.includes('\x1b[1;2C')) {
        return false; // Let Shift+Right propagate to view level
      }
      if (key.shift && key.leftArrow) {
        return false; // Let Shift+Left propagate to view level
      }
      if (key.shift && key.rightArrow) {
        return false; // Let Shift+Right propagate to view level
      }

      // Alt+Arrow for word movement
      // Alt+Left: \x1b[1;3D or \x1bb
      // Alt+Right: \x1b[1;3C or \x1bf
      if (
        input.includes('[1;3D') ||
        input.includes('\x1b[1;3D') ||
        input === '\x1bb'
      ) {
        
        moveWordLeft();
        return true;
      }
      if (
        input.includes('[1;3C') ||
        input.includes('\x1b[1;3C') ||
        input === '\x1bf'
      ) {
        
        moveWordRight();
        return true;
      }
      if (key.meta && key.leftArrow) {
        
        moveWordLeft();
        return true;
      }
      if (key.meta && key.rightArrow) {
        
        moveWordRight();
        return true;
      }

      // Arrow keys for cursor movement
      // Left/Right: Always consume (cursor movement within line)
      // Up/Down: Only consume if there's a line to move to
      // This allows propagation to slash command palette, VirtualList, etc. when at boundary
      if (key.leftArrow) {
        
        moveCursorLeft();
        return true;
      }
      if (key.rightArrow) {
        
        moveCursorRight();
        return true;
      }
      if (key.upArrow) {
        // Only consume if there's a line above to move to
        if (cursorRow > 0) {
          moveCursorUp();
          return true;
        }
        // At top of input - let arrow propagate (for slash commands, VirtualList, etc.)
        return false;
      }
      if (key.downArrow) {
        // Only consume if there's a line below to move to
        if (cursorRow < lines.length - 1) {
          moveCursorDown();
          return true;
        }
        // At bottom of input - let arrow propagate
        return false;
      }

      // Home/End keys
      if (key.home || input === '\x1b[H') {
        
        moveCursorToLineStart();
        return true;
      }
      if (key.end || input === '\x1b[F') {
        
        moveCursorToLineEnd();
        return true;
      }

      // Ignore other special keys
      if (key.escape || key.tab || key.pageUp || key.pageDown) {
        return false;
      }

      // Filter to only printable characters
      const clean = input
        .split('')
        .filter((ch) => {
          const code = ch.charCodeAt(0);
          // Allow printable ASCII (space through tilde) and non-ASCII (unicode)
          return code >= 32 && code !== 127;
        })
        .join('');

      if (clean) {
        
        // Use insertString for bulk insert to avoid stale closure issues
        insertString(clean);
        return true;
      } else {
        
      }

      return false;
    },
  });

  // Render cursor at position
  const renderLineWithCursor = (
    line: string,
    lineIdx: number,
    actualRow: number
  ): React.ReactNode => {
    const isCursorRow = actualRow === cursorRow;

    if (!isCursorRow) {
      return <Text key={lineIdx}>{line || ' '}</Text>;
    }

    // Split line at cursor position
    const before = line.slice(0, cursorCol);
    const cursorChar = line[cursorCol] ?? ' ';
    const after = line.slice(cursorCol + 1);

    return (
      <Text key={lineIdx}>
        {before}
        <Text inverse>{cursorChar}</Text>
        {after}
      </Text>
    );
  };

  // Empty state with placeholder
  if (lines.length === 1 && lines[0] === '') {
    return (
      <Box flexDirection="column">
        <Text>
          <Text dimColor>{placeholder}</Text>
          <Text inverse> </Text>
        </Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      {visibleLines.map((line, idx) => {
        const actualRow = scrollOffset + idx;
        return renderLineWithCursor(line, idx, actualRow);
      })}
    </Box>
  );
};
