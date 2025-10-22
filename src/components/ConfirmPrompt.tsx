import React, { useState, useEffect } from 'react';
import { Box, Text, useInput, useApp } from 'ink';

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

  useInput((input, key) => {
    if (selected !== null) {
      return;
    }

    if (key.upArrow) {
      setCursor(Math.max(0, cursor - 1));
    } else if (key.downArrow) {
      setCursor(Math.min(options.length - 1, cursor + 1));
    } else if (key.return) {
      setSelected(options[cursor].value);
    }
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
