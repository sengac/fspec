/**
 * ConfirmPrompt - Interactive CLI component for yes/no confirmation
 *
 * Used during `fspec init` and `fspec remove-init-files` for user confirmation.
 * Renders standalone (outside main TUI), so uses useInputCompat's fallback mode.
 *
 * INPUT-001: Uses centralized input handling (falls back to useInput when no InputManager)
 */

import React, { useState, useEffect } from 'react';
import { Box, Text, useApp } from 'ink';
import { useInputCompat, InputPriority } from '../tui/input/index.js';

interface ConfirmPromptProps {
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onSubmit: (confirmed: boolean) => void;
}

export const ConfirmPrompt: React.FC<ConfirmPromptProps> = ({
  message,
  confirmLabel = 'Yes',
  cancelLabel = 'No',
  onSubmit,
}) => {
  const options = [
    { label: confirmLabel, value: true },
    { label: cancelLabel, value: false },
  ];
  const [cursor, setCursor] = useState(0);
  const [selected, setSelected] = useState<boolean | null>(null);
  const { exit } = useApp();

  useInputCompat({
    id: 'confirm-prompt-nav',
    priority: InputPriority.MEDIUM,
    isActive: selected === null,
    handler: (_input, key) => {
      if (selected !== null) {
        return false;
      }

      if (key.upArrow) {
        setCursor(Math.max(0, cursor - 1));
        return true;
      } else if (key.downArrow) {
        setCursor(Math.min(options.length - 1, cursor + 1));
        return true;
      } else if (key.return) {
        setSelected(options[cursor].value);
        return true;
      }
      return false;
    },
  });

  useEffect(() => {
    if (selected !== null) {
      onSubmit(selected);
      // Exit after a brief delay
      setTimeout(() => {
        exit();
      }, 100);
    }
  }, [selected, onSubmit, exit]);

  if (selected !== null) {
    const selectedOption = options.find(o => o.value === selected);
    return (
      <Box flexDirection="column">
        <Text color="green">✓ {selectedOption?.label}</Text>
      </Box>
    );
  }

  return (
    <Box flexDirection="column">
      <Text bold>{message}</Text>
      <Text dimColor>(Use ↑↓ to navigate, ENTER to select)</Text>
      <Text> </Text>
      {options.map((option, index) => {
        const isCursor = index === cursor;
        const marker = isCursor ? '▶' : ' ';

        return (
          <Text key={option.label} color={isCursor ? 'cyan' : undefined}>
            {marker} {option.label}
          </Text>
        );
      })}
    </Box>
  );
};
