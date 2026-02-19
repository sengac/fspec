/**
 * CreateSessionDialog.tsx - Confirmation dialog for starting a new agent conversation
 *
 * VIEWNV-001: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
 * GIT-029: Added Isolated toggle for creating sessions with git worktrees
 * TUI-067: Show context-appropriate text based on whether a work unit is selected
 *
 * This dialog is shown in two contexts:
 * 1. When pressing Enter on a work unit card - shows work-unit-aware text
 * 2. When Shift+Right past the last session - shows generic unattached text
 *
 * Features:
 * - Yes/No confirmation with Isolated toggle option
 * - Context-aware text: work-unit-linked vs unattached session
 * - Uses the base Dialog component for consistent modal styling
 * - When Isolated is ON, creates session with git worktree for safe changes
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useInputCompat, InputPriority } from '../tui/input/index';

/**
 * TUI-067: Work unit info for context-aware dialog text
 */
export interface WorkUnitInfo {
  id: string;
  title?: string;
}

export interface CreateSessionDialogProps {
  /** Callback when user confirms - starts new agent conversation */
  onConfirm: (isolated: boolean) => void;
  /** Callback when user cancels - stays at current position */
  onCancel: () => void;
  /** TUI-067: Optional work unit for context-aware dialog text */
  workUnit?: WorkUnitInfo;
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
  workUnit,
}) => {
  const [selectedButton, setSelectedButton] = useState<'yes' | 'no'>('yes');
  const [isolated, setIsolated] = useState(false);

  // TUI-067: Context-aware title and description based on work unit
  const title = workUnit ? `Work on ${workUnit.id}?` : 'Start New Agent?';
  const description = workUnit
    ? 'Start an AI session for this task'
    : 'Begin a fresh AI conversation, not linked to any task.';

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
      <Text bold>{title}</Text>
      <Text dimColor>{description}</Text>

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
