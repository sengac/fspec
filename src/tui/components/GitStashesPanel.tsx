/**
 * GitStashesPanel Component - displays git stashes in header
 *
 * Coverage:
 * - BOARD-019: Add fspec logo to TUI header
 */

import React from 'react';
import { Box, Text } from 'ink';

interface GitStashesPanelProps {
  stashes: any[];
}

export const GitStashesPanel: React.FC<GitStashesPanelProps> = ({ stashes }) => {
  const stashCount = stashes.length;
  const displayStashes = stashes.slice(0, 1); // Show only first stash to save space

  return (
    <Box flexDirection="column">
      <Text>
        Git Stashes ({stashCount})
        {displayStashes.length > 0 && `: ${displayStashes[0].message || 'No message'}`}
        {displayStashes.length === 0 && ': No stashes'}
      </Text>
    </Box>
  );
};
