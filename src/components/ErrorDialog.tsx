/**
 * ErrorDialog - Simple error display dialog
 *
 * Shows an error message with dismissal via ESC key.
 * Reuses the base Dialog component for consistent modal styling.
 */

import React from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';

export interface ErrorDialogProps {
  /** Error message to display */
  message: string;
  /** Callback when dialog is dismissed */
  onClose: () => void;
}

export const ErrorDialog: React.FC<ErrorDialogProps> = ({ message, onClose }) => {
  return (
    <Dialog onClose={onClose} borderColor="red" isActive={true}>
      <Box flexDirection="column" minWidth={40} padding={1}>
        <Box marginBottom={1}>
          <Text bold color="red">
            Error
          </Text>
        </Box>
        <Box>
          <Text color="red">{message}</Text>
        </Box>
        <Box marginTop={1}>
          <Text dimColor>Press ESC to dismiss</Text>
        </Box>
      </Box>
    </Dialog>
  );
};
