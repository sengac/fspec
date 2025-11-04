import React from 'react';
import { Box, Text, useStdout } from 'ink';
import { basename } from 'path';
import cliTruncate from 'cli-truncate';

export interface WorkUnitAttachmentsProps {
  attachments?: string[];
  width?: number;
}

export const WorkUnitAttachments: React.FC<WorkUnitAttachmentsProps> = ({
  attachments,
  width: propWidth,
}) => {
  const { stdout } = useStdout();
  const terminalWidth = propWidth ?? (stdout?.columns || 80);

  // Calculate available width (terminal width - borders and padding)
  const availableWidth = Math.max(10, terminalWidth - 4);

  // Handle no attachments
  if (!attachments || attachments.length === 0) {
    return (
      <Box height={1} flexShrink={0}>
        <Text dimColor>No attachments</Text>
      </Box>
    );
  }

  // Extract filenames from paths
  const filenames = attachments.map(path => basename(path));

  // Build comma-separated list
  let displayText = filenames.join(', ');

  // Check if we need to truncate
  if (displayText.length > availableWidth) {
    // Try to fit as many filenames as possible
    let fittedText = '';
    let count = 0;

    for (let i = 0; i < filenames.length; i++) {
      const testText = count === 0
        ? filenames[i]
        : fittedText + ', ' + filenames[i];

      const remaining = filenames.length - i - 1;
      const moreText = remaining > 0 ? `, ...${remaining} more` : '';
      const fullText = testText + moreText;

      if (fullText.length <= availableWidth) {
        fittedText = testText;
        count = i + 1;
      } else {
        break;
      }
    }

    // Add the "...N more" suffix
    const remaining = filenames.length - count;
    if (remaining > 0) {
      displayText = fittedText + `, ...${remaining} more`;
    } else {
      displayText = fittedText;
    }
  }

  // Final truncation to ensure we don't overflow
  displayText = cliTruncate(displayText, availableWidth, { position: 'end' });

  return (
    <Box height={1} flexShrink={0}>
      <Text>{displayText}</Text>
    </Box>
  );
};
