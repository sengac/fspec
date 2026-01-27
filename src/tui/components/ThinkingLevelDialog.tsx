/**
 * ThinkingLevelDialog - Modal dialog for selecting thinking level
 *
 * TUI-054: Allows users to set the base thinking level via /thinking command.
 * The selected level persists for the session and affects all subsequent requests.
 *
 * Effective level = max(baseLevel, detectedLevelFromText)
 * Exception: Disable keywords (quickly, briefly) always force Off.
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 * to ensure this dialog captures input when visible.
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from '../../components/Dialog';
import { useInputCompat, InputPriority } from '../input/index';
import { JsThinkingLevel } from '../../utils/thinkingLevel';

/** Thinking level option with display label and description */
interface ThinkingLevelOption {
  level: JsThinkingLevel;
  label: string;
  description: string;
}

/** Available thinking levels with descriptions */
const THINKING_LEVELS: ThinkingLevelOption[] = [
  {
    level: JsThinkingLevel.Off,
    label: 'Off',
    description: 'No extended thinking',
  },
  {
    level: JsThinkingLevel.Low,
    label: 'Low',
    description: '~4K tokens, quick analysis',
  },
  {
    level: JsThinkingLevel.Medium,
    label: 'Medium',
    description: '~10K tokens, balanced',
  },
  {
    level: JsThinkingLevel.High,
    label: 'High',
    description: '~32K tokens, deep reasoning',
  },
];

export interface ThinkingLevelDialogProps {
  /** Current base thinking level (used as initial selection) */
  currentLevel: JsThinkingLevel;
  /** Called when user selects a level (Enter key) */
  onSelect: (level: JsThinkingLevel) => void;
  /** Called when user cancels (Escape key) */
  onClose: () => void;
}

export const ThinkingLevelDialog: React.FC<ThinkingLevelDialogProps> = ({
  currentLevel,
  onSelect,
  onClose,
}) => {
  // Initialize selection to current level
  const [selectedIndex, setSelectedIndex] = useState(currentLevel);

  // Handle keyboard input with CRITICAL priority
  // Modal dialogs must capture all input when visible
  useInputCompat({
    id: 'thinking-level-dialog',
    priority: InputPriority.CRITICAL,
    description: 'Thinking level selection dialog',
    handler: (_input, key) => {
      if (key.escape) {
        onClose();
        return true; // Consumed
      }

      if (key.return) {
        onSelect(selectedIndex as JsThinkingLevel);
        onClose();
        return true; // Consumed
      }

      if (key.upArrow) {
        // Wrap around: Off (0) -> High (3)
        setSelectedIndex(prev => (prev === 0 ? 3 : prev - 1));
        return true; // Consumed
      }

      if (key.downArrow) {
        // Wrap around: High (3) -> Off (0)
        setSelectedIndex(prev => (prev === 3 ? 0 : prev + 1));
        return true; // Consumed
      }

      // Consume all other input when dialog is open
      return true;
    },
  });

  return (
    <Dialog onClose={onClose} borderColor="yellow" isActive={false}>
      <Box flexDirection="column" minWidth={45}>
        <Box marginBottom={1}>
          <Text bold color="yellow">
            Thinking Level
          </Text>
        </Box>

        <Box flexDirection="column">
          {THINKING_LEVELS.map((option, index) => {
            const isSelected = index === selectedIndex;

            return (
              <Box key={option.level}>
                <Text
                  backgroundColor={isSelected ? 'yellow' : undefined}
                  color={isSelected ? 'black' : 'white'}
                >
                  {isSelected ? '▸ ' : '  '}
                  {option.label}
                </Text>
                <Text dimColor={!isSelected}>
                  {' - '}
                  {option.description}
                </Text>
              </Box>
            );
          })}
        </Box>

        <Box marginTop={1} justifyContent="center">
          <Text dimColor>↑↓ Navigate │ Enter Select │ Esc Close</Text>
        </Box>
      </Box>
    </Dialog>
  );
};
