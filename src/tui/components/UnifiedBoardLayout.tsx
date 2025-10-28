/**
 * UnifiedBoardLayout - Table-based Kanban board layout
 *
 * Coverage:
 * - ITF-004: Fix TUI Kanban column layout to match table style
 *
 * Implements unified table layout with:
 * - Box-drawing characters (┌┬┐ ├┼┤ └┴┘)
 * - Integrated Git Stashes/Changed Files panels
 * - Scrolling with Page Up/Down support
 * - Scroll indicators (↑ ↓)
 */

import React, { useMemo, useEffect, useState } from 'react';
import { Box, Text, useInput, useStdout } from 'ink';
import chalk from 'chalk';
import { Logo } from './Logo';
import { GitStashesPanel } from './GitStashesPanel';
import { ChangedFilesPanel } from './ChangedFilesPanel';

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
  dependencies?: string[];
  epic?: string;
  updated?: string;
  stateHistory?: StateHistoryEntry[];
}

interface UnifiedBoardLayoutProps {
  workUnits: WorkUnit[];
  stashes?: any[];
  stagedFiles?: string[];
  unstagedFiles?: string[];
  focusedColumnIndex?: number;
  selectedWorkUnitIndex?: number;
  selectedWorkUnit?: WorkUnit | null;
  onColumnChange?: (delta: number) => void;
  onWorkUnitChange?: (delta: number) => void;
  onEnter?: () => void;
  onPageUp?: () => void;
  onPageDown?: () => void;
  onMoveUp?: () => void;
  onMoveDown?: () => void;
  // BOARD-014: Optional terminal dimensions (for testing)
  terminalWidth?: number;
  terminalHeight?: number;
}

const STATES = ['backlog', 'specifying', 'testing', 'implementing', 'validating', 'done', 'blocked'] as const;

// Helper: Calculate optimal column width (same pattern as BoardDisplay)
const calculateColumnWidth = (terminalWidth: number): number => {
  const borders = 2; // Left and right outer borders
  const separators = STATES.length - 1; // Column separators (6 for 7 columns)
  const availableWidth = terminalWidth - borders - separators;
  const calculatedWidth = Math.floor(availableWidth / STATES.length);

  // Return calculated width (will adapt to terminal size)
  return Math.max(8, calculatedWidth); // Absolute minimum of 8 chars (same as BoardDisplay)
};

// Helper: Calculate viewport height based on terminal height (BOARD-014)
// Available height = terminal rows - all fixed-height sections
const calculateViewportHeight = (terminalHeight: number): number => {
  // Fixed rows count:
  // - Top border: 1
  // - Git Stashes header + content: 2
  // - Changed Files header + content: 2
  // - Separator after git: 1
  // - Work Unit Details header: 1
  // - Work Unit Details content: 4
  // - Separator before columns: 1
  // - Column headers: 1
  // - Header separator: 1
  // - Bottom separator: 1
  // - Footer: 1
  // - Bottom border: 1
  // Total: 17 fixed rows
  const fixedRows = 17;
  const availableRows = terminalHeight - fixedRows;

  // Ensure minimum of 5 rows for columns (fallback for very small terminals)
  return Math.max(5, availableRows);
};

// Helper: Pad or truncate text
const fitToWidth = (text: string, width: number): string => {
  if (text.length > width) {
    return text.substring(0, width);
  }
  return text.padEnd(width, ' ');
};

// Helper: Center text within given width
const centerText = (text: string, width: number): string => {
  if (text.length >= width) {
    return text.substring(0, width);
  }
  const totalPadding = width - text.length;
  const leftPadding = Math.floor(totalPadding / 2);
  const rightPadding = totalPadding - leftPadding;
  return ' '.repeat(leftPadding) + text + ' '.repeat(rightPadding);
};

// Helper: Apply character-by-character shimmer gradient (BOARD-009)
// Applies 3-level gradient: gray (dim) → base color → bright color → base color → gray
const applyCharacterShimmer = (
  text: string,
  shimmerPosition: number,
  type: 'story' | 'bug' | 'task'
): string => {
  const colors: Record<string, { base: string; bright: string }> = {
    story: { base: 'white', bright: 'whiteBright' },
    bug: { base: 'red', bright: 'redBright' },
    task: { base: 'blue', bright: 'blueBright' },
  };

  // Default to story colors if type is unknown
  const { base, bright } = colors[type] || colors.story;

  return text
    .split('')
    .map((char, idx) => {
      const distance = Math.abs(idx - shimmerPosition);
      if (distance === 0) {
        // Peak brightness at shimmer position
        return (chalk as any)[bright](char);
      } else if (distance === 1) {
        // Adjacent characters: base color
        return (chalk as any)[base](char);
      } else {
        // Further characters: dim (gray)
        return chalk.gray(char);
      }
    })
    .join('');
};

// Helper: Apply character-by-character background shimmer gradient (BOARD-009)
// For selected + last-changed work units
const applyBackgroundCharacterShimmer = (
  text: string,
  shimmerPosition: number
): string => {
  return text
    .split('')
    .map((char, idx) => {
      const distance = Math.abs(idx - shimmerPosition);
      if (distance === 0) {
        // Peak brightness background at shimmer position
        return chalk.bgGreenBright.black(char);
      } else {
        // All other characters: base green background
        return chalk.bgGreen.black(char);
      }
    })
    .join('');
};

// Helper: Build border row with separator type
const buildBorderRow = (
  colWidth: number,
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

  return left + STATES.map(() => '─'.repeat(colWidth)).join(separatorChar) + right;
};

export const UnifiedBoardLayout: React.FC<UnifiedBoardLayoutProps> = ({
  workUnits,
  stashes = [],
  stagedFiles = [],
  unstagedFiles = [],
  focusedColumnIndex = 0,
  selectedWorkUnitIndex = 0,
  selectedWorkUnit = null,
  onColumnChange,
  onWorkUnitChange,
  onEnter,
  onPageUp,
  onPageDown,
  onMoveUp,
  onMoveDown,
  terminalWidth: propTerminalWidth,
  terminalHeight: propTerminalHeight,
}) => {
  // Get terminal dimensions from props (for testing) or Ink hook (for production)
  const { stdout } = useStdout();
  const terminalWidth = propTerminalWidth ?? (stdout?.columns || 80);
  const terminalHeight = propTerminalHeight ?? ((stdout?.rows || 24) - 1);

  // Calculate column width reactively based on terminal width
  const colWidth = useMemo(() => calculateColumnWidth(terminalWidth), [terminalWidth]);

  // Calculate viewport height (BOARD-014: dynamic column height based on terminal height)
  const VIEWPORT_HEIGHT = useMemo(() => calculateViewportHeight(terminalHeight), [terminalHeight]);

  // Shimmer animation state (BOARD-009: character position for wave effect)
  const [shimmerPosition, setShimmerPosition] = useState<number>(0);

  // Scroll offset per column (track scroll position for each column)
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
      // BOARD-016: Use file order directly - NO runtime sorting
      // All sorting happens at write time in work-units.json
      const units = workUnits.filter(wu => wu.status === status);

      const totalPoints = units.reduce((sum, wu) => sum + (wu.estimate || 0), 0);
      return { status, units, count: units.length, totalPoints };
    });
  }, [workUnits]);

  // Compute last changed work unit (BOARD-009: find work unit with most recent updated timestamp)
  const lastChangedWorkUnit = useMemo(() => {
    if (workUnits.length === 0) return null;

    return workUnits.reduce((latest, current) => {
      // Get the most recent stateHistory timestamp for each work unit
      const latestStateTimestamp = latest.stateHistory && latest.stateHistory.length > 0
        ? new Date(latest.stateHistory[latest.stateHistory.length - 1].timestamp).getTime()
        : 0;
      const currentStateTimestamp = current.stateHistory && current.stateHistory.length > 0
        ? new Date(current.stateHistory[current.stateHistory.length - 1].timestamp).getTime()
        : 0;

      return currentStateTimestamp > latestStateTimestamp ? current : latest;
    });
  }, [workUnits]);

  // Shimmer animation effect (BOARD-009: advance character position every 100ms)
  useEffect(() => {
    if (!lastChangedWorkUnit) return;

    const interval = setInterval(() => {
      setShimmerPosition(prev => {
        // Calculate text length for the last changed work unit
        const estimate = lastChangedWorkUnit.estimate || 0;
        const storyPointsText = estimate > 0 ? ` [${estimate}]` : '';
        const text = `${lastChangedWorkUnit.id}${storyPointsText}`;
        const maxPosition = text.length;

        // Loop back to start when reaching the end
        return (prev + 1) % maxPosition;
      });
    }, 100); // 100ms per character

    return () => {
      clearInterval(interval);
    };
  }, [lastChangedWorkUnit]);

  // Automatic scrolling: adjust scroll offset to keep selected item visible (BOARD-012)
  // This implements the navigateTo pattern from cage VirtualList
  useEffect(() => {
    const currentColumn = STATES[focusedColumnIndex];
    const columnUnits = groupedWorkUnits[focusedColumnIndex].units;

    // Skip if no items in column
    if (columnUnits.length === 0) return;

    // Calculate new scroll offset to keep selected item visible
    setScrollOffsets(prev => {
      const currentOffset = prev[currentColumn] || 0;

      // Simple approach: calculate what offset would show the selected item
      // Determine if we'll have arrows at this offset
      const willShowUpArrow = currentOffset > 0;
      const willShowDownArrow = currentOffset + VIEWPORT_HEIGHT < columnUnits.length;
      const arrowsConsumed = (willShowUpArrow ? 1 : 0) + (willShowDownArrow ? 1 : 0);
      const effectiveHeight = VIEWPORT_HEIGHT - arrowsConsumed;

      // Calculate which items are visible with current offset
      const firstVisible = currentOffset + (willShowUpArrow ? 1 : 0);
      const lastVisible = firstVisible + effectiveHeight - 1;

      // Check if selected item is visible
      if (selectedWorkUnitIndex >= firstVisible && selectedWorkUnitIndex <= lastVisible) {
        // Already visible, no scroll needed
        return prev;
      }

      // Need to scroll - calculate new offset
      let newOffset;

      if (selectedWorkUnitIndex < firstVisible) {
        // Scroll up: position selected item near top
        // If we'll have an up arrow, offset = selectedIndex - 1, else offset = selectedIndex
        newOffset = Math.max(0, selectedWorkUnitIndex - 1);

        // But if that would put us at offset 0, don't subtract 1
        if (selectedWorkUnitIndex === 0) {
          newOffset = 0;
        }
      } else {
        // Scroll down: position selected item near bottom
        // Work backwards from selected item
        // We want: offset + (upArrow ? 1 : 0) + effectiveHeight - 1 = selectedWorkUnitIndex
        // So: offset = selectedWorkUnitIndex - effectiveHeight + 1 - (upArrow ? 1 : 0)

        // Assume we'll have both arrows when scrolled down
        const estimatedEffectiveHeight = VIEWPORT_HEIGHT - 2; // Assume both arrows
        newOffset = selectedWorkUnitIndex - estimatedEffectiveHeight + 1;

        // But recalculate to be sure
        const testUpArrow = newOffset > 0;
        const testDownArrow = newOffset + VIEWPORT_HEIGHT < columnUnits.length;
        const testArrows = (testUpArrow ? 1 : 0) + (testDownArrow ? 1 : 0);
        const testEffectiveHeight = VIEWPORT_HEIGHT - testArrows;

        // Adjust if needed
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
  }, [selectedWorkUnitIndex, focusedColumnIndex, groupedWorkUnits]);

  // Handle keyboard input
  useInput((input, key) => {
    // Page Up/Down for scrolling
    if (key.pageDown) {
      const currentColumn = STATES[focusedColumnIndex];
      const currentOffset = scrollOffsets[currentColumn];
      const columnUnits = groupedWorkUnits[focusedColumnIndex].units;

      if (currentOffset + VIEWPORT_HEIGHT < columnUnits.length) {
        setScrollOffsets(prev => ({
          ...prev,
          [currentColumn]: Math.min(currentOffset + VIEWPORT_HEIGHT, columnUnits.length - VIEWPORT_HEIGHT),
        }));
      }
      onPageDown?.();
      return;
    }

    if (key.pageUp) {
      const currentColumn = STATES[focusedColumnIndex];
      const currentOffset = scrollOffsets[currentColumn];

      if (currentOffset > 0) {
        setScrollOffsets(prev => ({
          ...prev,
          [currentColumn]: Math.max(0, currentOffset - VIEWPORT_HEIGHT),
        }));
      }
      onPageUp?.();
      return;
    }

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
  });

  // Build table rows
  const rows: string[] = [];

  // Top border (no columns above - use plain separator)
  rows.push(buildBorderRow(colWidth, '┌', '─', '┐', 'plain'));

  // Git Context panel (combined Git Stashes and Changed Files)
  const totalWidth = colWidth * STATES.length + (STATES.length - 1);
  rows.push('│' + fitToWidth(`Git Stashes (${stashes.length})`, totalWidth) + '│');
  if (stashes.length > 0) {
    // Display first few stashes (truncated to fit)
    const stashDisplay = stashes.slice(0, 3).map(s => {
      // Parse checkpoint message: "fspec-checkpoint:WORK-UNIT-ID:NAME:TIMESTAMP"
      const message = s.commit?.message || s.message || '';
      const parts = message.split(':');
      const name = parts.length >= 3 ? parts[2] : (s.oid?.substring(0, 7) || 'unknown');

      // Calculate relative time
      const timestamp = s.commit?.author?.timestamp || 0;
      const now = Date.now() / 1000;
      const diffSeconds = now - timestamp;
      const diffHours = Math.floor(diffSeconds / 3600);
      const diffDays = Math.floor(diffSeconds / 86400);
      const timeAgo = diffDays > 0 ? `${diffDays} days ago` : `${diffHours} hours ago`;

      return `${name} (${timeAgo})`;
    }).join(', ');
    rows.push('│' + fitToWidth(`  ${stashDisplay}`, totalWidth) + '│');
  } else {
    rows.push('│' + fitToWidth('No stashes', totalWidth) + '│');
  }

  // Changed Files section (no separator - same panel)
  const fileCount = `${stagedFiles.length} staged, ${unstagedFiles.length} unstaged`;
  rows.push('│' + fitToWidth(`Changed Files (${fileCount})`, totalWidth) + '│');
  if (stagedFiles.length === 0 && unstagedFiles.length === 0) {
    rows.push('│' + fitToWidth('  No changes', totalWidth) + '│');
  } else {
    // Display first few changed files (staged first, then unstaged)
    const allFiles = [
      ...stagedFiles.slice(0, 2).map(f => `+ ${f}`),
      ...unstagedFiles.slice(0, 2).map(f => `M ${f}`)
    ];
    const filesDisplay = allFiles.slice(0, 3).map(f => fitToWidth(`  ${f}`, totalWidth / 3)).join(' ');
    rows.push('│' + fitToWidth(filesDisplay, totalWidth) + '│');
  }

  // Separator after Git Context panel
  rows.push(buildBorderRow(colWidth, '├', '─', '┤', 'plain'));

  // Work Unit Details panel (BOARD-014: static 4 lines high)
  rows.push('│' + fitToWidth('Work Unit Details', totalWidth) + '│');

  // Always output exactly 4 detail lines (static height)
  const detailLines: string[] = [];

  if (selectedWorkUnit) {
    // Line 1: Title with ID
    const titleLine = `${selectedWorkUnit.id}: ${selectedWorkUnit.title}`;
    detailLines.push(fitToWidth(`  ${titleLine}`, totalWidth));

    // Line 2: First line of description (if exists)
    if (selectedWorkUnit.description && selectedWorkUnit.description.trim().length > 0) {
      const descLines = selectedWorkUnit.description.split('\n');
      const firstDescLine = descLines[0].trim();
      detailLines.push(fitToWidth(`  ${firstDescLine}`, totalWidth));
    } else {
      detailLines.push(fitToWidth('', totalWidth));
    }

    // Line 3: Metadata (Epic, Estimate, Status)
    const metadata: string[] = [];
    if (selectedWorkUnit.epic) {
      metadata.push(`Epic: ${selectedWorkUnit.epic}`);
    }
    if (selectedWorkUnit.estimate !== undefined) {
      metadata.push(`Estimate: ${selectedWorkUnit.estimate}pts`);
    }
    if (selectedWorkUnit.status) {
      metadata.push(`Status: ${selectedWorkUnit.status}`);
    }
    detailLines.push(fitToWidth(`  ${metadata.join(' | ')}`, totalWidth));

    // Line 4: Empty line for spacing
    detailLines.push(fitToWidth('', totalWidth));
  } else {
    // No work unit selected - fill with 4 empty lines
    for (let i = 0; i < 4; i++) {
      detailLines.push(i === 0 ? centerText('No work unit selected', totalWidth) : fitToWidth('', totalWidth));
    }
  }

  // Safety check: ensure we have exactly 4 lines
  while (detailLines.length < 4) {
    detailLines.push(fitToWidth('', totalWidth));
  }

  // Output the 4 detail lines
  for (let i = 0; i < 4; i++) {
    rows.push('│' + detailLines[i] + '│');
  }

  // Separator after Changed Files (top - no columns above, columns start below)
  rows.push(buildBorderRow(colWidth, '├', '┬', '┤', 'top'));

  // Column headers (with focus highlighting using chalk)
  rows.push('│' + STATES.map((state, idx) => {
    const header = state.toUpperCase();
    const paddedHeader = fitToWidth(header, colWidth);
    // Highlight focused column in cyan
    return idx === focusedColumnIndex ? chalk.cyan(paddedHeader) : chalk.gray(paddedHeader);
  }).join('│') + '│');

  // Header separator
  rows.push(buildBorderRow(colWidth, '├', '┼', '┤'));

  // Data rows (with scrolling support)
  const maxRows = VIEWPORT_HEIGHT;
  for (let rowIndex = 0; rowIndex < maxRows; rowIndex++) {
    const cells = STATES.map((state, colIndex) => {
      const column = groupedWorkUnits[colIndex];
      const scrollOffset = scrollOffsets[state];
      const itemIndex = scrollOffset + rowIndex;

      // Show scroll indicators ONLY if there are items to scroll to
      if (rowIndex === 0 && scrollOffset > 0 && column.units.length > 0) {
        return fitToWidth('↑', colWidth); // Up arrow at top when scrolled down
      }
      if (rowIndex === maxRows - 1 && scrollOffset + maxRows < column.units.length) {
        return fitToWidth('↓', colWidth); // Down arrow at bottom when more items below
      }

      if (itemIndex >= column.units.length) {
        return fitToWidth('', colWidth);
      }

      const wu = column.units[itemIndex];
      const estimate = wu.estimate || 0;

      // Build text without emoji icons (BOARD-008)
      // Only show story points if > 0, format as [N]
      const storyPointsText = estimate > 0 ? ` [${estimate}]` : '';
      const text = `${wu.id}${storyPointsText}`;
      const paddedText = fitToWidth(text, colWidth);

      // Check if this work unit is selected
      const isSelected = colIndex === focusedColumnIndex && itemIndex === selectedWorkUnitIndex;

      // Check if this work unit is the last changed (BOARD-009)
      const isLastChanged = lastChangedWorkUnit?.id === wu.id;

      // Apply color-coding (BOARD-009 character-by-character shimmer):
      // - Selected + last-changed: character shimmer on background (bgGreen with moving bgGreenBright)
      // - Selected only: green background (no shimmer)
      // - Last-changed only: character shimmer on text color based on type
      // - Normal: type-based color (white/red/blue, no shimmer)
      if (isSelected && isLastChanged) {
        // Selected AND last-changed: character-by-character background shimmer
        return applyBackgroundCharacterShimmer(paddedText, shimmerPosition);
      } else if (isSelected) {
        // Selected only: green background (no shimmer)
        return chalk.bgGreen.black(paddedText);
      } else if (isLastChanged) {
        // Last-changed only: character-by-character text shimmer based on type
        return applyCharacterShimmer(paddedText, shimmerPosition, wu.type);
      } else {
        // Normal: type-based color (no shimmer)
        if (wu.type === 'bug') {
          return chalk.red(paddedText);
        } else if (wu.type === 'task') {
          return chalk.blue(paddedText);
        } else {
          // story type defaults to white
          return chalk.white(paddedText);
        }
      }
    });
    rows.push('│' + cells.join('│') + '│');
  }

  // Footer separator (bottom - columns end above, no columns below)
  rows.push(buildBorderRow(colWidth, '├', '┴', '┤', 'bottom'));

  // Footer row (centered with diamond separators)
  rows.push('│' + centerText('← → Columns ◆ ↑↓ Work Units ◆ [ Priority Up ◆ ] Priority Down ◆ ↵ Details ◆ ESC Back', totalWidth) + '│');

  // Bottom border (no columns below - use plain separator)
  rows.push(buildBorderRow(colWidth, '└', '─', '┘', 'plain'));

  // Split rows into sections for hybrid rendering
  const separatorAfterGit = rows[5]; // Separator after header
  const restOfRows = rows.slice(6); // Everything else (work details, columns, footer)

  return (
    <Box flexDirection="column" width={terminalWidth}>
      {/* Header section: Box with borderStyle, master container inside with Logo + Git panels */}
      <Box borderStyle="single" paddingX={1}>
        <Box flexDirection="row">
          <Logo />
          <Box flexGrow={1} flexDirection="column" height={4}>
            <GitStashesPanel stashes={stashes} />
            <ChangedFilesPanel stagedFiles={stagedFiles} unstagedFiles={unstagedFiles} />
          </Box>
        </Box>
      </Box>

      {/* Separator after git panels */}
      <Text>{separatorAfterGit}</Text>

      {/* Rest of the board */}
      {restOfRows.map((row, idx) => {
        const isHeader = idx === 2; // Column header row (adjusted index)
        return (
          <Text
            key={idx + 6}
            bold={isHeader}
            color={isHeader ? 'cyan' : undefined}
          >
            {row}
          </Text>
        );
      })}
    </Box>
  );
};
