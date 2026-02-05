/**
 * AnchorPreviewPane - Renders the preview for a selected anchor
 *
 * Displays the actual turn content embedded in the anchor:
 * - User message from that turn
 * - Assistant response from that turn
 * - Tool calls made in that turn
 *
 * Uses VirtualList for scrollable content display.
 * NO METADATA, NO FALLBACKS - only actual turn content.
 */

import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import type { AnchorPoint } from '../types/anchor';

export interface AnchorPreviewPaneProps {
  /** Selected anchor to display content for */
  selectedAnchor: AnchorPoint | null;
  /** Width of the pane */
  width: number;
  /** Height available for content */
  contentHeight: number;
}

/**
 * Convert anchor's embedded turn content to display lines
 */
function anchorContentToLines(anchor: AnchorPoint, width: number): string[] {
  const lines: string[] = [];
  const contentWidth = Math.max(20, width - 4); // Account for padding

  // Helper to wrap text
  const wrapText = (text: string, indent: string = ''): void => {
    const words = text.split(/\s+/);
    let currentLine = indent;
    const maxLen = contentWidth - indent.length;

    for (const word of words) {
      if (currentLine.length + word.length + 1 > maxLen && currentLine.length > indent.length) {
        lines.push(currentLine);
        currentLine = indent + word;
      } else {
        currentLine += (currentLine.length > indent.length ? ' ' : '') + word;
      }
    }
    if (currentLine.length > indent.length) {
      lines.push(currentLine);
    }
  };

  // User message
  if (anchor.userMessage) {
    lines.push('USER:');
    lines.push('');
    wrapText(anchor.userMessage, '');
    lines.push('');
  }

  // Assistant response
  if (anchor.assistantResponse) {
    lines.push('ASSISTANT:');
    lines.push('');
    // Split by newlines first to preserve formatting
    const responseLines = anchor.assistantResponse.split('\n');
    for (const line of responseLines) {
      if (line.trim()) {
        wrapText(line, '');
      } else {
        lines.push('');
      }
    }
    lines.push('');
  }

  // Tool calls
  if (anchor.toolCalls && anchor.toolCalls.length > 0) {
    lines.push('TOOLS:');
    lines.push('');
    for (const tc of anchor.toolCalls) {
      const status = tc.success ? '+' : '-';
      lines.push(`  [${status}] ${tc.tool}`);
    }
    lines.push('');
  }

  // If no content at all, show a message
  if (lines.length === 0) {
    lines.push('No turn content available for this anchor.');
    lines.push('');
    lines.push('This anchor may have been created before');
    lines.push('turn content capture was implemented.');
  }

  return lines;
}

export function AnchorPreviewPane({
  selectedAnchor,
  width,
  contentHeight,
}: AnchorPreviewPaneProps): React.ReactElement {
  // Convert anchor content to lines for VirtualList
  const previewLines = useMemo(() => {
    if (!selectedAnchor) {
      return ['Select an anchor to view content'];
    }

    return anchorContentToLines(selectedAnchor, width);
  }, [selectedAnchor, width]);

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
          {selectedAnchor ? `TURN ${selectedAnchor.turnIndex}` : 'CONTENT'}
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
