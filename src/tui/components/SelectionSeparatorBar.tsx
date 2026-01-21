/**
 * SelectionSeparatorBar - Visual indicator for selected turn boundaries
 *
 * SOLID: Single responsibility - renders the arrow bar for turn selection
 * DRY: Shared between AgentView and SplitSessionView
 *
 * Renders a gray bar with arrows (▼ or ▲) to indicate the boundaries
 * of the currently selected turn in turn-select mode.
 */

import React from 'react';
import { Box, Text } from 'ink';
import { generateArrowBar } from '../utils/turnSelection';

export interface SelectionSeparatorBarProps {
  /** Direction of arrows: 'top' shows ▼, 'bottom' shows ▲ */
  direction: 'top' | 'bottom';
  /** Width of the bar in characters */
  width: number;
  /** Optional key for React list rendering */
  reactKey?: string;
}

/**
 * Visual separator bar with arrows indicating selected turn boundaries.
 *
 * - Top bar (▼▼▼): Appears above the selected turn
 * - Bottom bar (▲▲▲): Appears below the selected turn
 *
 * Both use gray background for visual contrast.
 */
export const SelectionSeparatorBar: React.FC<SelectionSeparatorBarProps> = ({
  direction,
  width,
  reactKey,
}) => {
  const arrowBar = generateArrowBar(width, direction);

  return (
    <Box key={reactKey} flexGrow={1}>
      <Text backgroundColor="gray" color="white">{arrowBar}</Text>
    </Box>
  );
};
