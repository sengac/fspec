/**
 * Logo Component - fspec ASCII logo
 *
 * Coverage:
 * - BOARD-019: Add fspec logo to TUI header
 */

import React from 'react';
import { Box, Text } from 'ink';

export const Logo: React.FC = () => {
  return (
    <Box flexDirection="column" width={12} height={4} flexShrink={0}>
      <Text>┏┓┏┓┏┓┏┓┏┓ </Text>
      <Text>┣ ┗┓┃┃┣ ┃ </Text>
      <Text>┻ ┗┛┣┛┗┛┗┛ </Text>
      <Text> </Text>
    </Box>
  );
};
