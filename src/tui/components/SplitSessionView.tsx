/**
 * SplitSessionView - Split view for watcher sessions
 *
 * WATCH-010: Watcher Split View UI
 * WATCH-011: Cross-Pane Selection with Correlation IDs
 * WATCH-015: Watcher Session Header Indicator
 * WATCH-018: Extract Split View to Separate Component
 * VIEWNV-001: Unified Shift+Arrow Navigation
 * INPUT-001: Uses centralized input handling with LOW priority
 *
 * Features:
 * - Two vertical panes (parent left, watcher right)
 * - Left/Right arrows switch active pane
 * - Shift+Left/Right navigates between sessions (VIEWNV-001)
 * - Tab toggles turn-select mode in active pane
 * - Up/Down navigate turns when in select mode (via VirtualList)
 * - Enter on selected parent turn pre-fills input with "Discuss Selected" context
 * - Input area always sends to watcher session
 * - WATCH-011: Cross-pane highlighting shows correlation between parent/watcher turns
 * - WATCH-015: Header shows watcher info, model capabilities, token usage, and context fill
 */

import React, { useCallback, useMemo } from 'react';
import { Box, Text } from 'ink';
import { VirtualList } from './VirtualList';
import { ConversationInputArea } from './ConversationInputArea';
import { SelectionSeparatorBar } from './SelectionSeparatorBar';
import { SessionHeader } from './SessionHeader';
import { useTerminalSize } from '../hooks/useTerminalSize';
import { useTurnSelection } from '../hooks/useTurnSelection';
import { useWatcherHeaderInfo } from '../hooks/useWatcherHeaderInfo';
import { useFspecStore } from '../store/fspecStore';
import {
  getSelectionSeparatorType,
  getFirstContentOfTurn,
  generateDiscussSelectedPrefill,
  getContentLineCount,
} from '../utils/turnSelection';
import { buildCorrelationMaps } from '../utils/correlationMapping';
import { useSessionNavigation } from '../hooks/useSessionNavigation';
import { CreateSessionDialog } from '../../components/CreateSessionDialog';
import { useShowCreateSessionDialog, useSessionActions } from '../store/sessionStore';
import { SlashCommandPalette } from './SlashCommandPalette';
import { useSlashCommandInput } from '../hooks/useSlashCommandInput';
import type { ConversationLine } from '../types/conversation';
import type { TokenTracker } from '../utils/sessionHeaderUtils';
import { useInputCompat, InputPriority } from '../input/index';

interface SplitSessionViewProps {
  /** Current watcher session ID - used to compute watcher header info */
  sessionId: string;
  parentSessionName: string;
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
  // WATCH-015: Header display props
  /** Model ID to display */
  modelId: string;
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
  /** Callback when user wants to open full turn content modal (WATCH-016) */
  onOpenTurnContent?: (messageIndex: number, content: string) => void;
  // VIEWNV-001: Session navigation callbacks
  /** Callback when navigating to another session */
  onNavigate: (sessionId: string) => void;
  /** Callback when navigating to board view */
  onNavigateToBoard: () => void;
}

export const SplitSessionView: React.FC<SplitSessionViewProps> = ({
  sessionId,
  parentSessionName,
  terminalWidth,
  parentConversation,
  watcherConversation,
  inputValue,
  onInputChange,
  onSubmit,
  isLoading,
  // WATCH-015: Header props
  modelId,
  displayReasoning = false,
  displayHasVision = false,
  displayContextWindow = 0,
  tokenUsage = { inputTokens: 0, outputTokens: 0 },
  rustTokens = { inputTokens: 0, outputTokens: 0 },
  contextFillPercentage = 0,
  isTurnSelectMode = false,
  onOpenTurnContent,
  // VIEWNV-001: Navigation callbacks
  onNavigate,
  onNavigateToBoard,
}) => {
  const { height: terminalHeight } = useTerminalSize();

  // Get watcher header info (slug, instance number) from session ID
  const watcherHeaderInfo = useWatcherHeaderInfo(sessionId);

  // Get work unit ID attached to this session
  const getWorkUnitBySession = useFspecStore(state => state.getWorkUnitBySession);
  const workUnitId = getWorkUnitBySession(sessionId);

  // VIEWNV-001: Session navigation hook
  const showCreateSessionDialog = useShowCreateSessionDialog();
  const { closeCreateSessionDialog, prepareForNewSession, requestAutoCreateSession } = useSessionActions();
  const sessionNavigation = useSessionNavigation({
    onNavigate,
    onNavigateToBoard,
  });

  // TUI-050: Slash command autocomplete palette with clean input handling
  const slashCommand = useSlashCommandInput({
    inputValue,
    onInputChange,
    onExecuteCommand: (cmd) => onSubmit(cmd),
    disabled: false, // SplitSessionView doesn't have overlays that would conflict
  });

  // TUI-050: Use slashCommand.handleInputChange from the hook
  const handleInputChangeWithSlash = slashCommand.handleInputChange;

  // Pane state
  const [activePane, setActivePane] = React.useState<'parent' | 'watcher'>(
    'watcher'
  );

  // Turn selection state (separate for each pane)
  const parentSelection = useTurnSelection();
  const watcherSelection = useTurnSelection();

  // Get current select mode for active pane
  const activeSelectMode =
    activePane === 'parent'
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

  // Handle keyboard input with LOW priority (view navigation)
  useInputCompat({
    id: 'split-session-view',
    priority: InputPriority.LOW,
    description: 'Split session view keyboard navigation',
    isActive: !showCreateSessionDialog,
    handler: (input, key) => {
      // VIEWNV-001: Skip input handling when create dialog is showing
      if (showCreateSessionDialog) return false;

      // TUI-050: Slash command palette keyboard handling
      // Handle BEFORE isLoading check - user should be able to navigate/select commands while loading
      if (slashCommand.handleInput(input, key)) {
        return true;
      }

      // Skip most input handling while loading (but slash commands are allowed above)
      if (isLoading) return false;

      // VIEWNV-001: Shift+Left/Right for session navigation
      // Handle escape sequences first (some terminals), then Ink key detection
      if (
        input.includes('[1;2D') ||
        input.includes('\x1b[1;2D') ||
        (key.shift && key.leftArrow)
      ) {
        sessionNavigation.handleShiftLeft();
        return true;
      }
      if (
        input.includes('[1;2C') ||
        input.includes('\x1b[1;2C') ||
        (key.shift && key.rightArrow)
      ) {
        sessionNavigation.handleShiftRight();
        return true;
      }

      // Left/Right arrows switch panes (only when NOT in select mode and NOT shift)
      if (!activeSelectMode) {
        if (key.leftArrow && !key.shift) {
          setActivePane('parent');
          return true;
        }
        if (key.rightArrow && !key.shift) {
          setActivePane('watcher');
          return true;
        }
      }

      // Tab toggles turn-select mode in active pane
      if (key.tab) {
        if (activePane === 'parent') {
          parentSelection.toggleSelectMode();
        } else {
          watcherSelection.toggleSelectMode();
        }
        return true;
      }

      // Escape exits select mode
      if (key.escape && activeSelectMode) {
        if (activePane === 'parent') {
          parentSelection.exitSelectMode();
        } else {
          watcherSelection.exitSelectMode();
        }
        return true;
      }

      // Enter in parent pane select mode: "Discuss Selected" - pre-fill input
      if (key.return && activePane === 'parent' && parentSelection.isSelectMode) {
        const selectedIndex = parentSelection.selectionRef.current.selectedIndex;
        const selectedLine = parentConversation[selectedIndex];

        if (selectedLine) {
          const messageIndex = selectedLine.messageIndex;
          const turnNumber = messageIndex + 1; // 1-indexed for display
          const turnContent = getFirstContentOfTurn(
            parentConversation,
            messageIndex
          );

          if (turnContent) {
            const prefill = generateDiscussSelectedPrefill(
              turnNumber,
              turnContent
            );
            onInputChange(prefill);
            parentSelection.exitSelectMode();
          }
        }
        return true;
      }

      // WATCH-016: Enter in watcher pane select mode: Open TurnContentModal
      if (
        key.return &&
        activePane === 'watcher' &&
        watcherSelection.isSelectMode
      ) {
        const selectedIndex = watcherSelection.selectionRef.current.selectedIndex;
        const selectedLine = watcherConversation[selectedIndex];

        if (selectedLine && onOpenTurnContent) {
          // Collect all content lines for this turn to build full content
          const fullContent = watcherConversation
            .filter(
              line =>
                line.messageIndex === selectedLine.messageIndex &&
                !line.isSeparator
            )
            .map(line => line.content)
            .join('\n');
          onOpenTurnContent(selectedLine.messageIndex, fullContent);
        }
        return true;
      }

      return false;
    },
  });

  // Calculate layout dimensions
  // Layout: SessionHeader(2: content+border) + split panes (flex) + input area(1)
  // Each pane has: pane header(1) + virtual list content
  const reservedHeight = 4; // SessionHeader(2) + input area(1) + buffer(1)
  const paneHeaderHeight = 1;
  const availableHeight = Math.max(1, terminalHeight - reservedHeight);
  const virtualListHeight = Math.max(1, availableHeight - paneHeaderHeight);
  const paneContentWidth = Math.floor(terminalWidth / 2) - 4;

  // Render function for conversation lines with selection and cross-pane highlighting
  const renderConversationItem = useCallback(
    (
      isParentPane: boolean,
      selectMode: boolean,
      paneLines: ConversationLine[],
      crossPaneHighlighted: Set<number>
    ) =>
      (
        line: ConversationLine,
        index: number,
        _isSelected: boolean,
        selectedIndex: number
      ) => {
        const paneActive = isParentPane
          ? activePane === 'parent'
          : activePane === 'watcher';

        // WATCH-011: Check if this line's turn should be cross-pane highlighted
        // Cross-pane highlight applies to the INACTIVE pane
        const isCrossPaneHighlighted =
          !paneActive && crossPaneHighlighted.has(line.messageIndex);

        // Selection separator bars
        const separatorType = getSelectionSeparatorType(
          line,
          index,
          paneLines,
          selectedIndex,
          selectMode
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
              {isCrossPaneHighlighted ? '│ ' : ''}
              {line.content}
            </Text>
          </Box>
        );
      },
    [activePane, paneContentWidth]
  );

  return (
    <Box flexDirection="column" flexGrow={1} overflow="hidden">
      {/* WATCH-015: Shared session header with watcher info */}
      <SessionHeader
        modelId={modelId}
        hasReasoning={displayReasoning}
        hasVision={displayHasVision}
        contextWindow={displayContextWindow}
        isSelectMode={isTurnSelectMode}
        isLoading={isLoading}
        tokenUsage={tokenUsage}
        rustTokens={rustTokens}
        contextFillPercentage={contextFillPercentage}
        workUnitId={workUnitId}
        watcherInfo={watcherHeaderInfo ? {
          slug: watcherHeaderInfo.slug,
          instanceNumber: watcherHeaderInfo.instanceNumber,
        } : undefined}
      />

      {/* Split panes container */}
      <Box flexDirection="row" flexGrow={1} overflow="hidden">
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
          overflow="hidden"
        >
          {/* Pane header */}
          <Box height={1} paddingX={1} overflow="hidden">
            <Text bold dimColor={activePane !== 'parent'} wrap="truncate">
              {activePane === 'parent' ? '> ' : '  '}PARENT
              {parentSelection.isSelectMode ? ' [SELECT]' : ''}
              {' '}({getContentLineCount(parentConversation)})
            </Text>
          </Box>
          {/* Pane content */}
          <Box flexGrow={1} flexBasis={0} overflow="hidden">
            <VirtualList
              items={parentConversation}
              renderItem={renderConversationItem(
                true,
                parentSelection.isSelectMode,
                parentConversation,
                crossPaneHighlightedTurns
              )}
              keyExtractor={(_line, index) => `parent-${index}`}
              emptyMessage="No parent conversation"
              showScrollbar={true}
              isFocused={activePane === 'parent' && !isLoading}
              scrollToEnd={true}
              selectionMode={parentSelection.isSelectMode ? 'item' : 'scroll'}
              groupBy={
                parentSelection.isSelectMode
                  ? line => line.messageIndex
                  : undefined
              }
              groupPaddingBefore={parentSelection.isSelectMode ? 1 : 0}
              selectionRef={parentSelection.selectionRef}
              fixedHeight={virtualListHeight}
            />
          </Box>
        </Box>

        {/* Right pane: Watcher conversation */}
        <Box flexDirection="column" flexGrow={1} flexBasis={0} overflow="hidden">
          {/* Pane header */}
          <Box height={1} paddingX={1} overflow="hidden">
            <Text bold dimColor={activePane !== 'watcher'} wrap="truncate">
              {activePane === 'watcher' ? '> ' : '  '}WATCHER
              {watcherSelection.isSelectMode ? ' [SELECT]' : ''}
              {' '}({getContentLineCount(watcherConversation)})
            </Text>
          </Box>
          {/* Pane content */}
          <Box flexGrow={1} flexBasis={0} overflow="hidden">
            <VirtualList
              items={watcherConversation}
              renderItem={renderConversationItem(
                false,
                watcherSelection.isSelectMode,
                watcherConversation,
                crossPaneHighlightedTurns
              )}
              keyExtractor={(_line, index) => `watcher-${index}`}
              emptyMessage="Start chatting with your watcher..."
              showScrollbar={true}
              isFocused={activePane === 'watcher' && !isLoading}
              scrollToEnd={true}
              selectionMode={watcherSelection.isSelectMode ? 'item' : 'scroll'}
              groupBy={
                watcherSelection.isSelectMode
                  ? line => line.messageIndex
                  : undefined
              }
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
        onChange={handleInputChangeWithSlash}
        onSubmit={onSubmit}
        isLoading={isLoading}
        placeholder={
          activeSelectMode
            ? '↑↓ Navigate | Enter Discuss | Tab/Esc Exit Select'
            : "Type a message... ('←/→' pane | 'Shift+←/→' sessions | 'Tab' select | 'Esc' cancel)"
        }
        isActive={!isLoading && !showCreateSessionDialog}
        suppressEnter={slashCommand.isVisible}
      />

      {/* TUI-050: Slash command autocomplete palette */}
      {slashCommand.isVisible && (
        <SlashCommandPalette
          isVisible={slashCommand.isVisible}
          filter={slashCommand.filter}
          commands={slashCommand.filteredCommands}
          selectedIndex={slashCommand.selectedIndex}
          dialogWidth={slashCommand.dialogWidth}
          maxVisibleItems={8}
        />
      )}

      {/* VIEWNV-001: Create session dialog (shown when navigating past right edge) */}
      {showCreateSessionDialog && (
        <CreateSessionDialog
          onConfirm={() => {
            // Prepare for new session (atomic transition via store)
            prepareForNewSession();
            // VIEWNV-001: Request immediate session creation so /thinking works
            requestAutoCreateSession();
          }}
          onCancel={() => {
            closeCreateSessionDialog();
          }}
        />
      )}
    </Box>
  );
};
