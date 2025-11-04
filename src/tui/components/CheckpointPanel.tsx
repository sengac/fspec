/**
 * CheckpointPanel Component - displays checkpoint counts in header
 *
 * Coverage:
 * - ITF-006: Replace Git Stashes with Checkpoint Component
 * - BUG-065: Complete TUI-016 IPC integration
 */

import React, { useEffect } from 'react';
import { Box, Text } from 'ink';
import { useFspecStore } from '../store/fspecStore';

interface CheckpointPanelProps {
  cwd?: string;
}

export const CheckpointPanel: React.FC<CheckpointPanelProps> = ({ cwd }) => {
  // Read checkpoint counts from Zustand store
  const checkpointCounts = useFspecStore(state => state.checkpointCounts);
  const loadCheckpointCounts = useFspecStore(state => state.loadCheckpointCounts);

  // Load checkpoint counts on mount
  useEffect(() => {
    void loadCheckpointCounts();
  }, [loadCheckpointCounts]);

  // Format display: "Checkpoints: X Manual, Y Auto" or "Checkpoints: None"
  const displayText = checkpointCounts.manual === 0 && checkpointCounts.auto === 0
    ? 'Checkpoints: None'
    : `Checkpoints: ${checkpointCounts.manual} Manual, ${checkpointCounts.auto} Auto`;

  return (
    <Box flexDirection="column">
      <Text>{displayText}</Text>
    </Box>
  );
};
