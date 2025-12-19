/**
 * FullScreenWrapper - Full-screen layout wrapper for TUI components
 *
 * Coverage: BOARD-013 - Full-Screen TUI Layout
 *
 * Ensures the BoardView fills the entire terminal screen with no wasted space.
 * Uses useStdout hook to detect terminal dimensions and responds to resize events.
 * Similar to CAGE's FullScreenWrapper implementation.
 */

import React, { type ReactNode, useEffect } from 'react';
import { Box, useStdout } from 'ink';

interface FullScreenWrapperProps {
  children: ReactNode;
}

/**
 * A full-screen wrapper that ensures the content fills the entire terminal.
 * - Full terminal width and height
 * - Clears screen on mount to start from position (0,0)
 * - Responsive to terminal resize events
 */
export const FullScreenWrapper: React.FC<FullScreenWrapperProps> = ({
  children,
}) => {
  const { stdout } = useStdout();

  // Clear screen before rendering to eliminate artifacts
  useEffect(() => {
    // Clear screen: ESC c (reset terminal)
    // This moves cursor to (0,0) and clears previous output
    stdout?.write('\x1Bc');
  }, [stdout]);

  // Get terminal dimensions
  const width = stdout?.columns ?? 80;
  const height = stdout?.rows ?? 24;

  return (
    <Box
      width={width}
      height={height}
      flexDirection="column"
    >
      {children}
    </Box>
  );
};
