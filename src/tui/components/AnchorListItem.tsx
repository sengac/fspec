/**
 * AnchorListItem - Renders a single anchor point in the anchor list
 *
 * Displays anchor metadata:
 * - Type label and anchor type code
 * - Turn number, weight score, and relative timestamp
 * - Description (wrapped to fit pane width)
 */

import React from 'react';
import { Box, Text } from 'ink';
import { ANCHOR_TYPE_LABELS, formatRelativeTime } from '../utils/anchorUtils';
import type { AnchorPoint } from '../types/anchor';

export interface AnchorListItemProps {
  /** The anchor point to render */
  anchor: AnchorPoint;
  /** Whether this item is currently selected */
  isSelected: boolean;
  /** Maximum width for description wrapping */
  maxDescriptionWidth: number;
}

/**
 * Wrap text to fit within a given width
 */
function wrapText(text: string, maxWidth: number): string[] {
  const words = text.split(/\s+/);
  const lines: string[] = [];
  let currentLine = '';

  for (const word of words) {
    if (currentLine.length + word.length + 1 > maxWidth && currentLine.length > 0) {
      lines.push(currentLine);
      currentLine = word;
    } else {
      currentLine += (currentLine.length > 0 ? ' ' : '') + word;
    }
  }
  if (currentLine.length > 0) {
    lines.push(currentLine);
  }

  return lines;
}

export function AnchorListItem({
  anchor,
  isSelected,
  maxDescriptionWidth,
}: AnchorListItemProps): React.ReactElement {
  const typeLabel = ANCHOR_TYPE_LABELS[anchor.anchorType] || `[${anchor.anchorType}]`;
  const relativeTime = formatRelativeTime(anchor.timestamp);

  // Wrap description to multiple lines
  const descriptionLines = anchor.description 
    ? wrapText(anchor.description, maxDescriptionWidth - 2) // -2 for margin
    : [];

  return (
    <Box flexDirection="column">
      <Box>
        <Text color={isSelected ? 'cyan' : undefined}>
          {isSelected ? '> ' : '  '}
        </Text>
        <Text bold={isSelected} color={isSelected ? 'white' : 'green'}>
          {typeLabel} {anchor.anchorType}
        </Text>
      </Box>
      <Box marginLeft={2}>
        <Text dimColor={!isSelected}>
          Turn {anchor.turnIndex} | {anchor.weight.toFixed(2)} | {relativeTime}
        </Text>
      </Box>
      {descriptionLines.map((line, idx) => (
        <Box key={idx} marginLeft={2}>
          <Text dimColor>{line}</Text>
        </Box>
      ))}
    </Box>
  );
}
