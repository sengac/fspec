import React from 'react';
import { Box, Text } from 'ink';

export interface WorkUnitTitleProps {
  id: string;
  title: string;
}

export const WorkUnitTitle: React.FC<WorkUnitTitleProps> = ({
  id,
  title,
}) => {
  const fullTitle = `${id}: ${title}`;

  return (
    <Box height={1} flexShrink={1}>
      <Text wrap="truncate">{fullTitle}</Text>
    </Box>
  );
};
