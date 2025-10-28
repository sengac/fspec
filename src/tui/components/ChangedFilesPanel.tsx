/**
 * ChangedFilesPanel Component - displays changed files in header
 *
 * Coverage:
 * - BOARD-019: Add fspec logo to TUI header
 */

import React from 'react';
import { Box, Text } from 'ink';

interface ChangedFilesPanelProps {
  stagedFiles: string[];
  unstagedFiles: string[];
}

export const ChangedFilesPanel: React.FC<ChangedFilesPanelProps> = ({
  stagedFiles,
  unstagedFiles,
}) => {
  const stagedCount = stagedFiles.length;
  const unstagedCount = unstagedFiles.length;

  return (
    <Box flexDirection="column">
      <Text>
        Changed Files: {stagedCount} staged, {unstagedCount} unstaged
      </Text>
      <Text>{'─'.repeat(80)}</Text>
      <Text>S View Stashes ◆ C View Changed Files</Text>
    </Box>
  );
};
