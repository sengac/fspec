/**
 * ConfirmationDialog - handles ONLY confirmation-specific logic.
 *
 * Responsibilities:
 * - Confirmation mode logic (yesno/typed/keypress)
 * - Input validation for typed mode
 * - Mapping riskLevel to borderColor for Dialog
 * - onConfirm/onCancel callback management
 *
 * Does NOT handle:
 * - Modal overlay rendering (delegated to Dialog)
 * - ESC key handling (delegated to Dialog)
 * - Border rendering (delegated to Dialog)
 *
 * Uses composition pattern - wraps Dialog component.
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useInputCompat, InputPriority } from '../tui/input/index';

type ConfirmMode = 'yesno' | 'typed' | 'keypress' | 'visual' | 'triple';
type RiskLevel = 'low' | 'medium' | 'high';
type TripleChoice = 'allowOnce' | 'allowSession' | 'deny';

export interface ConfirmationDialogProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmMode?: ConfirmMode;
  typedPhrase?: string;
  riskLevel?: RiskLevel;
  description?: string;
  /** Callback for triple mode - called with user's choice */
  onTripleConfirm?: (choice: TripleChoice) => void;
}

/**
 * ConfirmationDialog - handles ONLY confirmation-specific logic.
 */
export const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  message,
  onConfirm,
  onCancel,
  confirmMode = 'yesno',
  typedPhrase,
  riskLevel,
  description,
  onTripleConfirm,
}) => {
  const [inputValue, setInputValue] = useState('');
  const [selectedButton, setSelectedButton] = useState<'yes' | 'no'>('yes'); // Default to 'yes'
  const [tripleSelection, setTripleSelection] = useState<TripleChoice>('allowOnce'); // Default for triple mode

  // Map riskLevel to borderColor for Dialog
  const getBorderColor = (): string | undefined => {
    if (!riskLevel) return undefined;
    switch (riskLevel) {
      case 'low':
        return 'green';
      case 'medium':
        return 'yellow';
      case 'high':
        return 'red';
      default:
        return undefined;
    }
  };

  const borderColor = getBorderColor();

  // Handle confirmation-specific key logic
  useInputCompat({
    id: 'confirmation-dialog-input',
    priority: InputPriority.CRITICAL,
    isActive: true,
    handler: (input, key) => {
      if (confirmMode === 'yesno') {
        // Y/N mode
        if (input.toLowerCase() === 'y') {
          onConfirm();
          return true;
        } else if (input.toLowerCase() === 'n') {
          onCancel();
          return true;
        }
      } else if (confirmMode === 'visual') {
        // Visual button mode (like CreateSessionDialog)
        if (key.leftArrow) {
          setSelectedButton('yes');
          return true;
        } else if (key.rightArrow) {
          setSelectedButton('no');
          return true;
        } else if (key.return) {
          if (selectedButton === 'yes') {
            onConfirm();
          } else {
            onCancel();
          }
          return true;
        }
      } else if (confirmMode === 'triple') {
        // Triple button mode: Allow Once / Allow Session / Deny
        // BLOCK-005: Sensitive path prompts with session allowance
        const tripleOptions: TripleChoice[] = ['allowOnce', 'allowSession', 'deny'];
        const currentIndex = tripleOptions.indexOf(tripleSelection);
        
        if (key.leftArrow) {
          // Wrap around: if at first, go to last
          const newIndex = currentIndex <= 0 ? tripleOptions.length - 1 : currentIndex - 1;
          setTripleSelection(tripleOptions[newIndex]);
          return true;
        } else if (key.rightArrow) {
          // Wrap around: if at last, go to first
          const newIndex = currentIndex >= tripleOptions.length - 1 ? 0 : currentIndex + 1;
          setTripleSelection(tripleOptions[newIndex]);
          return true;
        } else if (key.return) {
          if (onTripleConfirm) {
            onTripleConfirm(tripleSelection);
          }
          return true;
        }
      } else if (confirmMode === 'typed') {
        // Typed phrase mode (case-insensitive)
        if (key.return) {
          if (inputValue.toLowerCase() === typedPhrase?.toLowerCase()) {
            onConfirm();
          }
          return true;
        } else if (key.backspace || key.delete) {
          setInputValue((prev) => prev.slice(0, -1));
          return true;
        } else if (input && !key.ctrl && !key.meta) {
          setInputValue((prev) => prev + input);
          return true;
        }
      } else if (confirmMode === 'keypress') {
        // Single keypress mode
        onConfirm();
        return true;
      }
      return false;
    },
  });

  return (
    <Dialog borderColor={borderColor} onClose={onCancel}>
      <Text bold>{message}</Text>
      {description && <Text dimColor>{description}</Text>}
      {description && <Text> </Text>}

      {confirmMode === 'yesno' && (
        <Text dimColor>Press Y to confirm, N to cancel, ESC to cancel</Text>
      )}

      {confirmMode === 'visual' && (
        <>
          <Box marginTop={1} justifyContent="center">
            <Box marginX={1}>
              <Text
                backgroundColor={selectedButton === 'yes' ? 'blue' : undefined}
                color={selectedButton === 'yes' ? 'white' : 'gray'}
                bold={selectedButton === 'yes'}
              >
                {' '}Yes{' '}
              </Text>
            </Box>
            <Box marginX={1}>
              <Text
                backgroundColor={selectedButton === 'no' ? 'blue' : undefined}
                color={selectedButton === 'no' ? 'white' : 'gray'}
                bold={selectedButton === 'no'}
              >
                {' '}No{' '}
              </Text>
            </Box>
          </Box>
          <Box marginTop={1} justifyContent="center">
            <Text dimColor>← → Navigate | Enter Select | Esc Cancel</Text>
          </Box>
        </>
      )}

      {confirmMode === 'triple' && (
        <>
          <Box marginTop={1} justifyContent="center">
            <Box marginX={1}>
              <Text
                backgroundColor={tripleSelection === 'allowOnce' ? 'green' : undefined}
                color={tripleSelection === 'allowOnce' ? 'white' : 'gray'}
                bold={tripleSelection === 'allowOnce'}
              >
                {' '}Allow Once{' '}
              </Text>
            </Box>
            <Box marginX={1}>
              <Text
                backgroundColor={tripleSelection === 'allowSession' ? 'blue' : undefined}
                color={tripleSelection === 'allowSession' ? 'white' : 'gray'}
                bold={tripleSelection === 'allowSession'}
              >
                {' '}Allow Session{' '}
              </Text>
            </Box>
            <Box marginX={1}>
              <Text
                backgroundColor={tripleSelection === 'deny' ? 'red' : undefined}
                color={tripleSelection === 'deny' ? 'white' : 'gray'}
                bold={tripleSelection === 'deny'}
              >
                {' '}Deny{' '}
              </Text>
            </Box>
          </Box>
          <Box marginTop={1} justifyContent="center">
            <Text dimColor>← → Navigate | Enter Select | Esc Cancel</Text>
          </Box>
        </>
      )}

      {confirmMode === 'typed' && (
        <>
          <Text dimColor>
            Type &quot;{typedPhrase}&quot; to confirm (ESC to cancel):
          </Text>
          <Text color="cyan">{inputValue}</Text>
        </>
      )}

      {confirmMode === 'keypress' && (
        <Text dimColor>Press any key to confirm, ESC to cancel</Text>
      )}
    </Dialog>
  );
};
