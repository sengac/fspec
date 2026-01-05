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
      logger.info(`[MultiLineInput] useInput called: input=${JSON.stringify(input)} (length=${input.length}) inputCodes=${input.split('').map(c => c.charCodeAt(0)).join(',')} key=${JSON.stringify(key)} isActive=${isActive}`);
      
      // Ignore mouse escape sequences
      if (key.mouse || input.includes('[M') || input.includes('[<')) {
        logger.info('[MultiLineInput] Ignoring mouse sequence');
        return;
      }

      // Enter submits
      if (key.return) {
        logger.info('[MultiLineInput] Enter detected - calling onSubmit');
        onSubmit();
        return;
      }

      // Backspace
      if (key.backspace || key.delete) {
        // Check for Alt+Backspace (word delete)
        // Alt modifier may come through as meta in some terminals
        if (key.meta) {
          logger.info('[MultiLineInput] Alt+Backspace - deleteWordBefore');
          deleteWordBefore();
        } else {
          logger.info('[MultiLineInput] Backspace - deleteCharBefore');
          deleteCharBefore();
        }
        return;
      }

      // Delete key (forward delete)
      if (input === '\x1b[3~') {
        logger.info('[MultiLineInput] Delete key - deleteCharAt');
        deleteCharAt();
        return;
      }

      // Shift+Arrow for history navigation (check before regular arrow handling)
      if (input.includes('[1;2A') || input.includes('\x1b[1;2A')) {
        logger.info('[MultiLineInput] Shift+Up (CSI) - history prev');
        handleHistoryPrev();
        return;
      }
      if (input.includes('[1;2B') || input.includes('\x1b[1;2B')) {
        logger.info('[MultiLineInput] Shift+Down (CSI) - history next');
        handleHistoryNext();
        return;
      }
      if (key.shift && key.upArrow) {
        logger.info('[MultiLineInput] Shift+Up (key) - history prev');
        handleHistoryPrev();
        return;
      }
      if (key.shift && key.downArrow) {
        logger.info('[MultiLineInput] Shift+Down (key) - history next');
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
        logger.info('[MultiLineInput] Alt+Left - moveWordLeft');
        moveWordLeft();
        return;
      }
      if (
        input.includes('[1;3C') ||
        input.includes('\x1b[1;3C') ||
        input === '\x1bf'
      ) {
        logger.info('[MultiLineInput] Alt+Right - moveWordRight');
        moveWordRight();
        return;
      }
      if (key.meta && key.leftArrow) {
        logger.info('[MultiLineInput] Meta+Left - moveWordLeft');
        moveWordLeft();
        return;
      }
      if (key.meta && key.rightArrow) {
        logger.info('[MultiLineInput] Meta+Right - moveWordRight');
        moveWordRight();
        return;
      }

      // Arrow keys for cursor movement
      if (key.leftArrow) {
        logger.info('[MultiLineInput] Left arrow - moveCursorLeft');
        moveCursorLeft();
        return;
      }
      if (key.rightArrow) {
        logger.info('[MultiLineInput] Right arrow - moveCursorRight');
        moveCursorRight();
        return;
      }
      if (key.upArrow) {
        logger.info('[MultiLineInput] Up arrow - moveCursorUp');
        moveCursorUp();
        return;
      }
      if (key.downArrow) {
        logger.info('[MultiLineInput] Down arrow - moveCursorDown');
        moveCursorDown();
        return;
      }

      // Home/End keys
      if (key.home || input === '\x1b[H') {
        logger.info('[MultiLineInput] Home - moveCursorToLineStart');
        moveCursorToLineStart();
        return;
      }
      if (key.end || input === '\x1b[F') {
        logger.info('[MultiLineInput] End - moveCursorToLineEnd');
        moveCursorToLineEnd();
        return;
      }

      // Ignore other special keys
      if (key.escape || key.tab || key.pageUp || key.pageDown) {
        logger.info('[MultiLineInput] Ignoring special key: escape/tab/pageUp/pageDown');
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
        logger.info(`[MultiLineInput] Inserting clean string: ${JSON.stringify(clean)}`);
        // Use insertString for bulk insert to avoid stale closure issues
        insertString(clean);
      } else {
        logger.info('[MultiLineInput] No action taken - no clean chars');
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
