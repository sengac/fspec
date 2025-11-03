import React from 'react';
import { Box, Text } from 'ink';

export interface CheckpointStatusProps {
  manualCount: number;
  autoCount: number;
}

export const CheckpointStatus: React.FC<CheckpointStatusProps> = ({
  manualCount,
  autoCount,
}) => {
  const text =
    manualCount === 0 && autoCount === 0
      ? 'Checkpoints: None'
      : `Checkpoints: ${manualCount} Manual, ${autoCount} Auto`;

  return (
    <Box flexGrow={1}>
      <Text>{text}</Text>
    </Box>
  );
};
