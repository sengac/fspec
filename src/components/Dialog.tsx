/**
 * Base Dialog component - provides ONLY modal overlay infrastructure.
 *
 * Responsibilities:
 * - Centered modal overlay rendering
 * - Border styling with optional color
 * - ESC key handling to call onClose
 * - Input capture control via isActive prop
 *
 * Does NOT handle:
 * - Business logic (confirmation, forms, etc.)
 * - Content-specific keyboard interactions
 * - Callbacks other than onClose
 *
 * Implements composition pattern - accepts children for content.
 *
 * INPUT-001: Uses centralized input handling with CRITICAL priority
 * (Modal dialogs should capture input before any other handlers)
 */

import React, { ReactNode } from 'react';
import { Box } from 'ink';
import { useInputCompat, InputPriority } from '../tui/input/index.js';

export interface DialogProps {
  children: ReactNode;
  onClose: () => void;
  borderColor?: string;
  isActive?: boolean;
}

/**
 * Base Dialog component - provides ONLY modal overlay infrastructure.
 */
export const Dialog: React.FC<DialogProps> = ({
  children,
  onClose,
  borderColor,
  isActive = true,
}) => {
  // Handle ESC key to close dialog (only when active)
  // Uses CRITICAL priority to ensure modal captures input before other handlers
  useInputCompat({
    id: 'dialog-esc',
    priority: InputPriority.CRITICAL,
    isActive,
    handler: (_input, key) => {
      if (key.escape) {
        onClose();
        return true;
      }
      return false;
    },
  });

  return (
    <Box
      position="absolute"
      width="100%"
      height="100%"
      justifyContent="center"
      alignItems="center"
      flexDirection="column"
    >
      <Box
        flexDirection="column"
        borderStyle="round"
        borderColor={borderColor}
        padding={1}
        backgroundColor="black"
      >
        {children}
      </Box>
    </Box>
  );
};
