import React, { ReactNode } from 'react';
import { Box, useInput } from 'ink';

export interface DialogProps {
  children: ReactNode;
  onClose: () => void;
  borderColor?: string;
  isActive?: boolean;
}

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
 */
export const Dialog: React.FC<DialogProps> = ({
  children,
  onClose,
  borderColor,
  isActive = true,
}) => {
  // Handle ESC key to close dialog (only when active)
  useInput((input, key) => {
    if (key.escape) {
      onClose();
    }
  }, { isActive });

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
