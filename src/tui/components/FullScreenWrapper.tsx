/**
 * FullScreenWrapper - Full-screen layout wrapper for TUI components
 *
 * Coverage: BOARD-013 - Full-Screen TUI Layout
 *
 * Ensures the BoardView fills the entire terminal screen with no wasted space.
 * Uses useTerminalSize hook to reactively track terminal dimensions on resize.
 *
 * Key optimization: Sets height to (rows - 1) to enable Ink's incremental rendering.
 * When output height >= terminal rows, Ink falls back to clearTerminal on every render.
 * By keeping height at rows-1, we stay below that threshold and Ink uses line-by-line
 * diffing instead, dramatically reducing flicker.
 *
 * Note: Ink's core resize handler (Ink.resized) recalculates Yoga layout and repaints,
 * but does NOT trigger React re-renders. useTerminalSize provides the explicit resize
 * subscription needed to re-render this component with updated dimensions.
 */

import React, { type ReactNode } from 'react';
import { Box } from 'ink';
import { useTerminalSize } from '../hooks/useTerminalSize';

interface FullScreenWrapperProps {
  children: ReactNode;
}

/**
 * A full-screen wrapper that ensures the content fills the entire terminal.
 * - Full terminal width
 * - Height set to rows-1 to enable Ink's incremental rendering
 * - Responsive to terminal resize events via useTerminalSize
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
  const { width, height: rawHeight } = useTerminalSize();

  // Use height-1 to keep output below terminal rows threshold
  // This enables Ink's incremental rendering instead of full clearTerminal
  // See ink.tsx line 270: if (lastOutputHeight >= stdout.rows) { clearTerminal... }
  const height = Math.max(1, rawHeight - 1);

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
