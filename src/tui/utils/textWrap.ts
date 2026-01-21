/**
 * Generic text wrapping utilities for VirtualList
 *
 * SOLID: Single responsibility - wrap text to fit terminal width
 * DRY: Shared by all VirtualList consumers that need text wrapping
 *
 * The Problem:
 * VirtualList assumes 1 item = 1 visual line. If content is wider than the
 * display area, Ink wraps it at render time, breaking VirtualList's math
 * (item count ≠ visual line count → broken scrolling).
 *
 * The Solution:
 * Pre-wrap content to the correct width BEFORE passing to VirtualList.
 * This ensures 1 item = 1 visual line.
 *
 * Usage:
 * 1. Determine your display width (terminalWidth, pane width, etc.)
 * 2. Use wrapText() to split content into lines that fit
 * 3. Pass the wrapped lines to VirtualList
 */

import { normalizeEmojiWidth, getVisualWidth } from './stringWidth';

/**
 * A single wrapped line of text with metadata for VirtualList rendering
 */
export interface WrappedLine<T = unknown> {
  /** The text content for this line (guaranteed to fit within maxWidth) */
  content: string;
  /** Index of the original item this line came from */
  sourceIndex: number;
  /** Line number within the original item (0-based) */
  lineIndex: number;
  /** Whether this is the first line of the original item */
  isFirstLine: boolean;
  /** Whether this is the last line of the original item */
  isLastLine: boolean;
  /** Optional metadata from the source item (for rendering context) */
  metadata?: T;
}

/**
 * Options for text wrapping
 */
export interface WrapOptions {
  /** Maximum visual width per line (required) */
  maxWidth: number;
  /** Preserve empty lines as single-space lines (default: true) */
  preserveEmptyLines?: boolean;
  /** Handle word breaking for long words (default: true) */
  breakLongWords?: boolean;
}

/**
 * Wrap a single string into multiple lines that fit within maxWidth.
 *
 * Handles:
 * - Newline characters (explicit line breaks)
 * - Word wrapping at word boundaries
 * - Long word breaking (when word > maxWidth)
 * - Unicode/emoji width via string-width
 *
 * @param text - The text to wrap
 * @param options - Wrapping options
 * @returns Array of line strings, each guaranteed to fit within maxWidth
 */
export function wrapText(text: string, options: WrapOptions): string[] {
  const {
    maxWidth,
    preserveEmptyLines = true,
    breakLongWords = true,
  } = options;
  const lines: string[] = [];

  // Normalize emoji for consistent width calculation
  const normalizedText = normalizeEmojiWidth(text);

  // Split on explicit newlines first
  const paragraphs = normalizedText.split('\n');

  for (const paragraph of paragraphs) {
    // Handle empty lines
    if (paragraph.length === 0) {
      if (preserveEmptyLines) {
        lines.push(' '); // Single space to maintain visual line
      }
      continue;
    }

    // Check if paragraph fits without wrapping
    const paragraphWidth = getVisualWidth(paragraph);
    if (paragraphWidth <= maxWidth) {
      lines.push(paragraph);
      continue;
    }

    // Word wrap the paragraph
    const words = paragraph.split(/(\s+)/); // Keep whitespace as separate tokens
    let currentLine = '';
    let currentWidth = 0;

    for (const word of words) {
      const wordWidth = getVisualWidth(word);

      // Skip empty tokens
      if (wordWidth === 0) {
        continue;
      }

      // Handle words that are longer than maxWidth
      if (wordWidth > maxWidth && breakLongWords) {
        // Flush current line first
        if (currentLine.trim()) {
          lines.push(currentLine.trimEnd());
          currentLine = '';
          currentWidth = 0;
        }

        // Break long word character by character
        let chunk = '';
        let chunkWidth = 0;
        for (const char of word) {
          const charWidth = getVisualWidth(char);
          if (chunkWidth + charWidth > maxWidth && chunk) {
            lines.push(chunk);
            chunk = char;
            chunkWidth = charWidth;
          } else {
            chunk += char;
            chunkWidth += charWidth;
          }
        }
        // Remaining chunk becomes start of next line
        if (chunk) {
          currentLine = chunk;
          currentWidth = chunkWidth;
        }
        continue;
      }

      // Check if word fits on current line
      if (currentWidth + wordWidth > maxWidth) {
        // Flush current line and start new one
        if (currentLine.trim()) {
          lines.push(currentLine.trimEnd());
        }
        // Don't start new line with whitespace
        currentLine = word.trim() ? word : '';
        currentWidth = word.trim() ? wordWidth : 0;
      } else {
        currentLine += word;
        currentWidth += wordWidth;
      }
    }

    // Flush remaining content
    if (currentLine.trim()) {
      lines.push(currentLine.trimEnd());
    }
  }

  return lines;
}

/**
 * Wrap multiple items into a flat array of WrappedLine objects for VirtualList.
 *
 * This is the main utility for preparing content for VirtualList display.
 * Each item's content is wrapped, and the resulting lines are annotated with
 * metadata about their source for rendering purposes.
 *
 * @param items - Array of items to wrap
 * @param getContent - Function to extract text content from an item
 * @param options - Wrapping options (must include maxWidth)
 * @param getMetadata - Optional function to extract metadata for rendering
 * @returns Flat array of WrappedLine objects ready for VirtualList
 *
 * @example
 * // Wrap messages for display
 * const lines = wrapItems(
 *   messages,
 *   (msg) => msg.content,
 *   { maxWidth: paneWidth },
 *   (msg) => ({ role: msg.role, isThinking: msg.type === 'thinking' })
 * );
 */
export function wrapItems<T, M = unknown>(
  items: T[],
  getContent: (item: T, index: number) => string,
  options: WrapOptions,
  getMetadata?: (item: T, index: number) => M
): WrappedLine<M>[] {
  const result: WrappedLine<M>[] = [];

  items.forEach((item, sourceIndex) => {
    const content = getContent(item, sourceIndex);
    const wrappedLines = wrapText(content, options);
    const metadata = getMetadata?.(item, sourceIndex);

    wrappedLines.forEach((lineContent, lineIndex) => {
      result.push({
        content: lineContent,
        sourceIndex,
        lineIndex,
        isFirstLine: lineIndex === 0,
        isLastLine: lineIndex === wrappedLines.length - 1,
        metadata,
      });
    });
  });

  return result;
}

/**
 * Calculate the correct maxWidth for a VirtualList pane.
 *
 * Use this utility to determine the wrap width based on your layout:
 * - Full width: Single pane taking full terminal
 * - Split width: Two equal panes with divider
 * - Custom: Specific pane widths
 *
 * @param terminalWidth - Total terminal width
 * @param layout - Layout type or custom width
 * @returns Calculated maxWidth for text wrapping
 *
 * @example
 * // Full-width pane
 * const maxWidth = calculatePaneWidth(terminalWidth, 'full');
 *
 * // Split-view pane (left or right)
 * const maxWidth = calculatePaneWidth(terminalWidth, 'split');
 */
export function calculatePaneWidth(
  terminalWidth: number,
  layout: 'full' | 'split' | number
): number {
  // Margins/padding constants
  const FULL_MARGIN = 6; // Borders, padding, scrollbar for full-width
  const SPLIT_DIVIDER = 1; // Divider between panes
  const SPLIT_MARGIN = 4; // Per-pane margin (borders, padding, scrollbar)

  if (layout === 'full') {
    return Math.max(10, terminalWidth - FULL_MARGIN);
  }

  if (layout === 'split') {
    // Each pane gets half the width minus divider and margins
    const availableWidth = terminalWidth - SPLIT_DIVIDER;
    const paneWidth = Math.floor(availableWidth / 2) - SPLIT_MARGIN;
    return Math.max(10, paneWidth);
  }

  // Custom width (number)
  return Math.max(10, layout);
}
