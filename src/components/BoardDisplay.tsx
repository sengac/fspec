import React, { useMemo } from 'react';
import { Box, Text, useStdout } from 'ink';

interface BoardColumn {
  id: string;
  title: string;
  estimate?: number;
}

interface BoardDisplayProps {
  columns?: Record<string, BoardColumn[]>;
  board: Record<string, string[]>;
  summary: string;
  limit?: number;
}

const STATES = ['backlog', 'specifying', 'testing', 'implementing', 'validating', 'done', 'blocked'] as const;

// Helper: Calculate optimal column width based on terminal size
const calculateColumnWidth = (terminalWidth: number): number => {
  const borders = 2; // Left and right outer borders
  const separators = STATES.length - 1; // Column separators
  const availableWidth = terminalWidth - borders - separators;
  const calculatedWidth = Math.floor(availableWidth / STATES.length);

  // Return calculated width (will adapt to terminal size)
  return Math.max(8, calculatedWidth); // Absolute minimum of 8 chars
};

// Helper: Pad or truncate text to fit column width
const fitToWidth = (text: string, width: number): string => {
  if (text.length > width) {
    return text.substring(0, width);
  }
  return text.padEnd(width, ' ');
};

// Helper: Build border row with proper junction characters
const buildBorderRow = (colWidth: number, junctionLeft: string, junctionMid: string, junctionRight: string): string => {
  return junctionLeft + STATES.map(() => '─'.repeat(colWidth)).join(junctionMid) + junctionRight;
};

// Helper: Format cell content with estimate
const formatCellContent = (item: BoardColumn | undefined, overflowCount: number, colWidth: number): string => {
  if (overflowCount > 0) {
    return fitToWidth(`... ${overflowCount} more`, colWidth);
  }
  if (!item) {
    return fitToWidth('', colWidth);
  }
  const pts = item.estimate ? ` [${item.estimate}]` : '';
  return fitToWidth(`${item.id}${pts}`, colWidth);
};

export const BoardDisplay: React.FC<BoardDisplayProps> = ({ columns, board, summary, limit = 3 }) => {
  // Get terminal dimensions from Ink's useStdout hook
  const { stdout } = useStdout();
  const terminalWidth = stdout?.columns || 80; // Default to 80 for compatibility

  // Calculate column width reactively based on terminal size
  const colWidth = useMemo(() => calculateColumnWidth(terminalWidth), [terminalWidth]);

  // Calculate max rows needed
  const maxRows = useMemo(() => Math.max(
    ...STATES.map(state => {
      const items = columns?.[state] || [];
      return Math.min(items.length, limit) + (items.length > limit ? 1 : 0);
    }),
    1
  ), [columns, limit]);

  // Build all table rows - memoized to avoid recalculation
  const rows = useMemo(() => {
    const tableRows: string[] = [];

    // Top border
    tableRows.push(buildBorderRow(colWidth, '┌', '┬', '┐'));

    // Header row
    tableRows.push('│' + STATES.map(state => fitToWidth(state.toUpperCase(), colWidth)).join('│') + '│');

    // Header separator
    tableRows.push(buildBorderRow(colWidth, '├', '┼', '┤'));

    // Data rows
    for (let rowIndex = 0; rowIndex < maxRows; rowIndex++) {
      const cells = STATES.map(state => {
        const items = columns?.[state] || [];
        const overflowCount = items.length > limit && rowIndex === limit ? items.length - limit : 0;
        const item = rowIndex < Math.min(items.length, limit) ? items[rowIndex] : undefined;
        return formatCellContent(item, overflowCount, colWidth);
      });
      tableRows.push('│' + cells.join('│') + '│');
    }

    // Summary separator
    const totalWidth = colWidth * STATES.length + (STATES.length - 1);
    tableRows.push('├' + '─'.repeat(totalWidth) + '┤');

    // Summary row
    tableRows.push('│' + fitToWidth(summary, totalWidth) + '│');

    // Bottom border
    tableRows.push('└' + '─'.repeat(totalWidth) + '┘');

    return tableRows;
  }, [colWidth, maxRows, columns, summary, limit]);

  // Render with appropriate styling
  return (
    <Box flexDirection="column">
      {rows.map((row, idx) => {
        const isHeader = idx === 1;
        const isSummary = idx === rows.length - 2;

        return (
          <Text
            key={idx}
            bold={isHeader || isSummary}
            color={isHeader ? 'cyan' : undefined}
          >
            {row}
          </Text>
        );
      })}
    </Box>
  );
};
