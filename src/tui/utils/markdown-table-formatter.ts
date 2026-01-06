/**
 * Markdown Table Formatter
 *
 * TUI-044: Formats markdown tables in AI output with proper column alignment
 * and box-drawing characters. Only processes tables, leaving other content unchanged.
 *
 * Uses the marked library to parse markdown and identify table tokens.
 * Also handles tables inside code fences by detecting table-like patterns.
 */

import { marked, type Token, type Tokens } from 'marked';
import chalk from 'chalk';
import { getVisualWidth } from './stringWidth';

/**
 * Render inline tokens (text, strong, em, etc.) to plain text with ANSI styling
 */
function renderInlineTokens(tokens: Token[] | undefined): string {
  if (!tokens) {
    return '';
  }

  return tokens
    .map(token => {
      switch (token.type) {
        case 'text':
          return (token as Tokens.Text).text;
        case 'strong':
          return chalk.bold(
            renderInlineTokens((token as Tokens.Strong).tokens)
          );
        case 'em':
          return chalk.italic(renderInlineTokens((token as Tokens.Em).tokens));
        case 'codespan':
          return (token as Tokens.Codespan).text;
        default:
          if ('text' in token) {
            return (token as { text: string }).text;
          }
          if ('raw' in token) {
            return (token as { raw: string }).raw;
          }
          return '';
      }
    })
    .join('');
}

/**
 * Get the visual width of a string in terminal columns.
 * Accounts for ANSI codes and Unicode characters (emojis, CJK, etc.)
 */
function getTextWidth(text: string): number {
  // Strip ANSI escape codes first, then get proper Unicode visual width
  // eslint-disable-next-line no-control-regex
  const stripped = text.replace(/\x1b\[[0-9;]*m/g, '');
  return getVisualWidth(stripped);
}

/**
 * Pad text to target width respecting alignment
 */
function padText(
  text: string,
  width: number,
  alignment: 'left' | 'center' | 'right' | null
): string {
  const textWidth = getTextWidth(text);
  const padding = width - textWidth;

  if (padding <= 0) {
    return text;
  }

  switch (alignment) {
    case 'right':
      return ' '.repeat(padding) + text;
    case 'center': {
      const leftPad = Math.floor(padding / 2);
      const rightPad = padding - leftPad;
      return ' '.repeat(leftPad) + text + ' '.repeat(rightPad);
    }
    case 'left':
    default:
      return text + ' '.repeat(padding);
  }
}

/**
 * Parse alignment from separator row (e.g., |:---|:---:|---:|)
 */
function parseAlignment(
  separatorCell: string
): 'left' | 'center' | 'right' | null {
  const trimmed = separatorCell.trim();
  const hasLeftColon = trimmed.startsWith(':');
  const hasRightColon = trimmed.endsWith(':');

  if (hasLeftColon && hasRightColon) {
    return 'center';
  } else if (hasRightColon) {
    return 'right';
  } else if (hasLeftColon) {
    return 'left';
  }
  return null; // default left
}

/**
 * Check if a string looks like a markdown table
 */
function looksLikeTable(text: string): boolean {
  const lines = text.trim().split('\n');
  if (lines.length < 2) {
    return false;
  }

  // Check if first line has pipes
  if (!lines[0].includes('|')) {
    return false;
  }

  // Check if second line looks like a separator (contains dashes and pipes)
  const secondLine = lines[1].trim();
  if (!secondLine.includes('|') || !secondLine.includes('-')) {
    return false;
  }

  // Verify separator line pattern: |---|---|---| or |:---|:---:|---:|
  const separatorPattern = /^\|?[\s:]*-+[\s:]*(\|[\s:]*-+[\s:]*)*\|?$/;
  if (!separatorPattern.test(secondLine)) {
    return false;
  }

  return true;
}

/**
 * Parse a raw table string into header, rows, and alignment
 */
interface ParsedTable {
  header: string[];
  rows: string[][];
  align: ('left' | 'center' | 'right' | null)[];
}

function parseRawTable(text: string): ParsedTable | null {
  const lines = text
    .trim()
    .split('\n')
    .filter(l => l.trim());
  if (lines.length < 2) {
    return null;
  }

  // Parse header row
  const headerLine = lines[0];
  const headerCells = headerLine
    .split('|')
    .map(c => c.trim())
    .filter((c, i, arr) => {
      // Filter out empty strings at start/end from leading/trailing pipes
      if (i === 0 && c === '') {
        return false;
      }
      if (i === arr.length - 1 && c === '') {
        return false;
      }
      return true;
    });

  if (headerCells.length === 0) {
    return null;
  }

  // Parse separator row for alignment
  const separatorLine = lines[1];
  const separatorCells = separatorLine
    .split('|')
    .map(c => c.trim())
    .filter((c, i, arr) => {
      if (i === 0 && c === '') {
        return false;
      }
      if (i === arr.length - 1 && c === '') {
        return false;
      }
      return true;
    });

  const align = separatorCells.map(parseAlignment);

  // Ensure align array matches header length
  while (align.length < headerCells.length) {
    align.push(null);
  }

  // Parse data rows
  const rows: string[][] = [];
  for (let i = 2; i < lines.length; i++) {
    const rowLine = lines[i];
    const rowCells = rowLine
      .split('|')
      .map(c => c.trim())
      .filter((c, idx, arr) => {
        if (idx === 0 && c === '') {
          return false;
        }
        if (idx === arr.length - 1 && c === '') {
          return false;
        }
        return true;
      });

    // Pad row to match header length
    while (rowCells.length < headerCells.length) {
      rowCells.push('');
    }

    rows.push(rowCells);
  }

  return { header: headerCells, rows, align };
}

/**
 * Render a parsed table to a formatted string with box-drawing characters
 */
function renderParsedTable(table: ParsedTable): string {
  const { header, rows, align } = table;

  // Calculate column widths based on max content width
  const colWidths: number[] = header.map((headerText, colIdx) => {
    let maxWidth = getTextWidth(headerText);

    for (const row of rows) {
      if (row[colIdx]) {
        maxWidth = Math.max(maxWidth, getTextWidth(row[colIdx]));
      }
    }

    return maxWidth;
  });

  // Build the table
  const lines: string[] = [];

  // Top border: ┌─────┬─────┐
  const topBorder = '┌─' + colWidths.map(w => '─'.repeat(w)).join('─┬─') + '─┐';
  lines.push(topBorder);

  // Header row (bold)
  const headerCells = header.map((text, i) => {
    const padded = padText(text, colWidths[i], align[i]);
    return chalk.bold(padded);
  });
  lines.push('│ ' + headerCells.join(' │ ') + ' │');

  // Header separator: ├─────┼─────┤
  const separator = '├─' + colWidths.map(w => '─'.repeat(w)).join('─┼─') + '─┤';
  lines.push(separator);

  // Data rows
  for (const row of rows) {
    const cells = row.map((text, i) => {
      return padText(text, colWidths[i], align[i]);
    });
    lines.push('│ ' + cells.join(' │ ') + ' │');
  }

  // Bottom border: └─────┴─────┘
  const bottomBorder =
    '└─' + colWidths.map(w => '─'.repeat(w)).join('─┴─') + '─┘';
  lines.push(bottomBorder);

  // Add trailing newline so content after the table starts on a new line
  return lines.join('\n') + '\n';
}

/**
 * Render a single table token to a formatted string with box-drawing characters
 */
function renderTable(table: Tokens.Table): string {
  const { header, rows, align } = table;

  // Calculate column widths based on max content width
  const colWidths: number[] = header.map((cell, colIdx) => {
    const headerText = renderInlineTokens(cell.tokens);
    let maxWidth = getTextWidth(headerText);

    for (const row of rows) {
      if (row[colIdx]) {
        const cellText = renderInlineTokens(row[colIdx].tokens);
        maxWidth = Math.max(maxWidth, getTextWidth(cellText));
      }
    }

    return maxWidth;
  });

  // Build the table
  const lines: string[] = [];

  // Top border: ┌─────┬─────┐
  const topBorder = '┌─' + colWidths.map(w => '─'.repeat(w)).join('─┬─') + '─┐';
  lines.push(topBorder);

  // Header row (bold)
  const headerCells = header.map((cell, i) => {
    const text = renderInlineTokens(cell.tokens);
    const padded = padText(text, colWidths[i], align[i]);
    return chalk.bold(padded);
  });
  lines.push('│ ' + headerCells.join(' │ ') + ' │');

  // Header separator: ├─────┼─────┤
  const separator = '├─' + colWidths.map(w => '─'.repeat(w)).join('─┼─') + '─┤';
  lines.push(separator);

  // Data rows
  for (const row of rows) {
    const cells = row.map((cell, i) => {
      const text = renderInlineTokens(cell.tokens);
      return padText(text, colWidths[i], align[i]);
    });
    lines.push('│ ' + cells.join(' │ ') + ' │');
  }

  // Bottom border: └─────┴─────┘
  const bottomBorder =
    '└─' + colWidths.map(w => '─'.repeat(w)).join('─┴─') + '─┘';
  lines.push(bottomBorder);

  // Add trailing newline so content after the table starts on a new line
  return lines.join('\n') + '\n';
}

/**
 * Format markdown tables in the content.
 *
 * Parses the content with marked, identifies table tokens, and replaces
 * the raw markdown table syntax with properly formatted box-drawing tables.
 * Also handles tables inside code fences by detecting table patterns.
 * Non-table content is preserved unchanged.
 *
 * @param content - Raw content that may contain markdown tables
 * @returns Content with tables formatted using box-drawing characters
 */
export function formatMarkdownTables(content: string): string {
  if (!content || typeof content !== 'string') {
    return content || '';
  }

  // Parse the content to find tables and code blocks
  const tokens = marked.lexer(content);

  // Check if there are any tables or code blocks that might contain tables
  const hasTables = tokens.some(t => t.type === 'table');
  const hasCodeBlocks = tokens.some(t => t.type === 'code');

  if (!hasTables && !hasCodeBlocks) {
    return content;
  }

  // Process tokens and rebuild content
  const parts: string[] = [];
  let lastEnd = 0;

  for (const token of tokens) {
    if (token.type === 'table') {
      const tableToken = token as Tokens.Table;
      // Find the raw table in the original content
      const rawTable = tableToken.raw;
      const startIdx = content.indexOf(rawTable, lastEnd);

      if (startIdx >= 0) {
        // Add content before this table
        if (startIdx > lastEnd) {
          parts.push(content.slice(lastEnd, startIdx));
        }

        // Add formatted table
        parts.push(renderTable(tableToken));

        lastEnd = startIdx + rawTable.length;
      }
    } else if (token.type === 'code') {
      const codeToken = token as Tokens.Code;
      const codeContent = codeToken.text;

      // Check if code block contains a table
      if (looksLikeTable(codeContent)) {
        const parsedTable = parseRawTable(codeContent);

        if (parsedTable) {
          // Find the raw code block in the original content
          const rawCode = codeToken.raw;
          const startIdx = content.indexOf(rawCode, lastEnd);

          if (startIdx >= 0) {
            // Add content before this code block
            if (startIdx > lastEnd) {
              parts.push(content.slice(lastEnd, startIdx));
            }

            // Add formatted table (replacing the entire code block)
            parts.push(renderParsedTable(parsedTable));

            lastEnd = startIdx + rawCode.length;
          }
        }
      }
    }
  }

  // Add remaining content after last table/code block
  if (lastEnd < content.length) {
    parts.push(content.slice(lastEnd));
  }

  // If no parts were added, return original content
  if (parts.length === 0) {
    return content;
  }

  return parts.join('');
}
