/**
 * CreateSessionDialog.tsx - Confirmation dialog for starting a new agent conversation
 *
 * VIEWNV-001: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
 * GIT-029: Added Isolated toggle for creating sessions with git worktrees
 *
 * This dialog is shown when the user navigates past the right edge of the session list
 * (Shift+Right from the last session or last watcher of the last session).
 *
 * Features:
 * - Yes/No confirmation with Isolated toggle option
 * - Creates a new agent conversation not linked to any work unit
 * - Uses the base Dialog component for consistent modal styling
 * - When Isolated is ON, creates session with git worktree for safe changes
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useInputCompat, InputPriority } from '../tui/input/index';

export interface CreateSessionDialogProps {
  /** Callback when user confirms - starts new agent conversation */
  onConfirm: (isolated: boolean) => void;
  /** Callback when user cancels - stays at current position */
  onCancel: () => void;
}

/**
 * CreateSessionDialog - A Yes/No confirmation dialog with Isolated toggle for starting a new agent.
 *
 * Navigation:
 * - Left/Right arrow keys: Navigate between Yes/No buttons
 * - Up/Down arrow keys: Toggle Isolated option
 * - Enter: Select the currently highlighted option
 * - ESC: Cancel (calls onCancel)
 */
export const CreateSessionDialog: React.FC<CreateSessionDialogProps> = ({
  onConfirm,
  onCancel,
}) => {
  const [selectedButton, setSelectedButton] = useState<'yes' | 'no'>('yes');
  const [isolated, setIsolated] = useState(false);

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
      } else if (key.upArrow || key.downArrow) {
        // Toggle isolated option
        setIsolated(prev => !prev);
        return true;
      } else if (key.return) {
        if (selectedButton === 'yes') {
          onConfirm(isolated);
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

      {/* Isolated Toggle */}
      <Box marginTop={1} justifyContent="center">
        <Text dimColor>Mode: </Text>
        <Text
          backgroundColor={!isolated ? 'blue' : undefined}
          color={!isolated ? 'white' : 'gray'}
          bold={!isolated}
        >
          {' '}Normal{' '}
        </Text>
        <Text dimColor> / </Text>
        <Text
          backgroundColor={isolated ? 'blue' : undefined}
          color={isolated ? 'white' : 'gray'}
          bold={isolated}
        >
          {' '}Isolated{' '}
        </Text>
      </Box>
      {isolated && (
        <Box justifyContent="center">
          <Text dimColor>
            Isolated: Changes made in a separate git worktree
          </Text>
        </Box>
      )}

      {/* Yes/No Buttons */}
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
        <Text dimColor>← → Select | ↑ ↓ Toggle Mode | Enter Confirm | Esc Cancel</Text>
      </Box>
    </Dialog>
  );
};
