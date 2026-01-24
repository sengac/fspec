/**
 * CreateSessionDialog.tsx - Confirmation dialog for creating a new session
 *
 * VIEWNV-001: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
 *
 * This dialog is shown when the user navigates past the right edge of the session list
 * (Shift+Right from the last session or last watcher of the last session).
 *
 * Features:
 * - Simple Yes/No confirmation
 * - Creates a new unattached session (not attached to any work unit)
 * - Uses the base Dialog component for consistent modal styling
 */

import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { Dialog } from './Dialog';

export interface CreateSessionDialogProps {
  /** Callback when user confirms - creates new session */
  onConfirm: () => void;
  /** Callback when user cancels - stays at current position */
  onCancel: () => void;
}

/**
 * CreateSessionDialog - A simple Yes/No confirmation dialog for creating a new session.
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

  useInput((input, key) => {
    if (key.leftArrow) {
      setSelectedButton('yes');
    } else if (key.rightArrow) {
      setSelectedButton('no');
    } else if (key.return) {
      if (selectedButton === 'yes') {
        onConfirm();
      } else {
        onCancel();
      }
    }
    // ESC is handled by Dialog component via onClose
  });

  return (
    <Dialog onClose={onCancel} borderColor="cyan">
      <Text bold>Would you like to create a new session?</Text>
      <Text dimColor>A new unattached session will be created.</Text>
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
