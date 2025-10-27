/**
 * useTerminalSize - React hook for responsive terminal dimensions
 *
 * Coverage:
 * - ITF-004: Fix TUI Kanban column layout to match table style
 *
 * Provides reactive terminal dimensions that automatically update when terminal resizes.
 * Based on cage project patterns for responsive TUI layouts.
 *
 * Key features:
 * - Registers resize listener on stdout 'resize' event
 * - Stores dimensions in React state to trigger re-renders
 * - Cleans up listener on unmount to prevent memory leaks
 * - Provides fallback dimensions (80x24) when stdout unavailable
 *
 * @see ~/projects/cage/packages/cli/src/shared/hooks/useResponsiveLayout.tsx
 */

import { useState, useEffect } from 'react';
import { useStdout } from 'ink';

export interface TerminalDimensions {
  width: number;
  height: number;
}

/**
 * Hook that provides reactive terminal dimensions
 *
 * @returns TerminalDimensions - Current terminal width and height
 *
 * @example
 * ```typescript
 * const MyComponent = () => {
 *   const { width, height } = useTerminalSize();
 *   const columnWidth = Math.floor(width / 7); // 7 columns
 *   return <Box width={width} height={height}>...</Box>;
 * };
 * ```
 */
export function useTerminalSize(): TerminalDimensions {
  const { stdout } = useStdout();

  // Initialize state with current dimensions (fallback to 80x24)
  const [dimensions, setDimensions] = useState<TerminalDimensions>(() => {
    const width = stdout?.columns || 80;
    const height = stdout?.rows || 24;
    return { width, height };
  });

  useEffect(() => {
    // Skip if stdout is not available
    if (!stdout) {
      return;
    }

    // Handler that updates dimensions on resize
    const handleResize = () => {
      const width = stdout.columns || 80;
      const height = stdout.rows || 24;
      setDimensions({ width, height });
    };

    // Register resize listener
    stdout.on('resize', handleResize);

    // CRITICAL: Cleanup listener on unmount to prevent memory leaks
    return () => {
      stdout.off('resize', handleResize);
    };
  }, [stdout]);

  return dimensions;
}
