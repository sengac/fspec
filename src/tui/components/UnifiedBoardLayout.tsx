/**
 * UnifiedBoardLayout - Pure flexbox layout with Text borders
 *
 * ALL FLEXBOX - NO borderStyle on any Box component
 * Borders are Text components with box-drawing characters
 * Content uses flexGrow/flexShrink for responsive sizing
 */

import React, { useMemo, useState, useEffect } from 'react';
import { Box, Text, useInput, useStdout, useStdin } from 'ink';
import chalk from 'chalk';
import { Logo } from './Logo';
import { CheckpointStatus } from './CheckpointStatus';
import { KeybindingShortcuts } from './KeybindingShortcuts';
import { WorkUnitTitle } from './WorkUnitTitle';
import { WorkUnitDescription } from './WorkUnitDescription';
import { WorkUnitMetadata } from './WorkUnitMetadata';
import { WorkUnitAttachments } from './WorkUnitAttachments';
import { useFspecStore } from '../store/fspecStore';

interface StateHistoryEntry {
  state: string;
  timestamp: string;
}

interface WorkUnit {
  id: string;
  title: string;
  type: 'story' | 'task' | 'bug';
  estimate?: number;
  status: string;
  description?: string;
  epic?: string;
  stateHistory?: StateHistoryEntry[];
  attachments?: string[];
}

interface UnifiedBoardLayoutProps {
  workUnits: WorkUnit[];
  selectedWorkUnit?: WorkUnit | null;
  focusedColumnIndex?: number;
  selectedWorkUnitIndex?: number;
  selectedId?: string | null; // For testing - ID of selected work unit
  onColumnChange?: (delta: number) => void;
  onWorkUnitChange?: (delta: number) => void;
  onEnter?: () => void;
  onPageUp?: () => void;
  onPageDown?: () => void;
  onMoveUp?: () => void;
  onMoveDown?: () => void;
  cwd?: string;
  terminalWidth?: number;
  terminalHeight?: number;
  isDialogOpen?: boolean; // Disable input when dialog is open
}

const STATES = ['backlog', 'specifying', 'testing', 'implementing', 'validating', 'done', 'blocked'] as const;

// Helper: Calculate column widths
const calculateColumnWidths = (terminalWidth: number): { baseWidth: number; remainder: number } => {
  const borders = 2;
  const separators = STATES.length - 1;
  const availableWidth = terminalWidth - borders - separators;
  const baseWidth = Math.floor(availableWidth / STATES.length);
  const remainder = availableWidth % STATES.length;
  return { baseWidth: Math.max(8, baseWidth), remainder: baseWidth >= 8 ? remainder : 0 };
};

const getColumnWidth = (columnIndex: number, baseWidth: number, remainder: number): number => {
  return columnIndex < remainder ? baseWidth + 1 : baseWidth;
};

// Helper: Calculate visual width accounting for emoji width
const getVisualWidth = (text: string): number => {
  let width = 0;
  for (const char of text) {
    const code = char.codePointAt(0) || 0;
    // Emoji ranges that render as 2 columns:
    // U+2300-U+27BF (Miscellaneous Technical, Dingbats) - includes ⏩ (U+23E9)
    // U+1F000+ (Emoticons, symbols, etc.)
    // Arrows like ↓ (U+2193) render as 1 column
    const isWide = (code >= 0x2300 && code <= 0x27BF) || code >= 0x1F000;
    width += isWide ? 2 : 1;
  }
  return width;
};

const fitToWidth = (text: string, width: number): string => {
  const visualWidth = getVisualWidth(text);

  if (visualWidth > width) {
    // Truncate while being careful about emoji boundaries
    let result = '';
    let currentVisualWidth = 0;
    for (const char of text) {
      const code = char.codePointAt(0) || 0;
      const isWide = (code >= 0x2300 && code <= 0x27BF) || code >= 0x1F000;
      const charWidth = isWide ? 2 : 1;
      if (currentVisualWidth + charWidth > width) break;
      result += char;
      currentVisualWidth += charWidth;
    }
    // Pad to exact width
    return result + ' '.repeat(width - currentVisualWidth);
  } else if (visualWidth < width) {
    // Pad with spaces to reach visual width
    return text + ' '.repeat(width - visualWidth);
  }
  return text;
};

const buildBorderRow = (
  baseWidth: number,
  remainder: number,
  left: string,
  mid: string,
  right: string,
  separatorType: 'plain' | 'top' | 'cross' | 'bottom' = 'cross'
): string => {
  const separatorChar = {
    plain: '─',
    top: '┬',
    cross: '┼',
    bottom: '┴',
  }[separatorType];
  return left + STATES.map((_, idx) => '─'.repeat(getColumnWidth(idx, baseWidth, remainder))).join(separatorChar) + right;
};

const calculateViewportHeight = (terminalHeight: number): number => {
  // Fixed rows breakdown:
  // 1 (top border) + 4 (header) + 1 (header separator) +
  // 5 (details) + 1 (details separator with ┬) + 1 (column headers) +
  // 1 (column separator with ┼) + 1 (footer separator with ┴) +
  // 1 (footer) + 1 (bottom border) + 1 (bottom padding) = 18
  const fixedRows = 18;
  return Math.max(5, terminalHeight - fixedRows);
};

export const UnifiedBoardLayout: React.FC<UnifiedBoardLayoutProps> = ({
  workUnits,
  selectedWorkUnit,
  focusedColumnIndex = 0,
  selectedWorkUnitIndex = 0,
  selectedId,
  onColumnChange,
  onWorkUnitChange,
  onEnter,
  onPageUp,
  onPageDown,
  onMoveUp,
  onMoveDown,
  cwd,
  terminalWidth: propTerminalWidth,
  terminalHeight: propTerminalHeight,
  isDialogOpen = false,
}) => {
  const { stdout } = useStdout();
  const terminalWidth = propTerminalWidth ?? (stdout?.columns || 80);
  const terminalHeight = propTerminalHeight ?? (stdout?.rows || 24);

  // Read checkpoint counts from Zustand store (updated via IPC)
  const checkpointCounts = useFspecStore(state => state.checkpointCounts);
  const loadCheckpointCounts = useFspecStore(state => state.loadCheckpointCounts);

  // Load checkpoint counts on mount
  useEffect(() => {
    void loadCheckpointCounts();
  }, [loadCheckpointCounts]);

  const { baseWidth: colWidth, remainder: colRemainder } = useMemo(
    () => calculateColumnWidths(terminalWidth),
    [terminalWidth]
  );

  // Calculate viewport height (BOARD-014: dynamic column height based on terminal height)
  const VIEWPORT_HEIGHT = useMemo(() => calculateViewportHeight(terminalHeight), [terminalHeight]);

  const totalWidth = STATES.reduce((sum, _, idx) => sum + getColumnWidth(idx, colWidth, colRemainder), 0) + (STATES.length - 1);

  // Column scroll offsets (BOARD-007)
  const [scrollOffsets, setScrollOffsets] = useState<Record<string, number>>({
    backlog: 0,
    specifying: 0,
    testing: 0,
    implementing: 0,
    validating: 0,
    done: 0,
    blocked: 0,
  });

  // Group work units by status
  const groupedWorkUnits = useMemo(() => {
    return STATES.map(status => {
      const units = workUnits.filter(wu => wu.status === status);
      const totalPoints = units.reduce((sum, wu) => sum + (wu.estimate || 0), 0);
      return { status, units, count: units.length, totalPoints };
    });
  }, [workUnits]);

  // Compute last changed work unit (TUI-017 - for emoji indicators)
  const lastChangedWorkUnit = useMemo(() => {
    if (workUnits.length === 0) return null;

    return workUnits.reduce((latest, current) => {
      const latestStateTimestamp = latest.stateHistory && latest.stateHistory.length > 0
        ? new Date(latest.stateHistory[latest.stateHistory.length - 1].timestamp).getTime()
        : 0;
      const currentStateTimestamp = current.stateHistory && current.stateHistory.length > 0
        ? new Date(current.stateHistory[current.stateHistory.length - 1].timestamp).getTime()
        : 0;

      return currentStateTimestamp > latestStateTimestamp ? current : latest;
    });
  }, [workUnits]);

  // Auto-scroll to keep selected work unit visible (BOARD-007)
  useEffect(() => {
    const currentColumn = STATES[focusedColumnIndex];
    const columnUnits = groupedWorkUnits[focusedColumnIndex].units;

    if (columnUnits.length === 0) return;

    setScrollOffsets(prev => {
      const scrollOffset = prev[currentColumn];
      const firstVisible = scrollOffset + (scrollOffset > 0 ? 1 : 0); // Account for up arrow
      const lastVisible = scrollOffset + VIEWPORT_HEIGHT - 1 - (scrollOffset + VIEWPORT_HEIGHT < columnUnits.length ? 1 : 0); // Account for down arrow

      // Check if selected item is visible
      if (selectedWorkUnitIndex >= firstVisible && selectedWorkUnitIndex <= lastVisible) {
        // Already visible, no scroll needed
        return prev;
      }

      // Need to scroll - calculate new offset
      let newOffset;

      if (selectedWorkUnitIndex < firstVisible) {
        // Scroll up: position selected item near top
        newOffset = Math.max(0, selectedWorkUnitIndex - 1);
        if (selectedWorkUnitIndex === 0) {
          newOffset = 0;
        }
      } else {
        // Scroll down: position selected item near bottom
        const estimatedEffectiveHeight = VIEWPORT_HEIGHT - 2; // Assume both arrows
        newOffset = selectedWorkUnitIndex - estimatedEffectiveHeight + 1;

        const testUpArrow = newOffset > 0;
        const testDownArrow = newOffset + VIEWPORT_HEIGHT < columnUnits.length;
        const testArrows = (testUpArrow ? 1 : 0) + (testDownArrow ? 1 : 0);
        const testEffectiveHeight = VIEWPORT_HEIGHT - testArrows;

        newOffset = selectedWorkUnitIndex - testEffectiveHeight + (testUpArrow ? 0 : 1);
      }

      // Clamp to valid range
      const maxOffset = Math.max(0, columnUnits.length - VIEWPORT_HEIGHT);
      newOffset = Math.max(0, Math.min(newOffset, maxOffset));

      return {
        ...prev,
        [currentColumn]: newOffset,
      };
    });
  }, [selectedWorkUnitIndex, focusedColumnIndex, groupedWorkUnits, VIEWPORT_HEIGHT]);

  // Handle mouse scroll for focused column (TUI-010)
  const handleColumnScroll = (direction: 'up' | 'down'): void => {
    if (direction === 'down') {
      // Scroll down: move selector down
      onWorkUnitChange?.(1);
    } else if (direction === 'up') {
      // Scroll up: move selector up
      onWorkUnitChange?.(-1);
    }
  };

  // Access raw stdin to handle Home/End keys (which useInput filters out)
  // @ts-expect-error internal_eventEmitter is not in public types
  const { internal_eventEmitter } = useStdin();

  useEffect(() => {
    if (!internal_eventEmitter || !onWorkUnitChange) return;

    const handleRawInput = (data: string) => {
      // Parse Home/End keys that useInput filters out
      // Home: ESC[H, ESC[1~, ESC[7~, ESCOH
      // End: ESC[F, ESC[4~, ESC[8~, ESCOF
      // ESC is \u001B (charCode 27)
      const isHome = data === '\u001B[H' || data === '\u001B[1~' || data === '\u001B[7~' || data === '\u001BOH';
      const isEnd = data === '\u001B[F' || data === '\u001B[4~' || data === '\u001B[8~' || data === '\u001BOF';

      if (isHome) {
        const columnUnits = groupedWorkUnits[focusedColumnIndex].units;
        if (columnUnits.length > 0) {
          onWorkUnitChange(-selectedWorkUnitIndex);
        }
      } else if (isEnd) {
        const columnUnits = groupedWorkUnits[focusedColumnIndex].units;
        if (columnUnits.length > 0) {
          onWorkUnitChange(columnUnits.length - 1 - selectedWorkUnitIndex);
        }
      }
    };

    internal_eventEmitter.on('input', handleRawInput);
    return () => {
      internal_eventEmitter.removeListener('input', handleRawInput);
    };
  }, [internal_eventEmitter, onWorkUnitChange, selectedWorkUnitIndex, focusedColumnIndex, groupedWorkUnits]);

  // Handle keyboard input
  useInput((input, key) => {
    // Mouse scroll handling (TUI-010)
    // Parse raw escape sequences for terminals that don't parse mouse events
    if (input && input.startsWith('[M')) {
      const buttonByte = input.charCodeAt(2);
      if (buttonByte === 96) {  // Scroll up (ASCII '`')
        handleColumnScroll('up');
        return;
      } else if (buttonByte === 97) {  // Scroll down (ASCII 'a')
        handleColumnScroll('down');
        return;
      }
    }

    // Handle Ink-parsed mouse events (primary method)
    if (key.mouse) {
      if (key.mouse.button === 'wheelDown') {
        handleColumnScroll('down');
        return;
      } else if (key.mouse.button === 'wheelUp') {
        handleColumnScroll('up');
        return;
      }
    }

    // Page Up/Down: Move selector by viewport height
    if (key.pageDown) {
      onWorkUnitChange?.(VIEWPORT_HEIGHT);
      return;
    }

    if (key.pageUp) {
      onWorkUnitChange?.(-VIEWPORT_HEIGHT);
      return;
    }

    // Home/End are handled via raw stdin event emitter above
    // because useInput filters them out before we can see them

    // Arrow keys handled by parent
    if (key.leftArrow || key.rightArrow) {
      onColumnChange?.(key.rightArrow ? 1 : -1);
    }
    if (key.upArrow || key.downArrow) {
      onWorkUnitChange?.(key.downArrow ? 1 : -1);
    }
    if (key.return) {
      onEnter?.();
    }

    // BOARD-010: Bracket keys for priority reordering
    if (input === '[') {
      onMoveUp?.();
    }
    if (input === ']') {
      onMoveDown?.();
    }

    // TUI-019: Open attachment dialog (handled at BoardView level)
    // The 'a' key handler is now in BoardView to show AttachmentDialog
  }, { isActive: !isDialogOpen });

  return (
    <Box flexDirection="column">
      {/* Top border */}
      <Text>{'┌' + '─'.repeat(totalWidth) + '┐'}</Text>

      {/* Header - 4 rows with Logo + CheckpointStatus/KeybindingShortcuts */}
      <Box flexDirection="row" height={4}>
        <Box flexDirection="column" width={1}>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
        </Box>
        <Box flexGrow={1} flexDirection="row" paddingX={1}>
          <Logo />
          <Box flexGrow={1} flexDirection="column">
            <CheckpointStatus manualCount={checkpointCounts.manual} autoCount={checkpointCounts.auto} />
            <KeybindingShortcuts />
          </Box>
        </Box>
        <Box flexDirection="column" width={1}>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
        </Box>
      </Box>

      {/* Separator after header */}
      <Text>{buildBorderRow(colWidth, colRemainder, '├', '─', '┤', 'plain')}</Text>

      {/* Work unit details - 5 rows */}
      <Box flexDirection="row" height={5}>
        <Box flexDirection="column" width={1}>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
        </Box>
        <Box flexGrow={1} flexDirection="column" flexShrink={1}>
          {selectedWorkUnit ? (
            <>
              <WorkUnitTitle id={selectedWorkUnit.id} title={selectedWorkUnit.title} />
              <WorkUnitDescription
                description={selectedWorkUnit.description || ''}
                width={terminalWidth}
              />
              <WorkUnitAttachments
                attachments={selectedWorkUnit.attachments}
                width={terminalWidth}
              />
              <WorkUnitMetadata
                epic={selectedWorkUnit.epic}
                estimate={selectedWorkUnit.estimate}
                status={selectedWorkUnit.status}
              />
            </>
          ) : (
            <Box flexGrow={1} justifyContent="center" alignItems="center">
              <Text>No work unit selected</Text>
            </Box>
          )}
        </Box>
        <Box flexDirection="column" width={1}>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
          <Text>│</Text>
        </Box>
      </Box>

      {/* Separator before columns (with top junctions ┬) */}
      <Text>{buildBorderRow(colWidth, colRemainder, '├', '┬', '┤', 'top')}</Text>

      {/* Column headers - string-based rendering with focus highlighting */}
      <Text>
        {'│' + STATES.map((state, idx) => {
          const header = state.toUpperCase();
          const currentColWidth = getColumnWidth(idx, colWidth, colRemainder);
          const paddedHeader = fitToWidth(header, currentColWidth);
          // Highlight focused column in cyan, others in gray
          return idx === focusedColumnIndex ? chalk.cyan(paddedHeader) : chalk.gray(paddedHeader);
        }).join('│') + '│'}
      </Text>

      {/* Column header separator (with cross junctions ┼) */}
      <Text>{buildBorderRow(colWidth, colRemainder, '├', '┼', '┤', 'cross')}</Text>

      {/* Column content - string-based rendering with colors and animations */}
      {Array.from({ length: VIEWPORT_HEIGHT }).map((_, rowIndex) => {
        const cells = STATES.map((state, colIndex) => {
          const column = groupedWorkUnits[colIndex];
          const scrollOffset = scrollOffsets[state];
          const itemIndex = scrollOffset + rowIndex;
          const currentColWidth = getColumnWidth(colIndex, colWidth, colRemainder);

          // Show scroll indicators ONLY if there are items to scroll to
          if (rowIndex === 0 && scrollOffset > 0 && column.units.length > 0) {
            return fitToWidth('↑', currentColWidth); // Up arrow at top when scrolled down
          }
          if (rowIndex === VIEWPORT_HEIGHT - 1 && scrollOffset + VIEWPORT_HEIGHT < column.units.length) {
            return fitToWidth('↓', currentColWidth); // Down arrow at bottom when more items below
          }

          if (itemIndex >= column.units.length) {
            return fitToWidth('', currentColWidth);
          }

          const wu = column.units[itemIndex];
          const estimate = wu.estimate || 0;
          const storyPointsText = estimate > 0 ? ` [${estimate}]` : '';

          // Check if this work unit is the last changed (TUI-017)
          const isLastChanged = lastChangedWorkUnit?.id === wu.id;
          const text = isLastChanged
            ? `⏩ ${wu.id}${storyPointsText} ⏩`
            : `${wu.id}${storyPointsText}`;

          const paddedText = fitToWidth(text, currentColWidth);

          // Check if this work unit is selected
          const isSelected = colIndex === focusedColumnIndex && itemIndex === selectedWorkUnitIndex;

          // Apply color-coding without shimmer animation (TUI-017)
          if (isSelected) {
            return chalk.bgGreen.black(paddedText);
          } else {
            if (wu.type === 'bug') {
              return chalk.red(paddedText);
            } else if (wu.type === 'task') {
              return chalk.blue(paddedText);
            } else {
              return chalk.white(paddedText);
            }
          }
        });

        return (
          <Text key={rowIndex}>{'│' + cells.join('│') + '│'}</Text>
        );
      })}

      {/* Footer separator (with bottom junctions ┴) */}
      <Text>{buildBorderRow(colWidth, colRemainder, '├', '┴', '┤', 'bottom')}</Text>

      {/* Footer */}
      <Box flexDirection="row" height={1}>
        <Text>│</Text>
        <Box flexGrow={1} justifyContent="center">
          <Text>← → Columns ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Details ◆ ESC Back</Text>
        </Box>
        <Text>│</Text>
      </Box>

      {/* Bottom border */}
      <Text>{'└' + '─'.repeat(totalWidth) + '┘'}</Text>

      {/* Bottom padding to prevent cutoff */}
      <Text>{''}</Text>
    </Box>
  );
};
