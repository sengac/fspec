import React from 'react';
import { Box, Text, useStdout } from 'ink';
import cliTruncate from 'cli-truncate';

export interface WorkUnitDescriptionProps {
  description: string;
  width?: number;
}

export const WorkUnitDescription: React.FC<WorkUnitDescriptionProps> = ({
  description,
  width: propWidth,
}) => {
  const { stdout } = useStdout();
  const terminalWidth = propWidth ?? (stdout?.columns || 80);

  // Normalize newlines to spaces for wrapping
  const normalized = description.replace(/\n/g, ' ').replace(/\s+/g, ' ').trim();

  // Calculate available width (terminal width - borders and padding)
  // Subtract: 2 for side borders, 2 for padding
  const availableWidth = Math.max(10, terminalWidth - 4);

  // Truncate to fit in 2 lines using cli-truncate (same as Ink internally)
  // TUI-012: Reduced from 3 lines to 2 lines to make space for attachments on line 3
  // Subtract 4 from maxChars to ensure ellipsis appears on line 2
  const maxChars = availableWidth * 2 - 4;
  const truncated = cliTruncate(normalized, maxChars, { position: 'end' });

  return (
    <Box height={2} flexShrink={0} width={availableWidth}>
      <Text color="cyan" wrap="wrap">
        {truncated}
      </Text>
    </Box>
  );
};
