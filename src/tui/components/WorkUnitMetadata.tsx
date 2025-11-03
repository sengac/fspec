import React from 'react';
import { Box, Text } from 'ink';

export interface WorkUnitMetadataProps {
  epic?: string;
  estimate?: number;
  status?: string;
}

export const WorkUnitMetadata: React.FC<WorkUnitMetadataProps> = ({
  epic,
  estimate,
  status,
}) => {
  const fields: string[] = [];

  if (epic) {
    fields.push(`Epic: ${epic}`);
  }

  if (estimate !== undefined) {
    fields.push(`Estimate: ${estimate}pts`);
  }

  if (status) {
    fields.push(`Status: ${status}`);
  }

  const text = fields.join(' | ');

  return (
    <Box height={1} flexShrink={1}>
      <Text wrap="truncate">{text}</Text>
    </Box>
  );
};
