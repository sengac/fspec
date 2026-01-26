/**
 * ThreeButtonDialog - A dialog with three horizontal button options.
 *
 * Uses Left/Right arrow keys to navigate between buttons.
 * Enter selects the currently highlighted option.
 * ESC cancels (calls onCancel).
 *
 * Extends base Dialog component for modal overlay infrastructure.
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useInputCompat, InputPriority } from '../tui/input/index.js';

export interface ThreeButtonDialogProps {
  message: string;
  options: [string, string, string]; // Exactly three options
  onSelect: (index: number, option: string) => void;
  onCancel: () => void;
  defaultSelectedIndex?: number;
  description?: string;
}

/**
 * ThreeButtonDialog - A dialog with three horizontal button options.
 */
export const ThreeButtonDialog: React.FC<ThreeButtonDialogProps> = ({
  message,
  options,
  onSelect,
  onCancel,
  defaultSelectedIndex = 0,
  description,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(defaultSelectedIndex);

  useInputCompat({
    id: 'three-button-dialog-nav',
    priority: InputPriority.CRITICAL,
    isActive: true,
    handler: (_input, key) => {
      if (key.leftArrow) {
        setSelectedIndex(prev => (prev > 0 ? prev - 1 : options.length - 1));
        return true;
      } else if (key.rightArrow) {
        setSelectedIndex(prev => (prev < options.length - 1 ? prev + 1 : 0));
        return true;
      } else if (key.return) {
        onSelect(selectedIndex, options[selectedIndex]);
        return true;
      }
      // ESC is handled by Dialog component via onClose
      return false;
    },
  });

  return (
    <Dialog onClose={onCancel} borderColor="yellow">
      <Text bold>{message}</Text>
      {description && <Text dimColor>{description}</Text>}
      <Box marginTop={1} justifyContent="center">
        {options.map((option, idx) => (
          <Box key={idx} marginX={1}>
            <Text
              backgroundColor={idx === selectedIndex ? 'blue' : undefined}
              color={idx === selectedIndex ? 'white' : 'gray'}
              bold={idx === selectedIndex}
            >
              {' '}
              {option}{' '}
            </Text>
          </Box>
        ))}
      </Box>
      <Box marginTop={1} justifyContent="center">
        <Text dimColor>← → Navigate | Enter Select | Esc Cancel</Text>
      </Box>
    </Dialog>
  );
};
