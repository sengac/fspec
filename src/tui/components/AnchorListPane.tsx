/**
 * AnchorListPane - Renders the anchor list pane with navigation
 *
 * Displays:
 * - Header with "ANCHORS" title
 * - Scrollable list of anchor items using VirtualList
 * - Handles selection via VirtualList's onFocus callback
 */

import React, { useCallback } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { AnchorListItem } from './AnchorListItem';
import type { AnchorPoint } from '../types/anchor';

export interface AnchorListPaneProps {
  /** Array of anchor points to display */
  anchorPoints: AnchorPoint[];
  /** Width of the pane */
  width: number;
  /** Height available for content */
  contentHeight: number;
  /** Callback when an anchor is focused/selected */
  onAnchorSelect: (anchor: AnchorPoint) => void;
}

export function AnchorListPane({
  anchorPoints,
  width,
  contentHeight,
  onAnchorSelect,
}: AnchorListPaneProps): React.ReactElement {
  // Calculate max description width (accounting for padding and margins)
  const maxDescriptionWidth = width - 6;

  const handleFocus = useCallback(
    (anchor: AnchorPoint, _index: number) => {
      onAnchorSelect(anchor);
    },
    [onAnchorSelect]
  );

  const renderAnchorItem = (
    anchor: AnchorPoint,
    index: number,
    isSelected: boolean,
    _selectedIndex: number
  ): React.ReactNode => (
    <AnchorListItem
      key={index}
      anchor={anchor}
      isSelected={isSelected}
      maxDescriptionWidth={maxDescriptionWidth}
    />
  );

  return (
    <Box
      width={width}
      flexDirection="column"
      paddingX={1}
    >
      <Box marginBottom={1}>
        <Text bold color="cyan">ANCHORS</Text>
      </Box>
      <Box flexGrow={1}>
        <VirtualList
          items={anchorPoints}
          renderItem={renderAnchorItem}
          keyExtractor={(item, idx) => `${item.turnIndex}-${idx}`}
          showScrollbar={true}
          isFocused={true}
          selectionMode="item"
          fixedHeight={contentHeight}
          onFocus={handleFocus}
        />
      </Box>
    </Box>
  );
}
