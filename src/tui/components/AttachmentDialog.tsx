/**
 * AttachmentDialog - Modal dialog for selecting attachments to open
 *
 * Displays a scrollable list of attachments with keyboard navigation.
 * Implements TUI-019: Attachment selection dialog with keyboard navigation
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 * to ensure this dialog captures input when visible.
 */

import React, { useState } from 'react';
import { Box, Text } from 'ink';
import { basename } from 'path';
import { Dialog } from '../../components/Dialog.js';
import { useInputCompat, InputPriority } from '../input/index.js';

// Configuration: Maximum attachments visible in viewport before scrolling
const DEFAULT_VIEWPORT_HEIGHT = 10;

export interface AttachmentDialogProps {
  attachments: string[];
  onSelect: (attachment: string) => void;
  onClose: () => void;
  viewportHeight?: number; // Optional override for viewport height
}

export const AttachmentDialog: React.FC<AttachmentDialogProps> = ({
  attachments,
  onSelect,
  onClose,
  viewportHeight = DEFAULT_VIEWPORT_HEIGHT,
}) => {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scrollOffset, setScrollOffset] = useState(0);

  const VIEWPORT_HEIGHT = viewportHeight;

  // Handle keyboard input with CRITICAL priority
  // Modal dialogs must capture all input when visible
  // Uses useInputCompat for backward compatibility with tests
  useInputCompat({
    id: 'attachment-dialog',
    priority: InputPriority.CRITICAL,
    description: 'Attachment selection dialog',
    handler: (input, key) => {
      if (key.escape) {
        onClose();
        return true; // Consumed
      }

      if (key.return) {
        onSelect(attachments[selectedIndex]);
        onClose();
        return true; // Consumed
      }

      if (key.upArrow) {
        setSelectedIndex((prev) => {
          const newIndex = Math.max(0, prev - 1);
          // Auto-scroll up if needed
          if (newIndex < scrollOffset) {
            setScrollOffset(newIndex);
          }
          return newIndex;
        });
        return true; // Consumed
      }

      if (key.downArrow) {
        setSelectedIndex((prev) => {
          const newIndex = Math.min(attachments.length - 1, prev + 1);
          // Auto-scroll down if needed
          if (newIndex >= scrollOffset + VIEWPORT_HEIGHT) {
            setScrollOffset(newIndex - VIEWPORT_HEIGHT + 1);
          }
          return newIndex;
        });
        return true; // Consumed
      }

      // Consume all other input when dialog is open
      // This prevents background handlers from receiving input
      return true;
    },
  });

  // Calculate visible items
  const visibleAttachments = attachments.slice(scrollOffset, scrollOffset + VIEWPORT_HEIGHT);

  const showUpIndicator = scrollOffset > 0;
  const showDownIndicator = scrollOffset + VIEWPORT_HEIGHT < attachments.length;

  return (
    <Dialog onClose={onClose} borderColor="cyan" isActive={false}>
      <Box flexDirection="column" minWidth={50}>
        <Box marginBottom={1}>
          <Text bold color="cyan">
            Attachments ({attachments.length})
          </Text>
        </Box>

        {showUpIndicator && (
          <Box justifyContent="center">
            <Text dimColor>↑ More above</Text>
          </Box>
        )}

        <Box flexDirection="column">
          {visibleAttachments.map((attachment, index) => {
            const actualIndex = scrollOffset + index;
            const isSelected = actualIndex === selectedIndex;
            const filename = basename(attachment);

            return (
              <Box key={actualIndex}>
                <Text
                  backgroundColor={isSelected ? 'green' : undefined}
                  color={isSelected ? 'black' : 'white'}
                >
                  {isSelected ? '> ' : '  '}
                  {filename}
                </Text>
              </Box>
            );
          })}
        </Box>

        {showDownIndicator && (
          <Box justifyContent="center">
            <Text dimColor>↓ More below</Text>
          </Box>
        )}

        <Box marginTop={1} justifyContent="center">
          <Text dimColor>↑↓ Navigate • Enter Open • Esc Close</Text>
        </Box>
      </Box>
    </Dialog>
  );
};
