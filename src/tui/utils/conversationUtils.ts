/**
 * Conversation utilities for message processing
 *
 * SOLID: Pure functions for converting ConversationMessage to ConversationLine
 * DRY: Uses shared textWrap utilities for line wrapping
 */

import type {
  ConversationMessage,
  ConversationLine,
} from '../types/conversation';
import { wrapText } from '../utils/textWrap';
import { normalizeEmojiWidth } from '../utils/stringWidth';

/**
 * Derive display role from message type (for coloring)
 * SOLID: Single responsibility - just determines the role category
 */
export const getDisplayRole = (
  msg: ConversationMessage
): 'user' | 'assistant' | 'tool' => {
  switch (msg.type) {
    case 'user-input':
      return 'user';
    case 'assistant-text':
      return 'assistant';
    case 'thinking':
      return 'assistant'; // Thinking is assistant content, rendered differently
    case 'tool-call':
      return 'tool';
    case 'status':
      return 'tool';
  }
};

/**
 * Wrap a single message into multiple ConversationLine objects
 *
 * Each line is guaranteed to fit within maxWidth and contain NO newlines.
 * This is critical for VirtualList which expects each item to be ONE visual line.
 *
 * @param msg - The conversation message to wrap
 * @param msgIndex - Index of the message (for grouping lines by message)
 * @param maxWidth - Maximum visual width per line
 * @param addSeparator - Whether to add a separator line after the message (default: true)
 * @returns Array of ConversationLine objects, each representing one visual line
 */
export const wrapMessageToLines = (
  msg: ConversationMessage,
  msgIndex: number,
  maxWidth: number,
  addSeparator: boolean = true
): ConversationLine[] => {
  const lines: ConversationLine[] = [];
  const role = getDisplayRole(msg);

  // Add role prefix to first line
  // SOLID: Thinking messages get no prefix (the [Thinking] header is already in content)
  const isThinking = msg.type === 'thinking';
  const prefix = isThinking
    ? ''
    : msg.type === 'user-input'
      ? 'You: '
      : msg.type === 'assistant-text'
        ? '● '
        : '';

  // Normalize emoji variation selectors for consistent width calculation
  const normalizedContent = normalizeEmojiWidth(msg.content);
  const contentLines = normalizedContent.split('\n');

  // Propagate semantic flags from message
  const isError = msg.isError;
  // WATCH-011: Propagate correlation fields
  const correlationId = msg.correlationId;
  const observedCorrelationIds = msg.observedCorrelationIds;

  contentLines.forEach((lineContent, lineIndex) => {
    let displayContent =
      lineIndex === 0 ? `${prefix}${lineContent}` : lineContent;

    // Add streaming indicator to last line of streaming message
    const isLastLine = lineIndex === contentLines.length - 1;
    if (msg.isStreaming && isLastLine) {
      displayContent += '...';
    }

    // Use shared wrapText utility for consistent line wrapping
    const wrappedLines = wrapText(displayContent, { maxWidth });

    // Handle empty content (becomes single space for visual line)
    if (wrappedLines.length === 0) {
      lines.push({
        role,
        content: ' ',
        messageIndex: msgIndex,
        isThinking,
        isError,
        correlationId,
        observedCorrelationIds,
      });
    } else {
      wrappedLines.forEach(wrappedContent => {
        lines.push({
          role,
          content: wrappedContent,
          messageIndex: msgIndex,
          isThinking,
          isError,
          correlationId,
          observedCorrelationIds,
        });
      });
    }
  });

  // Add separator line after message for visual grouping (TUI-042)
  if (addSeparator) {
    lines.push({
      role,
      content: ' ',
      messageIndex: msgIndex,
      isSeparator: true,
      isThinking,
      isError,
      correlationId,
      observedCorrelationIds,
    });
  }

  return lines;
};

/**
 * Convert an array of ConversationMessage to an array of ConversationLine
 *
 * Convenience function that wraps all messages and flattens the result.
 * Each message includes a trailing separator line for visual grouping.
 *
 * @param messages - Array of conversation messages
 * @param maxWidth - Maximum visual width per line
 * @returns Flattened array of ConversationLine objects
 */
export const messagesToLines = (
  messages: ConversationMessage[],
  maxWidth: number
): ConversationLine[] => {
  const allLines: ConversationLine[] = [];

  messages.forEach((msg, msgIndex) => {
    const msgLines = wrapMessageToLines(msg, msgIndex, maxWidth, true);
    allLines.push(...msgLines);
  });

  return allLines;
};
