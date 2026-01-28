/**
 * CreateSessionDialog.tsx - Confirmation dialog for starting a new agent conversation
 *
 * VIEWNV-001: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
 *
 * This dialog is shown when the user navigates past the right edge of the session list
 * (Shift+Right from the last session or last watcher of the last session).
 *
 * Features:
 * - Simple Yes/No confirmation
 * - Creates a new agent conversation not linked to any work unit
 * - Uses the base Dialog component for consistent modal styling
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useInputCompat, InputPriority } from '../tui/input/index';

export interface CreateSessionDialogProps {
  /** Callback when user confirms - starts new agent conversation */
  onConfirm: () => void;
  /** Callback when user cancels - stays at current position */
  onCancel: () => void;
}

/**
 * CreateSessionDialog - A simple Yes/No confirmation dialog for starting a new agent.
 *
 * Uses Left/Right arrow keys to navigate between buttons.
 * Enter selects the currently highlighted option.
 * ESC cancels (calls onCancel).
 */
export const CreateSessionDialog: React.FC<CreateSessionDialogProps> = ({
  onConfirm,
  onCancel,
}) => {
  const [selectedButton, setSelectedButton] = useState<'yes' | 'no'>('yes');

  useInputCompat({
    id: 'create-session-dialog-nav',
    priority: InputPriority.CRITICAL,
    isActive: true,
    handler: (_input, key) => {
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
      // ESC is handled by Dialog component via onClose
      return false;
    },
  });

  return (
    <Dialog onClose={onCancel} borderColor="cyan">
      <Text bold>Start New Agent?</Text>
      <Text dimColor>Begin a fresh AI conversation, not linked to any task.</Text>
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
    </Dialog>
  );
};
