import React, { useState } from 'react';
import { Text, useInput } from 'ink';
import { Dialog } from './Dialog';

type ConfirmMode = 'yesno' | 'typed' | 'keypress';
type RiskLevel = 'low' | 'medium' | 'high';

export interface ConfirmationDialogProps {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmMode?: ConfirmMode;
  typedPhrase?: string;
  riskLevel?: RiskLevel;
  description?: string;
}

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
 */
export const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  message,
  onConfirm,
  onCancel,
  confirmMode = 'yesno',
  typedPhrase,
  riskLevel,
  description,
}) => {
  const [inputValue, setInputValue] = useState('');

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
  useInput((input, key) => {
    if (confirmMode === 'yesno') {
      // Y/N mode
      if (input.toLowerCase() === 'y') {
        onConfirm();
      } else if (input.toLowerCase() === 'n') {
        onCancel();
      }
    } else if (confirmMode === 'typed') {
      // Typed phrase mode
      if (key.return) {
        if (inputValue === typedPhrase) {
          onConfirm();
        }
      } else if (key.backspace || key.delete) {
        setInputValue((prev) => prev.slice(0, -1));
      } else if (input && !key.ctrl && !key.meta) {
        setInputValue((prev) => prev + input);
      }
    } else if (confirmMode === 'keypress') {
      // Single keypress mode
      onConfirm();
    }
  });

  return (
    <Dialog borderColor={borderColor} onClose={onCancel}>
      <Text bold>{message}</Text>
      {description && <Text dimColor>{description}</Text>}
      {description && <Text> </Text>}

      {confirmMode === 'yesno' && (
        <Text dimColor>Press Y to confirm, N to cancel, ESC to cancel</Text>
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
