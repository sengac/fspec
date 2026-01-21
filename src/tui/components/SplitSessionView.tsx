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
import { VirtualList } from './VirtualList';
import { ConversationInputArea } from './ConversationInputArea';
import { useTerminalSize } from '../hooks/useTerminalSize';
import type { ConversationLine } from '../types/conversation';

interface SplitSessionViewProps {
  parentSessionName: string;
  watcherRoleName: string;
  terminalWidth: number;
  parentConversation: ConversationLine[];
  watcherConversation: ConversationLine[];
  /** Current input value */
  inputValue: string;
  /** Callback when input value changes */
  onInputChange: (value: string) => void;
  /** Callback when user submits message */
  onSubmit: (value: string) => void;
  /** Whether the AI is currently processing */
  isLoading: boolean;
}

/**
 * SplitSessionView component - renders the watcher split view UI
 *
 * Features:
 * - Two vertical panes (parent left, watcher right)
 * - Left/Right arrows switch active pane
 * - VirtualList for scrollable conversations
 * - Input area sends to watcher session
 */
export const SplitSessionView: React.FC<SplitSessionViewProps> = ({
  parentSessionName,
  watcherRoleName,
  terminalWidth,
  parentConversation,
  watcherConversation,
  inputValue,
  onInputChange,
  onSubmit,
  isLoading,
}) => {
  const { height: terminalHeight } = useTerminalSize();

  const [activePane, setActivePane] = useState<'parent' | 'watcher'>('watcher');

  // Handle Left/Right arrow keys to switch panes (only when not loading)
  useInput((input, key) => {
    if (isLoading) {
      return; // Don't switch panes while loading
    }
    if (key.leftArrow) {
      setActivePane('parent');
    } else if (key.rightArrow) {
      setActivePane('watcher');
    }
  });

  // Calculate explicit heights for the panes
  // Layout: Header(2) + Split panes + Input(2) + Hints(1) = 5 lines reserved from outer container
  // Within split panes: Pane header(2) + VirtualList content
  // Note: FullScreenWrapper gives us terminalHeight - 1
  const outerReservedLines = 5; // header(2) + input(2) + hints(1)
  const paneHeaderLines = 2;    // pane header(1 text + 1 border)
  const splitPanesHeight = Math.max(1, terminalHeight - 1 - outerReservedLines);
  const virtualListHeight = Math.max(1, splitPanesHeight - paneHeaderLines);

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
          wrap="truncate"
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
        {isLoading && <Text color="yellow"> ⏳</Text>}
      </Box>

      {/* Split panes container - use explicit height instead of relying on flexbox measurement */}
      <Box flexDirection="row" height={splitPanesHeight}>
        {/* Left pane: Parent conversation */}
        <Box
          flexDirection="column"
          flexGrow={1}
          flexBasis={0}
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
              isFocused={activePane === 'parent' && !isLoading}
              scrollToEnd={true}
              selectionMode="scroll"
              fixedHeight={virtualListHeight}
            />
          </Box>
        </Box>

        {/* Right pane: Watcher conversation */}
        <Box flexDirection="column" flexGrow={1} flexBasis={0}>
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
              isFocused={activePane === 'watcher' && !isLoading}
              scrollToEnd={true}
              selectionMode="scroll"
              fixedHeight={virtualListHeight}
            />
          </Box>
        </Box>
      </Box>

      {/* Input area - sends to watcher session */}
      <ConversationInputArea
        value={inputValue}
        onChange={onInputChange}
        onSubmit={onSubmit}
        isLoading={isLoading}
        placeholder="Type a message to your watcher..."
        isActive={!isLoading}
      />

      {/* Keyboard hints */}
      <Box paddingX={1}>
        <Text dimColor>
          ←/→ Switch Pane | ↑↓ Scroll | Esc Cancel
        </Text>
      </Box>
    </Box>
  );
};
