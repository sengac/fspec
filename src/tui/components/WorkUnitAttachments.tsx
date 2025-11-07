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

  // Reserve space for prefix (different based on whether attachments exist)
  const hasAttachments = attachments && attachments.length > 0;
  const prefix = hasAttachments
    ? 'Attachments (use the "A" key to view): '
    : 'Attachments: ';
  const prefixLength = prefix.length;
  const contentWidth = Math.max(4, availableWidth - prefixLength);

  // Handle no attachments
  if (!hasAttachments) {
    return (
      <Box height={1} flexShrink={0}>
        <Text>
          <Text dimColor>{prefix}</Text>
          <Text dimColor>none</Text>
        </Text>
      </Box>
    );
  }

  // Extract filenames from paths
  const filenames = attachments.map(path => basename(path));

  // Build comma-separated list
  let displayText = filenames.join(', ');

  // Check if we need to truncate (use contentWidth instead of availableWidth)
  if (displayText.length > contentWidth) {
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

      if (fullText.length <= contentWidth) {
        fittedText = testText;
        count = i + 1;
      } else {
        break;
      }
    }

    // Add the "...N more" suffix
    const remaining = filenames.length - count;
    if (remaining > 0) {
      if (count === 0) {
        // No filenames fit - show first filename truncated with "...N more" if possible
        const moreText = `, ...${remaining} more`;
        const maxFirstFilenameWidth = contentWidth - moreText.length;

        if (maxFirstFilenameWidth > 3) {
          // Enough room to show truncated filename + suffix
          const truncatedFirst = cliTruncate(filenames[0], maxFirstFilenameWidth, { position: 'end' });
          displayText = truncatedFirst + moreText;
        } else {
          // Not enough room for suffix, just show truncated filename
          displayText = cliTruncate(filenames[0], contentWidth, { position: 'end' });
        }
      } else {
        // Some filenames fit - use fitted text + suffix
        displayText = fittedText + `, ...${remaining} more`;
      }
    } else {
      displayText = fittedText;
    }
  }

  // Final truncation to ensure we don't overflow (use contentWidth and truncate-end)
  displayText = cliTruncate(displayText, contentWidth, { position: 'end' });

  return (
    <Box height={1} flexShrink={0}>
      <Text>
        <Text dimColor>{prefix}</Text>
        <Text>{displayText}</Text>
      </Text>
    </Box>
  );
};
