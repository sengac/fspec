import React from 'react';
import { Box, Text } from 'ink';

export const KeybindingShortcuts: React.FC = () => {
  return (
    <Box
      flexGrow={1}
      borderTop={true}
      borderBottom={false}
      borderLeft={false}
      borderRight={false}
      borderStyle="single"
    >
      <Text>C View Checkpoints ◆ F View Changed Files</Text>
    </Box>
  );
};
