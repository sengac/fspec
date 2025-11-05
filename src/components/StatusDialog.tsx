/**
 * StatusDialog - Reusable progress dialog component
 *
 * Shows real-time progress for batch operations (file restore, deletion, etc.)
 * with auto-close on completion and error state handling.
 *
 * Coverage: spec/features/checkpoint-restore-progress-dialog.feature
 */

import React, { useEffect, useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { Dialog } from './Dialog';

export interface StatusDialogProps {
  /** Current item being processed */
  currentItem: string;
  /** Current item index (1-based) */
  currentIndex: number;
  /** Total number of items */
  totalItems: number;
  /** Operation status: restoring, complete, error */
  status: 'restoring' | 'complete' | 'error';
  /** Error message (required when status is 'error') */
  errorMessage?: string;
  /** Operation type verb (default: "Restoring") */
  operationType?: string;
  /** Callback when dialog closes */
  onClose: () => void;
}

export const StatusDialog: React.FC<StatusDialogProps> = ({
  currentItem,
  currentIndex,
  totalItems,
  status,
  errorMessage,
  operationType = 'Restoring',
  onClose,
}) => {
  const [countdown, setCountdown] = useState(3);

  // Auto-close after 3 seconds when status is "complete"
  useEffect(() => {
    if (status === 'complete') {
      const timer = setTimeout(() => {
        onClose();
      }, 3000);

      // Update countdown every second
      const countdownInterval = setInterval(() => {
        setCountdown(prev => Math.max(0, prev - 1));
      }, 1000);

      return () => {
        clearTimeout(timer);
        clearInterval(countdownInterval);
      };
    }
  }, [status, onClose]);

  // Handle ESC key to close (for complete and error states)
  useInput(
    (input, key) => {
      if (key.escape) {
        onClose();
      }
    },
    { isActive: status === 'complete' || status === 'error' }
  );

  return (
    <Dialog onClose={onClose} borderColor="cyan" isActive={false}>
      <Box flexDirection="column" minWidth={60} padding={1}>
        {status === 'restoring' && (
          <>
            <Box marginBottom={1}>
              <Text bold color="cyan">
                {operationType} Files
              </Text>
            </Box>
            <Box>
              <Text>
                {operationType} {currentItem}
              </Text>
            </Box>
            <Box marginTop={1}>
              <Text dimColor>
                ({currentIndex}/{totalItems})
              </Text>
            </Box>
          </>
        )}

        {status === 'complete' && (
          <>
            <Box marginBottom={1}>
              <Text bold color="green">
                {operationType.replace('ing', '')} Complete!
              </Text>
            </Box>
            <Box>
              <Text dimColor>Closing in {countdown} seconds...</Text>
            </Box>
            <Box marginTop={1}>
              <Text dimColor>Press ESC to close now</Text>
            </Box>
          </>
        )}

        {status === 'error' && (
          <>
            <Box marginBottom={1}>
              <Text bold color="red">
                Error
              </Text>
            </Box>
            <Box>
              <Text color="red">{errorMessage}</Text>
            </Box>
            <Box marginTop={1}>
              <Text dimColor>Press ESC to dismiss</Text>
            </Box>
          </>
        )}
      </Box>
    </Dialog>
  );
};
