/**
 * AnchorPreviewPane - Renders the preview for a selected anchor
 *
 * Displays:
 * - Header with turn number
 * - Scrollable content using VirtualList
 * - Turn details (user message, assistant response, tool calls)
 * - Fallback to anchor metadata when turn details unavailable
 */

import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { turnDetailsToLines, ANCHOR_TYPE_LABELS, formatRelativeTime } from '../utils/anchorUtils';
import type { AnchorPoint, AnchorTurnDetails } from '../types/anchor';

export interface AnchorPreviewPaneProps {
  /** Selected anchor to display info for */
  selectedAnchor: AnchorPoint | null;
  /** Turn details to display (null if not loaded) */
  turnDetails: AnchorTurnDetails | null;
  /** Whether details are currently loading */
  isLoading: boolean;
  /** Width of the pane */
  width: number;
  /** Height available for content */
  contentHeight: number;
}

/**
 * Convert anchor info to display lines
 */
function anchorInfoToLines(anchor: AnchorPoint): string[] {
  const lines: string[] = [];
  
  lines.push(`TURN ${anchor.turnIndex}`);
  lines.push('─'.repeat(40));
  lines.push('');
  
  // Type
  const typeLabel = ANCHOR_TYPE_LABELS[anchor.anchorType] || anchor.anchorType;
  lines.push(`Type: ${typeLabel} ${anchor.anchorType}`);
  lines.push('');
  
  // Confidence and Weight
  lines.push(`Confidence: ${(anchor.confidence * 100).toFixed(0)}%`);
  lines.push(`Weight: ${anchor.weight.toFixed(2)}`);
  lines.push('');
  
  // Timestamp
  lines.push(`Created: ${formatRelativeTime(anchor.timestamp)}`);
  lines.push('');
  
  // Description (wrap long lines)
  lines.push('Description:');
  const descWords = anchor.description.split(' ');
  let currentLine = '  ';
  for (const word of descWords) {
    if (currentLine.length + word.length + 1 > 60) {
      lines.push(currentLine);
      currentLine = '  ' + word;
    } else {
      currentLine += (currentLine.length > 2 ? ' ' : '') + word;
    }
  }
  if (currentLine.length > 2) {
    lines.push(currentLine);
  }
  
  return lines;
}

export function AnchorPreviewPane({
  selectedAnchor,
  turnDetails,
  isLoading,
  width,
  contentHeight,
}: AnchorPreviewPaneProps): React.ReactElement {
  // Convert anchor/turn details to lines for VirtualList
  const previewLines = useMemo(() => {
    if (!selectedAnchor) {
      return ['Select an anchor to view details'];
    }
    
    if (isLoading) {
      return ['Loading...'];
    }
    
    // If we have turn details, show them
    if (turnDetails) {
      return turnDetailsToLines(turnDetails);
    }
    
    // Otherwise show anchor info
    return anchorInfoToLines(selectedAnchor);
  }, [selectedAnchor, turnDetails, isLoading]);

  const renderPreviewLine = (
    line: string,
    index: number,
    _isSelected: boolean,
    _selectedIndex: number
  ): React.ReactNode => (
    <Box key={index}>
      <Text wrap="truncate-end">{line}</Text>
    </Box>
  );

  return (
    <Box
      width={width}
      flexDirection="column"
      paddingX={1}
    >
      <Box marginBottom={1}>
        <Text bold color="cyan">
          {selectedAnchor ? `TURN ${selectedAnchor.turnIndex}` : 'DETAILS'}
        </Text>
      </Box>
      <Box flexGrow={1}>
        <VirtualList
          items={previewLines}
          renderItem={renderPreviewLine}
          keyExtractor={(_item, idx) => `line-${idx}`}
          showScrollbar={true}
          isFocused={false}
          selectionMode="scroll"
          fixedHeight={contentHeight}
        />
      </Box>
    </Box>
  );
}
