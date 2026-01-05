/**
 * FullScreenWrapper - Full-screen layout wrapper for TUI components
 *
 * Coverage: BOARD-013 - Full-Screen TUI Layout
 *
 * Ensures the BoardView fills the entire terminal screen with no wasted space.
 * Uses useStdout hook to detect terminal dimensions and responds to resize events.
 *
 * Key optimization: Sets height to (rows - 1) to enable Ink's incremental rendering.
 * When output height >= terminal rows, Ink falls back to clearTerminal on every render.
 * By keeping height at rows-1, we stay below that threshold and Ink uses line-by-line
 * diffing instead, dramatically reducing flicker.
 */

import React, { type ReactNode } from 'react';
import { Box, useStdout } from 'ink';

interface FullScreenWrapperProps {
  children: ReactNode;
}

/**
 * A full-screen wrapper that ensures the content fills the entire terminal.
 * - Full terminal width
 * - Height set to rows-1 to enable Ink's incremental rendering
 * - Responsive to terminal resize events
 *
 * Note: We intentionally do NOT manage alternate screen buffer or cursor visibility
 * here because:
 * 1. Ink's log-update already manages cursor visibility
 * 2. useEffect runs AFTER Ink's first render, causing timing issues with alt screen
 * 3. Alt screen management is better done at the render() call site if needed
 */
export const FullScreenWrapper: React.FC<FullScreenWrapperProps> = ({
  children,
}) => {
  const { stdout } = useStdout();

  // Get terminal dimensions
  const width = stdout?.columns ?? 80;
  // Use height-1 to keep output below terminal rows threshold
  // This enables Ink's incremental rendering instead of full clearTerminal
  // See ink.tsx line 270: if (lastOutputHeight >= stdout.rows) { clearTerminal... }
  const height = Math.max(1, (stdout?.rows ?? 24) - 1);

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
