/**
 * MultiLineInput - Multi-line text input component with cursor navigation
 *
 * Implements TUI-039: Multi-line text input with cursor navigation for AgentView
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
import { Box, Text, useInput } from 'ink';
import { useMultiLineInput } from '../hooks/useMultiLineInput';
import { logger } from '../../utils/logger';

export interface MultiLineInputProps {
  value: string;
  onChange: (value: string) => void;
  onSubmit: () => void;
  placeholder?: string;
  isActive?: boolean;
  maxVisibleLines?: number;
  onHistoryPrev?: () => void;
  onHistoryNext?: () => void;
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
  const lastExternalValueRef = useRef(value);
  useEffect(() => {
    if (value !== lastExternalValueRef.current && value !== hookValue) {
      setValue(value);
      lastExternalValueRef.current = value;
    }
  }, [value, hookValue, setValue]);

  // Notify parent of internal value changes
  useEffect(() => {
    if (hookValue !== lastExternalValueRef.current) {
      lastExternalValueRef.current = hookValue;
      onChange(hookValue);
    }
  }, [hookValue, onChange]);

  useInput(
    (input, key) => {
      // Log ALL keyboard input for debugging
      
      
      // Ignore mouse escape sequences
      if (key.mouse || input.includes('[M') || input.includes('[<')) {
        
        return;
      }

      // Enter submits
      if (key.return) {

        onSubmit();
        return;
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
        return;
      }

      // Delete key (forward delete)
      if (input === '\x1b[3~') {
        
        deleteCharAt();
        return;
      }

      // Shift+Arrow for history navigation (check before regular arrow handling)
      if (input.includes('[1;2A') || input.includes('\x1b[1;2A')) {
        
        handleHistoryPrev();
        return;
      }
      if (input.includes('[1;2B') || input.includes('\x1b[1;2B')) {
        
        handleHistoryNext();
        return;
      }
      if (key.shift && key.upArrow) {
        
        handleHistoryPrev();
        return;
      }
      if (key.shift && key.downArrow) {
        
        handleHistoryNext();
        return;
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
        return;
      }
      if (
        input.includes('[1;3C') ||
        input.includes('\x1b[1;3C') ||
        input === '\x1bf'
      ) {
        
        moveWordRight();
        return;
      }
      if (key.meta && key.leftArrow) {
        
        moveWordLeft();
        return;
      }
      if (key.meta && key.rightArrow) {
        
        moveWordRight();
        return;
      }

      // Arrow keys for cursor movement
      if (key.leftArrow) {
        
        moveCursorLeft();
        return;
      }
      if (key.rightArrow) {
        
        moveCursorRight();
        return;
      }
      if (key.upArrow) {
        
        moveCursorUp();
        return;
      }
      if (key.downArrow) {
        
        moveCursorDown();
        return;
      }

      // Home/End keys
      if (key.home || input === '\x1b[H') {
        
        moveCursorToLineStart();
        return;
      }
      if (key.end || input === '\x1b[F') {
        
        moveCursorToLineEnd();
        return;
      }

      // Ignore other special keys
      if (key.escape || key.tab || key.pageUp || key.pageDown) {
        
        return;
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
      } else {
        
      }
    },
    { isActive }
  );

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
