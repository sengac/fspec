/**
 * NotificationDialog - Success/info notification display dialog
 *
 * Shows a success or info message with auto-dismiss after configurable delay.
 * User can also dismiss early with ESC key.
 * Reuses the base Dialog component for consistent modal styling.
 *
 * Coverage: spec/features/watcher-templates.feature
 */

import React, { useEffect, useState } from 'react';
import { Box, Text } from 'ink';
import { Dialog } from './Dialog';

export type NotificationType = 'success' | 'info' | 'warning';

export interface NotificationDialogProps {
  /** Notification message to display */
  message: string;
  /** Type of notification (affects color) */
  type?: NotificationType;
  /** Auto-dismiss delay in milliseconds (default: 2000, 0 = no auto-dismiss) */
  autoDismissMs?: number;
  /** Callback when dialog is dismissed */
  onClose: () => void;
}

const getColorForType = (type: NotificationType): string => {
  switch (type) {
    case 'success':
      return 'green';
    case 'info':
      return 'cyan';
    case 'warning':
      return 'yellow';
    default:
      return 'green';
  }
};

const getTitleForType = (type: NotificationType): string => {
  switch (type) {
    case 'success':
      return 'Success';
    case 'info':
      return 'Info';
    case 'warning':
      return 'Warning';
    default:
      return 'Success';
  }
};

export const NotificationDialog: React.FC<NotificationDialogProps> = ({
  message,
  type = 'success',
  autoDismissMs = 2000,
  onClose,
}) => {
  const [countdown, setCountdown] = useState(Math.ceil(autoDismissMs / 1000));
  const color = getColorForType(type);
  const title = getTitleForType(type);

  // Auto-dismiss after delay
  useEffect(() => {
    if (autoDismissMs <= 0) return;

    const timer = setTimeout(() => {
      onClose();
    }, autoDismissMs);

    // Update countdown every second
    const countdownInterval = setInterval(() => {
      setCountdown(prev => Math.max(0, prev - 1));
    }, 1000);

    return () => {
      clearTimeout(timer);
      clearInterval(countdownInterval);
    };
  }, [autoDismissMs, onClose]);

  return (
    <Dialog onClose={onClose} borderColor={color} isActive={true}>
      <Box flexDirection="column" minWidth={40} padding={1}>
        <Box marginBottom={1}>
          <Text bold color={color}>
            {title}
          </Text>
        </Box>
        <Box>
          <Text>{message}</Text>
        </Box>
        {autoDismissMs > 0 && (
          <Box marginTop={1}>
            <Text dimColor>Closing in {countdown}s... (ESC to dismiss)</Text>
          </Box>
        )}
        {autoDismissMs <= 0 && (
          <Box marginTop={1}>
            <Text dimColor>Press ESC to dismiss</Text>
          </Box>
        )}
      </Box>
    </Dialog>
  );
};
