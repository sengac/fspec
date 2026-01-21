/**
 * TurnContentModal - Modal dialog for viewing full turn content
 *
 * TUI-045: Displays selected turn content in a full-screen scrollable modal.
 * Replaces the inline /expand command with a dedicated modal view.
 */

import React from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { wrapText } from '../utils/textWrap';

// Diff color constants (matching AgentView)
const DIFF_COLORS = {
  removed: '#8B0000', // Dark red
  added: '#006400', // Dark green
};

// Line type for VirtualList
interface ModalLine {
  role: 'user' | 'assistant' | 'tool' | 'watcher'; // WATCH-012: Added watcher role
  content: string;
  lineIndex: number;
}

export interface TurnContentModalProps {
  content: string;
  role: 'user' | 'assistant' | 'tool' | 'watcher'; // WATCH-012: Added watcher role
  terminalWidth: number;
  terminalHeight: number;
  isFocused: boolean;
}

/**
 * Get modal title based on message role
 */
const getRoleTitle = (role: 'user' | 'assistant' | 'tool' | 'watcher'): string => {
  switch (role) {
    case 'user':
      return 'User Message';
    case 'assistant':
      return 'Assistant Response';
    case 'tool':
      return 'Tool Output';
    case 'watcher':
      return 'Watcher Input'; // WATCH-012: Watcher message title
    default:
      return 'Content';
  }
};

/**
 * Wrap content into lines for VirtualList display
 * Uses shared wrapText utility for consistent line wrapping
 */
const wrapContentToLines = (
  content: string,
  role: 'user' | 'assistant' | 'tool' | 'watcher', // WATCH-012: Added watcher role
  maxWidth: number
): ModalLine[] => {
  const wrappedLines = wrapText(content, { maxWidth });
  
  return wrappedLines.map((lineContent, lineIndex) => ({
    role,
    content: lineContent,
    lineIndex,
  }));
};

/**
 * Render a single line with diff coloring support
 */
const renderModalLine = (line: ModalLine): React.ReactNode => {
  const content = line.content;

  // Check for diff markers
  const rIdx = content.indexOf('[R]');
  const aIdx = content.indexOf('[A]');

  if (line.role === 'tool' && (rIdx >= 0 || aIdx >= 0)) {
    const markerIdx = rIdx >= 0 ? rIdx : aIdx;
    const markerType = rIdx >= 0 ? 'R' : 'A';
    const lineWithoutMarker = content.slice(0, markerIdx) + content.slice(markerIdx + 3);
    return (
      <Box flexGrow={1}>
        <Text
          backgroundColor={markerType === 'R' ? DIFF_COLORS.removed : DIFF_COLORS.added}
          color="white"
        >
          {lineWithoutMarker}
        </Text>
      </Box>
    );
  }

  // Context lines (diff without marker) - line number gray, content white
  if (line.role === 'tool' && /^[L ]?\s*\d+\s{3}/.test(content)) {
    const match = content.match(/^([L ]?\s*\d+\s{3})(.*)$/);
    if (match) {
      const [, lineNumPart, contentPart] = match;
      return (
        <Box flexGrow={1}>
          <Text color="gray">{lineNumPart}</Text>
          <Text>{contentPart}</Text>
        </Box>
      );
    }
  }

  // Default rendering - user is green, watcher is magenta (WATCH-012), others white
  const baseColor = line.role === 'user' ? 'green' : line.role === 'watcher' ? 'magenta' : 'white';
  return (
    <Box flexGrow={1}>
      <Text color={baseColor}>{content}</Text>
    </Box>
  );
};

export const TurnContentModal: React.FC<TurnContentModalProps> = ({
  content,
  role,
  terminalWidth,
  terminalHeight,
  isFocused,
}) => {
  // Prepare lines for VirtualList
  const maxWidth = terminalWidth - 10; // Account for modal borders and padding
  const modalLines = wrapContentToLines(content, role, maxWidth);

  return (
    <Box
      position="absolute"
      width="100%"
      height="100%"
      justifyContent="center"
      alignItems="center"
      flexDirection="column"
    >
      <Box
        flexDirection="column"
        borderStyle="round"
        borderColor="cyan"
        padding={1}
        backgroundColor="black"
        width={terminalWidth - 4}
        height={terminalHeight - 6}
      >
        {/* Modal title based on message role */}
        <Box marginBottom={1}>
          <Text bold color="cyan">
            {getRoleTitle(role)}
          </Text>
        </Box>

        {/* Modal content - scrollable VirtualList */}
        <Box flexGrow={1} flexBasis={0}>
          <VirtualList
            items={modalLines}
            renderItem={renderModalLine}
            keyExtractor={(_line, index) => `modal-line-${index}`}
            emptyMessage="No content"
            showScrollbar={true}
            isFocused={isFocused}
            scrollToEnd={false}
            selectionMode="scroll"
          />
        </Box>

        {/* Modal footer with navigation hints */}
        <Box marginTop={1}>
          <Text dimColor>↑↓ Scroll | Esc Close</Text>
        </Box>
      </Box>
    </Box>
  );
};
