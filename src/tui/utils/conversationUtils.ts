/**
 * Conversation utilities for message processing
 *
 * SOLID: Pure functions for converting ConversationMessage to ConversationLine
 * DRY: Shared between AgentView (main conversation) and SplitSessionView (parent conversation)
 */

import type {
  ConversationMessage,
  ConversationLine,
} from '../types/conversation';
import { normalizeEmojiWidth, getVisualWidth } from '../utils/stringWidth';

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
 * @returns Array of ConversationLine objects, each representing one visual line
 */
export const wrapMessageToLines = (
  msg: ConversationMessage,
  msgIndex: number,
  maxWidth: number
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

  contentLines.forEach((lineContent, lineIndex) => {
    let displayContent =
      lineIndex === 0 ? `${prefix}${lineContent}` : lineContent;

    // Add streaming indicator to last line of streaming message
    const isLastLine = lineIndex === contentLines.length - 1;
    if (msg.isStreaming && isLastLine) {
      displayContent += '...';
    }

    // Wrap long lines manually to fit terminal width (using visual width for Unicode)
    if (getVisualWidth(displayContent) === 0) {
      lines.push({
        role,
        content: ' ',
        messageIndex: msgIndex,
        isThinking,
        isError,
      });
    } else {
      // Split into words, keeping whitespace
      const words = displayContent.split(/(\s+)/);
      let currentLine = '';
      let currentWidth = 0;

      for (const word of words) {
        const wordWidth = getVisualWidth(word);

        if (wordWidth === 0) {
          continue;
        }

        // If word alone exceeds max width, force break it character by character
        if (wordWidth > maxWidth) {
          // Flush current line first
          if (currentLine) {
            lines.push({
              role,
              content: currentLine,
              messageIndex: msgIndex,
              isThinking,
              isError,
            });
            currentLine = '';
            currentWidth = 0;
          }
          // Break long word by visual width
          let chunk = '';
          let chunkWidth = 0;
          for (const char of word) {
            const charWidth = getVisualWidth(char);
            if (chunkWidth + charWidth > maxWidth && chunk) {
              lines.push({
                role,
                content: chunk,
                messageIndex: msgIndex,
                isThinking,
                isError,
              });
              chunk = char;
              chunkWidth = charWidth;
            } else {
              chunk += char;
              chunkWidth += charWidth;
            }
          }
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
            lines.push({
              role,
              content: currentLine.trimEnd(),
              messageIndex: msgIndex,
              isThinking,
              isError,
            });
          }
          // Don't start line with whitespace
          currentLine = word.trim() ? word : '';
          currentWidth = word.trim() ? wordWidth : 0;
        } else {
          currentLine += word;
          currentWidth += wordWidth;
        }
      }

      // Flush remaining content
      if (currentLine.trim()) {
        lines.push({
          role,
          content: currentLine.trimEnd(),
          messageIndex: msgIndex,
          isThinking,
          isError,
        });
      }
    }
  });

  return lines;
};

/**
 * Convert an array of ConversationMessage to an array of ConversationLine
 *
 * Convenience function that wraps all messages and flattens the result.
 * Adds separator lines between messages for visual grouping.
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
    // Add separator before each message (except first)
    if (msgIndex > 0) {
      allLines.push({
        role: 'assistant',
        content: '',
        messageIndex: msgIndex,
        isSeparator: true,
      });
    }

    const msgLines = wrapMessageToLines(msg, msgIndex, maxWidth);
    allLines.push(...msgLines);
  });

  return allLines;
};
