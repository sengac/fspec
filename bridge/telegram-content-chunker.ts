/**
 * Telegram Content-Aware Chunker
 *
 * BRIDGE-006: Intelligent Content-Aware Chunking for Telegram Display
 *
 * Implements boundary detection, content summarization, and markdown validation
 * for streaming content to Telegram.
 */

// ===========================================
// TYPES
// ===========================================

export interface ChunkBoundary {
  type:
    | 'sentence'
    | 'paragraph'
    | 'heading'
    | 'code_block'
    | 'list'
    | 'max_size';
  position: number;
  priority: number; // code_block=5, heading=4, paragraph=3, sentence=2, max_size=1
}

export interface FlushResult {
  messages: string[];
  remaining: string;
}

// ===========================================
// CONSTANTS
// ===========================================

const TELEGRAM_MAX_LENGTH = 4096;
const BOUNDARY_PRIORITIES = {
  code_block: 5,
  heading: 4,
  paragraph: 3,
  sentence: 2,
  list: 2,
  max_size: 1,
};

// ===========================================
// BOUNDARY DETECTION
// ===========================================

/**
 * Find the best sentence boundary before maxPosition.
 * Returns position after sentence-ending punctuation.
 */
export function findSentenceBoundary(
  text: string,
  maxPosition: number
): number {
  // Look for sentence endings: . ! ? followed by space or newline
  const sentenceEndPattern = /[.!?]/g;
  let lastMatch = -1;
  let match: RegExpExecArray | null;

  while ((match = sentenceEndPattern.exec(text)) !== null) {
    // Position after the punctuation
    const endPos = match.index + 1;
    if (endPos <= maxPosition) {
      // Check if followed by whitespace (indicating end of sentence)
      const nextChar = text[endPos];
      if (nextChar === undefined || /\s/.test(nextChar)) {
        lastMatch = endPos;
      }
    } else {
      break;
    }
  }

  // If we found a sentence boundary, return it
  if (lastMatch > 0) {
    return lastMatch;
  }

  // Check for sentence at exact end of text
  if (text.length <= maxPosition && /[.!?]$/.test(text)) {
    return text.length;
  }

  // No sentence boundary found, return maxPosition
  return maxPosition;
}

/**
 * Find paragraph boundary (double newline).
 */
export function findParagraphBoundary(
  text: string,
  maxPosition: number
): number {
  // Look for paragraph breaks (double newline)
  const paragraphPattern = /\n\n/g;
  let lastMatch = -1;
  let match: RegExpExecArray | null;

  while ((match = paragraphPattern.exec(text)) !== null) {
    // Position after both newlines (split point - cleaner for next message)
    const endPos = match.index + 2;
    if (endPos <= maxPosition) {
      lastMatch = endPos;
    } else {
      break;
    }
  }

  if (lastMatch > 0) {
    return lastMatch;
  }

  return maxPosition;
}

/**
 * Find heading boundary (# ## ###).
 * Returns position before the heading.
 */
export function findHeadingBoundary(text: string, maxPosition: number): number {
  // Look for headings preceded by newlines
  const headingPattern = /\n(#{1,6}\s)/g;
  let lastMatch = -1;
  let match: RegExpExecArray | null;

  while ((match = headingPattern.exec(text)) !== null) {
    // Position at the newline before heading
    if (match.index < maxPosition) {
      lastMatch = match.index + 1; // After the newline, before the #
    } else {
      break;
    }
  }

  if (lastMatch > 0) {
    return lastMatch;
  }

  return maxPosition;
}

/**
 * Find code block boundary (```...```).
 * Returns position after complete code block.
 */
export function findCodeBlockBoundary(
  text: string,
  maxPosition: number
): number {
  // Find opening and closing ``` pairs
  const codeBlockPattern = /```[\s\S]*?```/g;
  let match: RegExpExecArray | null;

  while ((match = codeBlockPattern.exec(text)) !== null) {
    const endPos = match.index + match[0].length;
    if (endPos <= maxPosition) {
      // If the entire code block fits, we can split after it
      return endPos;
    } else if (match.index < maxPosition) {
      // Code block starts before maxPosition but extends beyond
      // We should split BEFORE the code block starts
      return match.index > 0 ? match.index : maxPosition;
    }
  }

  return maxPosition;
}

/**
 * Find list boundary (consecutive - or * or numbered lines).
 * Returns position after complete list.
 */
export function findListBoundary(text: string, maxPosition: number): number {
  // Look for list patterns
  const listItemPattern = /^[\s]*[-*+]|\d+\./m;
  const lines = text.split('\n');
  let inList = false;
  let listEndPos = 0;
  let currentPos = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const isLastLine = i === lines.length - 1;
    const lineEnd = currentPos + line.length + (isLastLine ? 0 : 1); // +1 for \n except last line

    if (listItemPattern.test(line)) {
      inList = true;
      if (lineEnd <= maxPosition) {
        listEndPos = lineEnd;
      }
    } else if (inList && line.trim() === '') {
      // Empty line might end the list
      if (lineEnd <= maxPosition) {
        listEndPos = lineEnd;
      }
      inList = false;
    } else if (inList) {
      // Non-list line ends the list
      inList = false;
    }

    currentPos = lineEnd + (isLastLine ? 0 : 1); // Move past \n
  }

  // If we ended while still in a list, the list goes to the end
  if (inList && text.length <= maxPosition) {
    return text.length;
  }

  return listEndPos > 0 ? listEndPos : maxPosition;
}

/**
 * Check if position is inside a code block.
 */
export function isInsideCodeBlock(text: string, position: number): boolean {
  const textBeforePosition = text.slice(0, position);
  const openings = (textBeforePosition.match(/```/g) || []).length;
  // Odd number of ``` markers means we're inside a code block
  return openings % 2 === 1;
}

/**
 * Check if a line is a table row (starts with |).
 */
function isTableRow(line: string): boolean {
  return /^\s*\|/.test(line);
}

/**
 * Check if a line is a table separator (|---|).
 */
function isTableSeparator(line: string): boolean {
  return /^\s*\|[\s\-:]+\|/.test(line);
}

/**
 * Check if position is inside a markdown table.
 */
export function isInsideTable(text: string, position: number): boolean {
  const textBeforePosition = text.slice(0, position);
  const lines = textBeforePosition.split('\n');

  // Work backwards to find if we're in a table
  let inTable = false;
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    if (isTableRow(line) || isTableSeparator(line)) {
      inTable = true;
    } else if (inTable && line.trim() !== '') {
      // Non-table, non-empty line before table rows means table hasn't started yet
      break;
    } else if (!inTable && line.trim() === '') {
      // Empty line and not in table - keep looking
      continue;
    } else if (!inTable) {
      // Non-table line and not in table - we're not in a table
      break;
    }
  }

  return inTable;
}

/**
 * Find table boundary (complete markdown table).
 * Returns position after complete table.
 */
export function findTableBoundary(text: string, maxPosition: number): number {
  const lines = text.split('\n');
  let tableStart = -1;
  let tableEnd = -1;
  let currentPos = 0;
  let inTable = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const isLastLine = i === lines.length - 1;
    const lineStart = currentPos;
    const lineEnd = currentPos + line.length + (isLastLine ? 0 : 1);

    if (isTableRow(line) || isTableSeparator(line)) {
      if (!inTable) {
        tableStart = lineStart;
        inTable = true;
      }
      if (lineEnd <= maxPosition) {
        tableEnd = lineEnd;
      }
    } else if (inTable) {
      // Non-table line ends the table
      if (line.trim() === '') {
        // Empty line after table - include it
        if (lineEnd <= maxPosition) {
          tableEnd = lineEnd;
        }
      }
      inTable = false;
    }

    currentPos = lineEnd + (isLastLine ? 0 : 1);
  }

  // If we're still in a table at the end
  if (inTable && text.length <= maxPosition) {
    return text.length;
  }

  // If table ends before maxPosition, return table end
  if (tableEnd > 0 && tableEnd <= maxPosition) {
    return tableEnd;
  }

  // If we're inside a table at maxPosition, return position before table
  if (tableStart > 0 && tableStart < maxPosition) {
    return tableStart;
  }

  return maxPosition;
}

/**
 * Find the best split point before maxPosition.
 * Uses priority: code_block=table > heading > paragraph > sentence > max_size
 */
export function getBestSplitPoint(text: string, maxPosition: number): number {
  // If text fits within limit, return full length
  if (text.length <= maxPosition) {
    return text.length;
  }

  // Reserve space for potential markdown closing markers
  // Worst case: closing code fence (4 chars: \n```)
  const safeMaxPosition = maxPosition - 5;

  // Check if we're inside a code block at maxPosition
  if (isInsideCodeBlock(text, safeMaxPosition)) {
    // Find the start of the code block and split before it
    const textBefore = text.slice(0, safeMaxPosition);
    const lastCodeBlockStart = textBefore.lastIndexOf('```');
    if (lastCodeBlockStart > 0) {
      // Look for a boundary before the code block
      const boundaryBefore = findParagraphBoundary(text, lastCodeBlockStart);
      if (boundaryBefore < lastCodeBlockStart && boundaryBefore > 0) {
        return boundaryBefore;
      }
      return lastCodeBlockStart;
    }
    // Code block starts at beginning - just truncate with room for closing
    return safeMaxPosition;
  }

  // Check if we're inside a table at maxPosition (same priority as code block)
  if (isInsideTable(text, safeMaxPosition)) {
    // Find the start of the table and split before it
    const lines = text.slice(0, safeMaxPosition).split('\n');
    let tableStartPos = safeMaxPosition;
    let currentPos = 0;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineStart = currentPos;
      currentPos += line.length + 1;

      if (/^\s*\|/.test(line)) {
        // First table row - find position before it
        tableStartPos = lineStart;
        break;
      }
    }

    if (tableStartPos > 0) {
      // Look for a boundary before the table
      const boundaryBefore = findParagraphBoundary(text, tableStartPos);
      if (boundaryBefore < tableStartPos && boundaryBefore > 0) {
        return boundaryBefore;
      }
      return tableStartPos;
    }
  }

  // Try boundaries in priority order
  const boundaries: ChunkBoundary[] = [];

  // Check for paragraph boundary
  const paragraphPos = findParagraphBoundary(text, safeMaxPosition);
  if (paragraphPos < safeMaxPosition && paragraphPos > 0) {
    boundaries.push({
      type: 'paragraph',
      position: paragraphPos,
      priority: BOUNDARY_PRIORITIES.paragraph,
    });
  }

  // Check for heading boundary
  const headingPos = findHeadingBoundary(text, safeMaxPosition);
  if (headingPos < safeMaxPosition && headingPos > 0) {
    boundaries.push({
      type: 'heading',
      position: headingPos,
      priority: BOUNDARY_PRIORITIES.heading,
    });
  }

  // Check for sentence boundary
  const sentencePos = findSentenceBoundary(text, safeMaxPosition);
  if (sentencePos < safeMaxPosition && sentencePos > 0) {
    boundaries.push({
      type: 'sentence',
      position: sentencePos,
      priority: BOUNDARY_PRIORITIES.sentence,
    });
  }

  // Sort by position (prefer later boundaries) then by priority
  boundaries.sort((a, b) => {
    // Prefer positions closer to maxPosition
    const posDiff = b.position - a.position;
    if (Math.abs(posDiff) > 100) {
      return posDiff; // If significantly different, prefer later
    }
    // Otherwise, prefer higher priority
    return b.priority - a.priority;
  });

  if (boundaries.length > 0) {
    return boundaries[0].position;
  }

  // No good boundary found, return safe max
  return safeMaxPosition;
}

// ===========================================
// CONTENT SUMMARIZATION
// ===========================================

// NOTE: Thinking content is now wrapped in <think>...</think> tags per BRIDGE-006 rule [10]
// The summarizeThinking() function was removed as it is no longer needed.

/**
 * Summarize tool result to a brief description.
 */
export function summarizeToolResult(
  toolName: string,
  content: string,
  args?: Record<string, unknown>
): string {
  const lineCount = (content.match(/\n/g) || []).length + 1;

  // Handle Read tool specially
  if (toolName === 'Read' && args?.file_path) {
    const filePath = String(args.file_path);
    return `📄 Read ${filePath} (${lineCount} lines)`;
  }

  // For large outputs, show summary
  if (content.length > 500) {
    return `[${toolName}] ${lineCount} lines of output`;
  }

  // For small outputs, could show content (but tests expect summary)
  return `[${toolName}] ${Math.min(lineCount, 10)} line${lineCount === 1 ? '' : 's'} of output`;
}

/**
 * Format tool call with name and arguments.
 */
export function formatToolCallSummary(
  toolName: string,
  args?: Record<string, unknown>
): string {
  if (!args || Object.keys(args).length === 0) {
    return `🔧 Running: ${toolName}`;
  }

  // For Fspec with command, show command
  if (toolName === 'Fspec' && args.command) {
    return `🔧 Running: Fspec(${args.command})`;
  }

  // For Read with file_path, show path
  if (toolName === 'Read' && args.file_path) {
    return `🔧 Running: Read(file_path: ${args.file_path})`;
  }

  // Generic: show first arg
  const firstKey = Object.keys(args)[0];
  const firstValue = args[firstKey];
  if (typeof firstValue === 'string' && firstValue.length < 50) {
    return `🔧 Running: ${toolName}(${firstKey}: ${firstValue})`;
  }

  return `🔧 Running: ${toolName}`;
}

// ===========================================
// MARKDOWN VALIDATION
// ===========================================

/**
 * Balance markdown markers (close unclosed code blocks, bold, inline code).
 */
export function balanceMarkdown(text: string): string {
  let result = text;

  // Count code block markers
  const codeBlockCount = (result.match(/```/g) || []).length;
  if (codeBlockCount % 2 === 1) {
    // Unclosed code block - add closing
    result += '\n```';
  }

  // Count bold markers (** pairs)
  const boldCount = (result.match(/\*\*/g) || []).length;
  if (boldCount % 2 === 1) {
    result += '**';
  }

  // Count italic markers (single * not part of **)
  // This is tricky - need to count * that aren't part of **
  const withoutBold = result.replace(/\*\*/g, '');
  const italicCount = (withoutBold.match(/\*/g) || []).length;
  if (italicCount % 2 === 1) {
    result += '*';
  }

  // Count inline code backticks (single ` not part of ```)
  const withoutCodeBlocks = result.replace(/```[\s\S]*?```/g, '');
  const backtickCount = (withoutCodeBlocks.match(/`/g) || []).length;
  if (backtickCount % 2 === 1) {
    result += '`';
  }

  return result;
}

// ===========================================
// CONTENT CHUNKER CLASS
// ===========================================

/**
 * Stateful content chunker that accumulates streaming data
 * and flushes at detected boundaries.
 */
export class ContentChunker {
  private buffer: string[] = [];
  private thinkingBuffer: string[] = [];
  private maxLength: number;

  constructor(maxLength: number = TELEGRAM_MAX_LENGTH) {
    this.maxLength = maxLength;
  }

  /**
   * Add text content to the buffer.
   */
  addText(text: string): void {
    this.buffer.push(text);
  }

  /**
   * Add thinking content to the buffer.
   */
  addThinking(thinking: string): void {
    this.thinkingBuffer.push(thinking);
  }

  /**
   * Get current buffer length.
   */
  getBufferLength(): number {
    return this.buffer.join('').length;
  }

  /**
   * Flush the buffer and return messages to send.
   */
  flush(): FlushResult {
    const messages: string[] = [];

    // Handle thinking - wrap in <think> tags, stream content naturally
    // Note: Must escape < and > characters for Telegram MarkdownV2
    if (this.thinkingBuffer.length > 0) {
      const combinedThinking = this.thinkingBuffer.join(' ');
      // Wrap thinking content in escaped <think> tags for MarkdownV2
      const thinkingContent = `\\<think\\>${combinedThinking}\\</think\\>`;

      // Apply same chunking logic to thinking content
      let text = thinkingContent;
      while (text.length > 0) {
        if (text.length <= this.maxLength) {
          messages.push(balanceMarkdown(text));
          break;
        }

        const splitPoint = getBestSplitPoint(text, this.maxLength);
        const chunk = text.slice(0, splitPoint);
        messages.push(balanceMarkdown(chunk));
        text = text.slice(splitPoint).trim();
      }
      this.thinkingBuffer = [];
    }

    // Handle text content
    if (this.buffer.length > 0) {
      let text = this.buffer.join('');
      this.buffer = [];

      // Split if too long
      while (text.length > 0) {
        if (text.length <= this.maxLength) {
          messages.push(balanceMarkdown(text));
          break;
        }

        const splitPoint = getBestSplitPoint(text, this.maxLength);
        const chunk = text.slice(0, splitPoint);
        messages.push(balanceMarkdown(chunk));
        text = text.slice(splitPoint).trim();
      }
    }

    return { messages, remaining: '' };
  }

  /**
   * Clear the buffer without flushing.
   */
  clear(): void {
    this.buffer = [];
    this.thinkingBuffer = [];
  }
}
