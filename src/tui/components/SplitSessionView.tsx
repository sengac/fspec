/**
 * SplitSessionView - Split view for watcher sessions
 *
 * WATCH-010: Watcher Split View UI
 * WATCH-018: Extract Split View to Separate Component
 *
 * Features:
 * - Two vertical panes (parent left, watcher right)
 * - Left/Right arrows switch active pane
 * - Tab toggles turn-select mode in active pane
 * - Up/Down navigate turns when in select mode (via VirtualList)
 * - Enter on selected parent turn pre-fills input with "Discuss Selected" context
 * - Input area always sends to watcher session
 */

import React, { useCallback } from 'react';
import { Box, Text, useInput } from 'ink';
import { VirtualList } from './VirtualList';
import { ConversationInputArea } from './ConversationInputArea';
import { SelectionSeparatorBar } from './SelectionSeparatorBar';
import { useTerminalSize } from '../hooks/useTerminalSize';
import { useTurnSelection } from '../hooks/useTurnSelection';
import {
  getSelectionSeparatorType,
  getFirstContentOfTurn,
  generateDiscussSelectedPrefill,
  getContentLineCount,
} from '../utils/turnSelection';
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

  // Pane state
  const [activePane, setActivePane] = React.useState<'parent' | 'watcher'>('watcher');

  // Turn selection state (separate for each pane)
  const parentSelection = useTurnSelection();
  const watcherSelection = useTurnSelection();

  // Get current select mode for active pane
  const activeSelectMode = activePane === 'parent'
    ? parentSelection.isSelectMode
    : watcherSelection.isSelectMode;

  // Handle keyboard input
  useInput((input, key) => {
    if (isLoading) return;

    // Left/Right arrows switch panes (only when NOT in select mode)
    if (!activeSelectMode) {
      if (key.leftArrow) {
        setActivePane('parent');
        return;
      }
      if (key.rightArrow) {
        setActivePane('watcher');
        return;
      }
    }

    // Tab toggles turn-select mode in active pane
    if (key.tab) {
      if (activePane === 'parent') {
        parentSelection.toggleSelectMode();
      } else {
        watcherSelection.toggleSelectMode();
      }
      return;
    }

    // Escape exits select mode
    if (key.escape && activeSelectMode) {
      if (activePane === 'parent') {
        parentSelection.exitSelectMode();
      } else {
        watcherSelection.exitSelectMode();
      }
      return;
    }

    // Enter in parent pane select mode: "Discuss Selected" - pre-fill input
    if (key.return && activePane === 'parent' && parentSelection.isSelectMode) {
      const selectedIndex = parentSelection.selectionRef.current.selectedIndex;
      const selectedLine = parentConversation[selectedIndex];

      if (selectedLine) {
        const messageIndex = selectedLine.messageIndex;
        const turnNumber = messageIndex + 1; // 1-indexed for display
        const turnContent = getFirstContentOfTurn(parentConversation, messageIndex);

        if (turnContent) {
          const prefill = generateDiscussSelectedPrefill(turnNumber, turnContent);
          onInputChange(prefill);
          parentSelection.exitSelectMode();
        }
      }
      return;
    }
  });

  // Calculate layout dimensions
  const outerReservedLines = 5; // header(2) + input(2) + hints(1)
  const paneHeaderLines = 2;
  const splitPanesHeight = Math.max(1, terminalHeight - 1 - outerReservedLines);
  const virtualListHeight = Math.max(1, splitPanesHeight - paneHeaderLines);
  const paneContentWidth = Math.floor(terminalWidth / 2) - 6;

  // Render function for conversation lines with selection highlighting
  const renderConversationItem = useCallback((
    isParentPane: boolean,
    selectMode: boolean,
    paneLines: ConversationLine[]
  ) => (
    line: ConversationLine,
    index: number,
    _isSelected: boolean,
    selectedIndex: number
  ) => {
    const paneActive = isParentPane ? activePane === 'parent' : activePane === 'watcher';

    // Selection separator bars
    const separatorType = getSelectionSeparatorType(
      line, index, paneLines, selectedIndex, selectMode
    );
    if (separatorType) {
      return (
        <SelectionSeparatorBar
          direction={separatorType}
          width={paneContentWidth}
          reactKey={`line-${index}`}
        />
      );
    }

    // Regular separator
    if (line.isSeparator) {
      return (
        <Box key={`line-${index}`}>
          <Text> </Text>
        </Box>
      );
    }

    // Content line with role-based coloring
    const baseColor = line.role === 'user'
      ? 'green'
      : line.isThinking
        ? 'yellow'
        : line.isError
          ? 'red'
          : undefined;

    return (
      <Box key={`line-${index}`}>
        <Text
          dimColor={!paneActive}
          color={baseColor}
          wrap="truncate"
        >
          {line.content}
        </Text>
      </Box>
    );
  }, [activePane, paneContentWidth]);

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

      {/* Split panes container */}
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
              {activePane === 'parent' ? '→ ' : '  '}PARENT
            </Text>
            {parentSelection.isSelectMode && (
              <Text color="yellow" bold> [SELECT]</Text>
            )}
            <Text dimColor={activePane !== 'parent'}> ({getContentLineCount(parentConversation)})</Text>
          </Box>
          <Box flexGrow={1} flexBasis={0}>
            <VirtualList
              items={parentConversation}
              renderItem={renderConversationItem(true, parentSelection.isSelectMode, parentConversation)}
              keyExtractor={(_line, index) => `parent-${index}`}
              emptyMessage="No parent conversation"
              showScrollbar={true}
              isFocused={activePane === 'parent' && !isLoading}
              scrollToEnd={true}
              selectionMode={parentSelection.isSelectMode ? 'item' : 'scroll'}
              groupBy={parentSelection.isSelectMode ? (line) => line.messageIndex : undefined}
              groupPaddingBefore={parentSelection.isSelectMode ? 1 : 0}
              selectionRef={parentSelection.selectionRef}
              fixedHeight={virtualListHeight}
            />
          </Box>
        </Box>

        {/* Right pane: Watcher conversation */}
        <Box flexDirection="column" flexGrow={1} flexBasis={0}>
          <Box paddingX={1} borderStyle="single" borderBottom={true} borderTop={false} borderLeft={false} borderRight={false}>
            <Text bold dimColor={activePane !== 'watcher'}>
              {activePane === 'watcher' ? '→ ' : '  '}WATCHER
            </Text>
            {watcherSelection.isSelectMode && (
              <Text color="yellow" bold> [SELECT]</Text>
            )}
            <Text dimColor={activePane !== 'watcher'}> ({getContentLineCount(watcherConversation)})</Text>
          </Box>
          <Box flexGrow={1} flexBasis={0}>
            <VirtualList
              items={watcherConversation}
              renderItem={renderConversationItem(false, watcherSelection.isSelectMode, watcherConversation)}
              keyExtractor={(_line, index) => `watcher-${index}`}
              emptyMessage="Start chatting with your watcher..."
              showScrollbar={true}
              isFocused={activePane === 'watcher' && !isLoading}
              scrollToEnd={true}
              selectionMode={watcherSelection.isSelectMode ? 'item' : 'scroll'}
              groupBy={watcherSelection.isSelectMode ? (line) => line.messageIndex : undefined}
              groupPaddingBefore={watcherSelection.isSelectMode ? 1 : 0}
              selectionRef={watcherSelection.selectionRef}
              fixedHeight={virtualListHeight}
            />
          </Box>
        </Box>
      </Box>

      {/* Input area - always sends to watcher session */}
      <ConversationInputArea
        value={inputValue}
        onChange={onInputChange}
        onSubmit={onSubmit}
        isLoading={isLoading}
        placeholder="Type a message to your watcher..."
        isActive={!isLoading}
      />

      {/* Context-sensitive keyboard hints */}
      <Box paddingX={1}>
        <Text dimColor>
          {activeSelectMode
            ? '↑↓ Navigate | Enter Discuss | Tab/Esc Exit Select'
            : '←/→ Switch Pane | Tab Select Turn | ↑↓ Scroll | Esc Cancel'}
        </Text>
      </Box>
    </Box>
  );
};
