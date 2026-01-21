/**
 * SplitSessionView - Split view for watcher sessions
 *
 * Extracted from AgentView.tsx to isolate and debug the watcher split view functionality.
 * Built compositionally - each piece added incrementally to isolate issues.
 *
 * WATCH-018: Extract Split View to Separate Component
 */

import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import { logger } from '../../utils/logger';
import { VirtualList } from './VirtualList';
import { useTerminalSize } from '../hooks/useTerminalSize';
import type { ConversationLine } from '../types/conversation';

interface SplitSessionViewProps {
  parentSessionName: string;
  watcherRoleName: string;
  terminalWidth: number;
  parentConversation: ConversationLine[];
  watcherConversation: ConversationLine[];
}

/**
 * SplitSessionView component - renders the watcher split view UI
 *
 * Both panes now use real conversation data with proper line wrapping.
 */
export const SplitSessionView: React.FC<SplitSessionViewProps> = ({
  parentSessionName,
  watcherRoleName,
  terminalWidth,
  parentConversation,
  watcherConversation,
}) => {
  const { height: terminalHeight } = useTerminalSize();

  logger.warn('[SplitSessionView] Rendering with real data');
  logger.warn(`[SplitSessionView] terminalHeight=${terminalHeight}, terminalWidth=${terminalWidth}`);
  logger.warn(`[SplitSessionView] parentConversation.length=${parentConversation.length}, watcherConversation.length=${watcherConversation.length}`);

  const [activePane, setActivePane] = useState<'parent' | 'watcher'>('watcher');

  // Handle Left/Right arrow keys to switch panes
  useInput((input, key) => {
    if (key.leftArrow) {
      logger.warn('[SplitSessionView] Left arrow - switching to parent pane');
      setActivePane('parent');
    } else if (key.rightArrow) {
      logger.warn('[SplitSessionView] Right arrow - switching to watcher pane');
      setActivePane('watcher');
    }
  });

  const paneWidth = Math.floor((terminalWidth - 3) / 2);

  // Reserved: Header(2) + Pane headers(2) + Input(2) + Hints(1) = 7
  const reservedLines = 7;
  const paneHeight = Math.max(1, terminalHeight - reservedLines);
  logger.warn(`[SplitSessionView] paneHeight=${paneHeight}, activePane=${activePane}`);

  // Render function for conversation lines
  const renderConversationItem = (isParentPane: boolean) => (
    line: ConversationLine,
    index: number,
    _isSelected: boolean
  ) => {
    const paneActive = isParentPane ? activePane === 'parent' : activePane === 'watcher';
    
    if (line.isSeparator) {
      return (
        <Box key={`line-${index}`}>
          <Text> </Text>
        </Box>
      );
    }
    return (
      <Box key={`line-${index}`}>
        <Text
          dimColor={!paneActive}
          color={line.role === 'user' ? 'green' : line.isThinking ? 'yellow' : line.isError ? 'red' : undefined}
        >
          {line.content}
        </Text>
      </Box>
    );
  };

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* Header */}
      <Box
        borderStyle="single"
        borderBottom={true}
        borderTop={false}
        borderLeft={false}
        borderRight={false}
        paddingX={1}
      >
        <Text bold color="magenta">
          👁️ {watcherRoleName} (watching: {parentSessionName})
        </Text>
      </Box>

      {/* Split panes container */}
      <Box flexDirection="row" flexGrow={1} flexBasis={0}>
        {/* Left pane: Parent conversation */}
        <Box
          flexDirection="column"
          width={paneWidth}
          borderStyle="single"
          borderRight={true}
          borderLeft={false}
          borderTop={false}
          borderBottom={false}
        >
          <Box paddingX={1} borderStyle="single" borderBottom={true} borderTop={false} borderLeft={false} borderRight={false}>
            <Text bold dimColor={activePane !== 'parent'}>
              {activePane === 'parent' ? '→ ' : '  '}PARENT ({parentConversation.length} lines)
            </Text>
          </Box>
          <Box flexGrow={1} flexBasis={0}>
            <VirtualList
              items={parentConversation}
              renderItem={renderConversationItem(true)}
              keyExtractor={(_line, index) => `parent-${index}`}
              emptyMessage="No parent conversation"
              showScrollbar={true}
              isFocused={activePane === 'parent'}
              scrollToEnd={true}
              selectionMode="scroll"
              fixedHeight={paneHeight}
            />
          </Box>
        </Box>

        {/* Right pane: Watcher conversation */}
        <Box flexDirection="column" width={paneWidth}>
          <Box paddingX={1} borderStyle="single" borderBottom={true} borderTop={false} borderLeft={false} borderRight={false}>
            <Text bold dimColor={activePane !== 'watcher'}>
              {activePane === 'watcher' ? '→ ' : '  '}WATCHER ({watcherConversation.length} lines)
            </Text>
          </Box>
          <Box flexGrow={1} flexBasis={0}>
            <VirtualList
              items={watcherConversation}
              renderItem={renderConversationItem(false)}
              keyExtractor={(_line, index) => `watcher-${index}`}
              emptyMessage="Start chatting with your watcher..."
              showScrollbar={true}
              isFocused={activePane === 'watcher'}
              scrollToEnd={true}
              selectionMode="scroll"
              fixedHeight={paneHeight}
            />
          </Box>
        </Box>
      </Box>

      {/* Input area placeholder */}
      <Box
        borderStyle="single"
        borderTop={true}
        borderBottom={false}
        borderLeft={false}
        borderRight={false}
        paddingX={1}
      >
        <Text color="green">&gt; </Text>
        <Text dimColor>[Input coming in Step 4]</Text>
      </Box>

      {/* Keyboard hints */}
      <Box paddingX={1}>
        <Text dimColor>
          ←/→ Switch Pane | ↑↓ Scroll | PgUp/PgDn Page | Home/End Jump
        </Text>
      </Box>
    </Box>
  );
};
