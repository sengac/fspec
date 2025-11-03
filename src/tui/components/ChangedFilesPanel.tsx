/**
 * ChangedFilesPanel Component - displays keyboard shortcuts for file viewing
 *
 * Coverage:
 * - BOARD-019: Add fspec logo to TUI header
 * - TUI-014: Remove file watching from main board, show only keyboard shortcuts
 */

import React from 'react';
import { Box, Text } from 'ink';

export const ChangedFilesPanel: React.FC = () => {
  return (
    <Box flexDirection="column">
      <Text>C View Checkpoints ◆ F View Changed Files</Text>
    </Box>
  );
};
