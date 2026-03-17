/**
 * RoleDialog - Modal dialog for editing session role (system prompt overlay)
 *
 * AMGR-012: Allows users to set/edit/clear a role via /role command.
 * TUI-082: Remove button for quick role clearing when editing existing role.
 * Uses the base Dialog component and useMultiLineInput hook.
 *
 * Features:
 * - Multi-line text area (6 visible lines) for role text
 * - Pre-populated with current role if one exists
 * - Tab cycles focus: textarea → OK → [Remove] → Cancel → textarea
 * - Remove button only shown when editing an existing role (red styling)
 * - Enter inserts newline in textarea, activates button when focused
 * - Left/right arrows navigate between visible buttons
 * - ESC cancels (handled by base Dialog)
 * - Empty submission clears the role
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useMultiLineInput } from '../tui/hooks/useMultiLineInput';
import { useInputCompat, InputPriority } from '../tui/input/index';

type FocusArea = 'textarea' | 'ok' | 'remove' | 'cancel';

export interface RoleDialogProps {
  /** Initial role text (pre-populated in textarea) */
  initialRole?: string;
  /** Called when user submits the role (empty string = clear) */
  onSubmit: (role: string) => void;
  /** Called when user cancels (ESC or Cancel button) */
  onClose: () => void;
}

const VISIBLE_LINES = 6;

export const RoleDialog: React.FC<RoleDialogProps> = ({
  initialRole = '',
  onSubmit,
  onClose,
}) => {
  const [focus, setFocus] = useState<FocusArea>('textarea');

  // TUI-082: Show Remove button only when editing an existing role
  const showRemove = initialRole.length > 0;

  const {
    visibleLines,
    scrollOffset,
    value,
    cursorRow,
    cursorCol,
    moveCursorLeft,
    moveCursorRight,
    moveCursorUp,
    moveCursorDown,
    moveCursorToLineStart,
    moveCursorToLineEnd,
    moveWordLeft,
    moveWordRight,
    insertString,
    insertNewline,
    deleteCharBefore,
    deleteCharAt,
    deleteWordBefore,
  } = useMultiLineInput({
    initialValue: initialRole,
    maxVisibleLines: VISIBLE_LINES,
  });

  // TUI-082: Build ordered list of buttons for tab/arrow navigation
  const buttonOrder: FocusArea[] = showRemove
    ? ['ok', 'remove', 'cancel']
    : ['ok', 'cancel'];

  const nextButton = (current: FocusArea): FocusArea => {
    const idx = buttonOrder.indexOf(current);
    if (idx === -1 || idx === buttonOrder.length - 1) {
      return 'textarea';
    }
    return buttonOrder[idx + 1];
  };

  const nextButtonRight = (current: FocusArea): FocusArea => {
    const idx = buttonOrder.indexOf(current);
    if (idx < buttonOrder.length - 1) {
      return buttonOrder[idx + 1];
    }
    return current;
  };

  const prevButtonLeft = (current: FocusArea): FocusArea => {
    const idx = buttonOrder.indexOf(current);
    if (idx > 0) {
      return buttonOrder[idx - 1];
    }
    return current;
  };

  useInputCompat({
    id: 'role-dialog-input',
    priority: InputPriority.CRITICAL,
    isActive: true,
    handler: (input, key) => {
      // Tab cycles focus: textarea → ok → [remove] → cancel → textarea
      if (key.tab) {
        setFocus(prev => {
          if (prev === 'textarea') {
            return 'ok';
          }
          return nextButton(prev);
        });
        return true;
      }

      // ESC handled by Dialog base — but also handle here for robustness
      if (key.escape) {
        onClose();
        return true;
      }

      if (focus === 'textarea') {
        // Text area input handling
        if (key.return) {
          insertNewline();
          return true;
        }

        if (key.backspace || key.delete) {
          if (key.meta) {
            deleteWordBefore();
          } else {
            deleteCharBefore();
          }
          return true;
        }

        // Forward delete
        if (input === '\x1b[3~') {
          deleteCharAt();
          return true;
        }

        // Alt+Arrow for word movement
        if (input.includes('[1;3D') || input === '\x1bb' || (key.meta && key.leftArrow)) {
          moveWordLeft();
          return true;
        }
        if (input.includes('[1;3C') || input === '\x1bf' || (key.meta && key.rightArrow)) {
          moveWordRight();
          return true;
        }

        // Arrow keys
        if (key.leftArrow) {
          moveCursorLeft();
          return true;
        }
        if (key.rightArrow) {
          moveCursorRight();
          return true;
        }
        if (key.upArrow) {
          moveCursorUp();
          return true;
        }
        if (key.downArrow) {
          moveCursorDown();
          return true;
        }

        // Home/End
        if (key.home || input === '\x1b[H') {
          moveCursorToLineStart();
          return true;
        }
        if (key.end || input === '\x1b[F') {
          moveCursorToLineEnd();
          return true;
        }

        // Ignore other special keys in textarea
        if (key.escape || key.pageUp || key.pageDown) {
          return false;
        }

        // Printable characters
        const clean = input
          .split('')
          .filter(ch => {
            const code = ch.charCodeAt(0);
            return code >= 32 && code !== 127;
          })
          .join('');

        if (clean) {
          insertString(clean);
          return true;
        }

        return true; // Consume all input when textarea focused
      }

      // Button row focus handling — works for ok, remove, and cancel
      if (focus === 'ok' || focus === 'remove' || focus === 'cancel') {
        if (key.leftArrow) {
          setFocus(prevButtonLeft(focus));
          return true;
        }
        if (key.rightArrow) {
          setFocus(nextButtonRight(focus));
          return true;
        }
        if (key.return) {
          if (focus === 'ok') {
            onSubmit(value);
          } else if (focus === 'remove') {
            // TUI-082: Remove = submit empty string to clear role
            onSubmit('');
          } else {
            onClose();
          }
          return true;
        }
        return true; // Consume input when buttons focused
      }

      return true;
    },
  });

  // Render text area lines with cursor
  const renderLine = (line: string, idx: number): React.ReactNode => {
    const actualRow = scrollOffset + idx;
    const isCursorRow = actualRow === cursorRow && focus === 'textarea';

    if (!isCursorRow) {
      return <Text key={idx}>{line || ' '}</Text>;
    }

    const before = line.slice(0, cursorCol);
    const cursorChar = line[cursorCol] ?? ' ';
    const after = line.slice(cursorCol + 1);

    return (
      <Text key={idx}>
        {before}
        <Text inverse>{cursorChar}</Text>
        {after}
      </Text>
    );
  };

  // Pad visible lines to always show VISIBLE_LINES
  const paddedLines = [...visibleLines];
  while (paddedLines.length < VISIBLE_LINES) {
    paddedLines.push('');
  }

  return (
    <Dialog onClose={onClose} borderColor="cyan" isActive={false}>
      <Box flexDirection="column" minWidth={50}>
        <Box marginBottom={1}>
          <Text bold color="cyan">
            Role
          </Text>
        </Box>

        {/* Text area */}
        <Box
          flexDirection="column"
          borderStyle="single"
          borderColor={focus === 'textarea' ? 'cyan' : 'gray'}
          paddingX={1}
        >
          {paddedLines.length === 1 && paddedLines[0] === '' && focus === 'textarea' ? (
            <Text>
              <Text dimColor>Enter role text...</Text>
              <Text inverse> </Text>
            </Text>
          ) : paddedLines.map((line, idx) => (
            idx < visibleLines.length
              ? renderLine(line, idx)
              : <Text key={idx} dimColor> </Text>
          ))}
        </Box>

        {/* OK / [Remove] / Cancel Buttons */}
        <Box marginTop={1} justifyContent="center">
          <Box marginX={1}>
            <Text
              backgroundColor={focus === 'ok' ? 'blue' : undefined}
              color={focus === 'ok' ? 'white' : 'gray'}
              bold={focus === 'ok'}
            >
              {' '}OK{' '}
            </Text>
          </Box>
          {/* TUI-082: Remove button — only visible when editing existing role */}
          {showRemove && (
            <Box marginX={1}>
              <Text
                backgroundColor={focus === 'remove' ? 'red' : undefined}
                color={focus === 'remove' ? 'white' : 'red'}
                bold={focus === 'remove'}
              >
                {' '}Remove{' '}
              </Text>
            </Box>
          )}
          <Box marginX={1}>
            <Text
              backgroundColor={focus === 'cancel' ? 'blue' : undefined}
              color={focus === 'cancel' ? 'white' : 'gray'}
              bold={focus === 'cancel'}
            >
              {' '}Cancel{' '}
            </Text>
          </Box>
        </Box>

        <Box marginTop={1} justifyContent="center">
          <Text dimColor>Tab Switch Focus │ Enter Newline/Select │ Esc Cancel</Text>
        </Box>
      </Box>
    </Dialog>
  );
};
