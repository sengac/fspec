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

interface WorkUnit {
  id: string;
  title: string;
  type: 'story' | 'task' | 'bug';
  estimate?: number;
  status: string;
}

interface UnifiedBoardLayoutProps {
  workUnits: WorkUnit[];
  stashes?: any[];
  stagedFiles?: string[];
  unstagedFiles?: string[];
  focusedColumnIndex?: number;
  selectedWorkUnitIndex?: number;
  onColumnChange?: (delta: number) => void;
  onWorkUnitChange?: (delta: number) => void;
  onEnter?: () => void;
  onPageUp?: () => void;
  onPageDown?: () => void;
}

const STATES = ['backlog', 'specifying', 'testing', 'implementing', 'validating', 'done', 'blocked'] as const;
const VIEWPORT_HEIGHT = 10; // Number of items visible at once

// Helper: Calculate optimal column width (same pattern as BoardDisplay)
const calculateColumnWidth = (terminalWidth: number): number => {
  const borders = 2; // Left and right outer borders
  const separators = STATES.length - 1; // Column separators (6 for 7 columns)
  const availableWidth = terminalWidth - borders - separators;
  const calculatedWidth = Math.floor(availableWidth / STATES.length);

  // Return calculated width (will adapt to terminal size)
  return Math.max(8, calculatedWidth); // Absolute minimum of 8 chars (same as BoardDisplay)
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
  onColumnChange,
  onWorkUnitChange,
  onEnter,
  onPageUp,
  onPageDown,
}) => {
  // Get terminal dimensions directly from Ink (same pattern as BoardDisplay)
  const { stdout } = useStdout();
  const terminalWidth = stdout?.columns || 80; // Default to 80 for compatibility

  // Calculate column width reactively based on terminal width
  const colWidth = useMemo(() => calculateColumnWidth(terminalWidth), [terminalWidth]);

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
      const units = workUnits.filter(wu => wu.status === status);
      const totalPoints = units.reduce((sum, wu) => sum + (wu.estimate || 0), 0);
      return { status, units, count: units.length, totalPoints };
    });
  }, [workUnits]);

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
  });

  // Build table rows
  const rows: string[] = [];

  // Top border (no columns above - use plain separator)
  rows.push(buildBorderRow(colWidth, '┌', '─', '┐', 'plain'));

  // Git Stashes panel (integrated as table row)
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

  // Separator after Git Stashes (plain - no columns above or below)
  rows.push(buildBorderRow(colWidth, '├', '─', '┤', 'plain'));

  // Changed Files panel (integrated as table row)
  const fileCount = `${stagedFiles.length} staged, ${unstagedFiles.length} unstaged`;
  rows.push('│' + fitToWidth(`Changed Files (${fileCount})`, totalWidth) + '│');
  if (stagedFiles.length === 0 && unstagedFiles.length === 0) {
    rows.push('│' + fitToWidth('No changes', totalWidth) + '│');
  } else {
    // Display first few changed files (staged first, then unstaged)
    const allFiles = [
      ...stagedFiles.slice(0, 2).map(f => `+ ${f}`),
      ...unstagedFiles.slice(0, 2).map(f => `M ${f}`)
    ];
    const filesDisplay = allFiles.slice(0, 3).map(f => fitToWidth(`  ${f}`, totalWidth / 3)).join(' ');
    rows.push('│' + fitToWidth(filesDisplay, totalWidth) + '│');
  }

  // Separator after Changed Files (top - no columns above, columns start below)
  rows.push(buildBorderRow(colWidth, '├', '┬', '┤', 'top'));

  // Column headers (with focus highlighting using chalk)
  rows.push('│' + STATES.map((state, idx) => {
    const column = groupedWorkUnits[idx];
    const header = `${state.toUpperCase()} (${column.count}) - ${column.totalPoints}pts`;
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
        return fitToWidth(itemIndex === 0 ? 'No work units' : '', colWidth);
      }

      const wu = column.units[itemIndex];
      const typeIcon = wu.type === 'bug' ? '🐛' : wu.type === 'task' ? '⚙️' : '📖';
      const estimate = wu.estimate || 0;
      const priorityIcon = estimate > 8 ? '🔴' : estimate >= 3 ? '🟡' : '🟢';
      // Keep full "pt" text and let fitToWidth handle truncation
      const text = `${typeIcon} ${wu.id} ${estimate}pt ${priorityIcon}`;
      const paddedText = fitToWidth(text, colWidth);

      // Highlight selected work unit in focused column with cyan background
      const isSelected = colIndex === focusedColumnIndex && itemIndex === selectedWorkUnitIndex;
      return isSelected ? chalk.bgCyan.black(paddedText) : paddedText;
    });
    rows.push('│' + cells.join('│') + '│');
  }

  // Footer separator (bottom - columns end above, no columns below)
  rows.push(buildBorderRow(colWidth, '├', '┴', '┤', 'bottom'));

  // Footer row (centered)
  rows.push('│' + centerText('← → Columns | ↑↓ jk Work Units | ↵ Details | ESC Back', totalWidth) + '│');

  // Bottom border (no columns below - use plain separator)
  rows.push(buildBorderRow(colWidth, '└', '─', '┘', 'plain'));

  return (
    <Box flexDirection="column" width={terminalWidth}>
      <Text bold>fspec Kanban Board</Text>
      {rows.map((row, idx) => {
        const isHeader = idx === 8; // Column header row
        return (
          <Text
            key={idx}
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
