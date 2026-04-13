/**
 * CreateSessionDialog.tsx - Confirmation dialog for starting a new agent conversation
 *
 * VIEWNV-001: Unified Shift+Arrow Navigation Across BoardView, AgentView, and SplitPaneView
 * GIT-029: Added Isolated option for creating sessions with git worktrees
 * TUI-067: Show context-appropriate text based on whether a work unit is selected
 * TUI-090: Simplified to 3 flat options: Yes, Yes - Isolated, Cancel
 *
 * This dialog is shown in two contexts:
 * 1. When pressing Enter on a work unit card - shows work-unit-aware text
 * 2. When Shift+Right past the last session - shows generic unattached text
 *
 * Features:
 * - 3 flat options: Yes, Yes - Isolated, Cancel
 * - Context-aware text: work-unit-linked vs unattached session
 * - Uses the base Dialog component for consistent modal styling
 * - Left/Right arrows navigate cyclically between options
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';
import { useInputCompat, InputPriority } from '../tui/input/index';

/** The three available options in the dialog */
type DialogOption = 'yes' | 'yes-isolated' | 'cancel';

/** Ordered list of options for cyclic navigation */
const OPTIONS: DialogOption[] = ['yes', 'yes-isolated', 'cancel'];

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
 * CreateSessionDialog - A 3-option dialog for starting a new agent.
 *
 * TUI-090: Simplified from Yes/No + toggle to 3 flat options.
 *
 * Navigation:
 * - Left/Right arrow keys: Navigate between Yes / Yes - Isolated / Cancel
 * - Enter: Confirm the currently highlighted option
 * - ESC: Cancel (calls onCancel)
 */
export const CreateSessionDialog: React.FC<CreateSessionDialogProps> = ({
  onConfirm,
  onCancel,
  workUnit,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);

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
      if (key.rightArrow) {
        setSelectedIndex(prev => (prev + 1) % OPTIONS.length);
        return true;
      } else if (key.leftArrow) {
        setSelectedIndex(prev => (prev - 1 + OPTIONS.length) % OPTIONS.length);
        return true;
      } else if (key.return) {
        const selected = OPTIONS[selectedIndex];
        if (selected === 'yes') {
          onConfirm(false);
        } else if (selected === 'yes-isolated') {
          onConfirm(true);
        } else {
          onCancel();
        }
        return true;
      }
      // ESC is handled by Dialog component via onClose
      return false;
    },
  });

  /**
   * Render a single option button with highlight styling.
   */
  const renderOption = (option: DialogOption, label: string): React.ReactNode => {
    const isSelected = OPTIONS[selectedIndex] === option;
    return (
      <Box marginX={1} key={option}>
        <Text
          backgroundColor={isSelected ? 'blue' : undefined}
          color={isSelected ? 'white' : 'gray'}
          bold={isSelected}
        >
          {` ${label} `}
        </Text>
      </Box>
    );
  };

  return (
    <Dialog onClose={onCancel} borderColor="cyan">
      <Text bold>{title}</Text>
      <Text dimColor>{description}</Text>

      {/* TUI-090: Three flat options */}
      <Box marginTop={1} justifyContent="center">
        {renderOption('yes', 'Yes')}
        {renderOption('yes-isolated', 'Yes - Isolated')}
        {renderOption('cancel', 'Cancel')}
      </Box>

      <Box marginTop={1} justifyContent="center">
        <Text dimColor>← → Select | Enter Confirm | Esc Cancel</Text>
      </Box>
    </Dialog>
  );
};
