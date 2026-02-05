/**
 * AnchorListItem - Renders a single anchor point in the anchor list
 *
 * Displays anchor metadata:
 * - Type label and anchor type code
 * - Turn number, weight score, and relative timestamp
 * - Optional description (truncated to fit)
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
  /** Maximum width for description truncation */
  maxDescriptionWidth: number;
}

export function AnchorListItem({
  anchor,
  isSelected,
  maxDescriptionWidth,
}: AnchorListItemProps): React.ReactElement {
  const typeLabel = ANCHOR_TYPE_LABELS[anchor.anchorType] || `[${anchor.anchorType}]`;
  const relativeTime = formatRelativeTime(anchor.timestamp);

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
      {anchor.description && (
        <Box marginLeft={2}>
          <Text dimColor wrap="truncate">
            {anchor.description.slice(0, maxDescriptionWidth)}
          </Text>
        </Box>
      )}
    </Box>
  );
}
