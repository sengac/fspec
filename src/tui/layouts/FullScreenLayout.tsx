/**
 * FullScreenLayout Component
 *
 * Provides a consistent full-screen layout with optional title bar and footer.
 * Adapted from cage's layout patterns for fspec's TUI infrastructure.
 *
 * Coverage: ITF-001 - Scenario: FullScreenLayout renders with title bar and footer
 */

import React from 'react';
import { Box, Text } from 'ink';

interface FullScreenLayoutProps {
  title?: string;
  footer?: string;
  children: React.ReactNode;
}

export const FullScreenLayout: React.FC<FullScreenLayoutProps> = ({
  title,
  footer,
  children,
}) => {
  return (
    <Box flexDirection="column" height="100%">
      {/* Title Bar */}
      {title && (
        <Box borderStyle="single" borderBottom={true} paddingX={1}>
          <Text bold>{title}</Text>
        </Box>
      )}

      {/* Main Content */}
      <Box flexGrow={1} flexDirection="column">
        {children}
      </Box>

      {/* Footer */}
      {footer && (
        <Box borderStyle="single" borderTop={true} paddingX={1}>
          <Text dimColor>{footer}</Text>
        </Box>
      )}
    </Box>
  );
};
