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
  /** Default thinking level for new sessions (null if not set) */
  defaultLevel: JsThinkingLevel | null;
  /** Called when user selects a level (Enter key) */
  onSelect: (level: JsThinkingLevel) => void;
  /** Called when user sets a default level (D key) */
  onSetDefault: (level: JsThinkingLevel) => void;
  /** Called when user cancels (Escape key) */
  onClose: () => void;
}

export const ThinkingLevelDialog: React.FC<ThinkingLevelDialogProps> = ({
  currentLevel,
  defaultLevel,
  onSelect,
  onSetDefault,
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
    handler: (input, key) => {
      if (key.escape) {
        onClose();
        return true; // Consumed
      }

      if (key.return) {
        onSelect(selectedIndex as JsThinkingLevel);
        onClose();
        return true; // Consumed
      }

      // Handle D key - set current selection as default
      if (input.toLowerCase() === 'd') {
        onSetDefault(selectedIndex as JsThinkingLevel);
        return true; // Consumed (dialog stays open)
      }

      if (key.upArrow) {
        // Wrap around: Off (0) -> High (last)
        const lastIndex = THINKING_LEVELS.length - 1;
        setSelectedIndex(prev => (prev === 0 ? lastIndex : prev - 1));
        return true; // Consumed
      }

      if (key.downArrow) {
        // Wrap around: High (last) -> Off (0)
        const lastIndex = THINKING_LEVELS.length - 1;
        setSelectedIndex(prev => (prev === lastIndex ? 0 : prev + 1));
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
            const isDefault = defaultLevel !== null && index === defaultLevel;

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
                  {isDefault ? ' (default)' : ''}
                </Text>
              </Box>
            );
          })}
        </Box>

        <Box marginTop={1} justifyContent="center">
          <Text dimColor>↑↓ Navigate │ Enter Select │ D Set Default │ Esc Close</Text>
        </Box>
      </Box>
    </Dialog>
  );
};
