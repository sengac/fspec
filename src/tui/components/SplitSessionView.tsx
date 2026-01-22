/**
 * SplitSessionView - Split view for watcher sessions
 *
 * WATCH-010: Watcher Split View UI
 * WATCH-011: Cross-Pane Selection with Correlation IDs
 * WATCH-015: Watcher Session Header Indicator
 * WATCH-018: Extract Split View to Separate Component
 *
 * Features:
 * - Two vertical panes (parent left, watcher right)
 * - Left/Right arrows switch active pane
 * - Tab toggles turn-select mode in active pane
 * - Up/Down navigate turns when in select mode (via VirtualList)
 * - Enter on selected parent turn pre-fills input with "Discuss Selected" context
 * - Input area always sends to watcher session
 * - WATCH-011: Cross-pane highlighting shows correlation between parent/watcher turns
 * - WATCH-015: Header shows model capabilities, token usage, and context fill
 */

import React, { useCallback, useMemo } from 'react';
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
import { buildCorrelationMaps } from '../utils/correlationMapping';
import type { ConversationLine } from '../types/conversation';

// WATCH-015: Token usage tracker interface
interface TokenTracker {
  inputTokens: number;
  outputTokens: number;
  cacheReadInputTokens?: number;
  cacheCreationInputTokens?: number;
}

// WATCH-015: Format context window size for display (200000 → "200k")
const formatContextWindow = (contextWindow: number): string => {
  if (contextWindow >= 1000000) {
    return `${(contextWindow / 1000000).toFixed(0)}M`;
  }
  return `${Math.round(contextWindow / 1000)}k`;
};

// WATCH-015: Get color based on context fill percentage
const getContextFillColor = (percentage: number): string => {
  if (percentage < 50) return 'green';
  if (percentage < 70) return 'yellow';
  if (percentage < 85) return 'magenta';
  return 'red';
};

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
  // WATCH-015: New props for header display
  /** Whether the model supports reasoning/extended thinking */
  displayReasoning?: boolean;
  /** Whether the model supports vision */
  displayHasVision?: boolean;
  /** Model's context window size in tokens */
  displayContextWindow?: number;
  /** Token usage from streaming updates */
  tokenUsage?: TokenTracker;
  /** Token usage from Rust state */
  rustTokens?: TokenTracker;
  /** Context fill percentage (0-100) */
  contextFillPercentage?: number;
  /** Whether turn select mode is active (for [SELECT] indicator) */
  isTurnSelectMode?: boolean;
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
  // WATCH-015: New props
  displayReasoning = false,
  displayHasVision = false,
  displayContextWindow = 0,
  tokenUsage = { inputTokens: 0, outputTokens: 0 },
  rustTokens = { inputTokens: 0, outputTokens: 0 },
  contextFillPercentage = 0,
  isTurnSelectMode = false,
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

  // WATCH-011: Build correlation maps between parent and watcher turns
  const { parentToWatcherTurns, watcherToParentTurns } = useMemo(
    () => buildCorrelationMaps(parentConversation, watcherConversation),
    [parentConversation, watcherConversation]
  );

  // WATCH-011: Compute cross-pane highlighted turns based on selection
  const crossPaneHighlightedTurns = useMemo(() => {
    const highlighted = new Set<number>();

    if (activePane === 'parent' && parentSelection.isSelectMode) {
      // Parent pane is active with selection - highlight correlated watcher turns
      const selectedIndex = parentSelection.selectionRef.current.selectedIndex;
      const selectedLine = parentConversation[selectedIndex];
      if (selectedLine) {
        const parentTurn = selectedLine.messageIndex;
        const watcherTurns = parentToWatcherTurns.get(parentTurn);
        if (watcherTurns) {
          watcherTurns.forEach(t => highlighted.add(t));
        }
      }
    } else if (activePane === 'watcher' && watcherSelection.isSelectMode) {
      // Watcher pane is active with selection - highlight correlated parent turns
      const selectedIndex = watcherSelection.selectionRef.current.selectedIndex;
      const selectedLine = watcherConversation[selectedIndex];
      if (selectedLine) {
        const watcherTurn = selectedLine.messageIndex;
        const parentTurns = watcherToParentTurns.get(watcherTurn);
        if (parentTurns) {
          parentTurns.forEach(t => highlighted.add(t));
        }
      }
    }

    return highlighted;
  }, [
    activePane,
    parentSelection.isSelectMode,
    watcherSelection.isSelectMode,
    parentSelection.selectionRef,
    watcherSelection.selectionRef,
    parentConversation,
    watcherConversation,
    parentToWatcherTurns,
    watcherToParentTurns,
  ]);

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
  // header(2) + input area(2)
  const outerReservedLines = 4;
  const paneHeaderLines = 2;
  const splitPanesHeight = Math.max(1, terminalHeight - 1 - outerReservedLines);
  const virtualListHeight = Math.max(1, splitPanesHeight - paneHeaderLines);
  const paneContentWidth = Math.floor(terminalWidth / 2) - 6;

  // Render function for conversation lines with selection and cross-pane highlighting
  const renderConversationItem = useCallback((
    isParentPane: boolean,
    selectMode: boolean,
    paneLines: ConversationLine[],
    crossPaneHighlighted: Set<number>
  ) => (
    line: ConversationLine,
    index: number,
    _isSelected: boolean,
    selectedIndex: number
  ) => {
    const paneActive = isParentPane ? activePane === 'parent' : activePane === 'watcher';

    // WATCH-011: Check if this line's turn should be cross-pane highlighted
    // Cross-pane highlight applies to the INACTIVE pane
    const isCrossPaneHighlighted = !paneActive && crossPaneHighlighted.has(line.messageIndex);

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
    // WATCH-011: Cross-pane highlighted lines get cyan color
    // WATCH-012: Watcher role gets magenta color
    const baseColor = isCrossPaneHighlighted
      ? 'cyan'
      : line.role === 'user'
        ? 'green'
        : line.role === 'watcher'
          ? 'magenta'
          : line.isThinking
            ? 'yellow'
            : line.isError
              ? 'red'
              : undefined;

    return (
      <Box key={`line-${index}`}>
        <Text
          dimColor={!paneActive && !isCrossPaneHighlighted}
          color={baseColor}
          bold={isCrossPaneHighlighted}
          wrap="truncate"
        >
          {isCrossPaneHighlighted ? '│ ' : ''}{line.content}
        </Text>
      </Box>
    );
  }, [activePane, paneContentWidth]);

  return (
    <Box flexDirection="column" flexGrow={1}>
      {/* WATCH-015: Enhanced header with model info, capabilities, and token stats */}
      <Box
        borderStyle="single"
        borderBottom={true}
        borderTop={false}
        borderLeft={false}
        borderRight={false}
        paddingX={1}
        flexDirection="row"
        flexWrap="nowrap"
      >
        <Box flexGrow={1} flexShrink={1} overflow="hidden">
          {/* Watcher indicator in magenta */}
          <Text bold color="magenta">
            👁️ {watcherRoleName} (watching: {parentSessionName})
          </Text>
          {/* Model capability indicators */}
          {displayReasoning && <Text color="magenta"> [R]</Text>}
          {displayHasVision && <Text color="blue"> [V]</Text>}
          {displayContextWindow > 0 && (
            <Text dimColor>
              {' '}
              [{formatContextWindow(displayContextWindow)}]
            </Text>
          )}
          {/* WATCH-015: Turn select mode indicator */}
          {isTurnSelectMode && (
            <Text color="cyan" bold>
              {' '}
              [SELECT]
            </Text>
          )}
        </Box>
        {/* Right side: token stats and percentage */}
        <Box flexShrink={0} flexGrow={0}>
          {isLoading && <Text color="yellow">⏳ </Text>}
          {/* Token usage display */}
          <Text dimColor>
            tokens: {Math.max(tokenUsage.inputTokens, rustTokens.inputTokens)}↓ {Math.max(tokenUsage.outputTokens, rustTokens.outputTokens)}↑
          </Text>
          {/* Context fill percentage with color coding */}
          <Text color={getContextFillColor(contextFillPercentage)}>
            {' '}[{contextFillPercentage}%]
          </Text>
        </Box>
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
              renderItem={renderConversationItem(true, parentSelection.isSelectMode, parentConversation, crossPaneHighlightedTurns)}
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
              renderItem={renderConversationItem(false, watcherSelection.isSelectMode, watcherConversation, crossPaneHighlightedTurns)}
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

      {/* Input area - matches AgentView pattern with hints in placeholder */}
      <ConversationInputArea
        value={inputValue}
        onChange={onInputChange}
        onSubmit={onSubmit}
        isLoading={isLoading}
        placeholder={activeSelectMode
          ? "↑↓ Navigate | Enter Discuss | Tab/Esc Exit Select"
          : "Type a message... ('←/→' switch pane | 'Tab' select turn | '↑↓' scroll | 'Esc' cancel)"}
        isActive={!isLoading}
      />
    </Box>
  );
};
