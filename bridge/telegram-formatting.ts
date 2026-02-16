/**
 * Telegram Message Formatting
 *
 * Handles formatting of messages for Telegram display, including
 * MarkdownV2 escaping and smart truncation.
 *
 * BRIDGE-002: Telegram Bridge Endpoint (extracted for maintainability)
 */

// ============================================================================
// Constants
// ============================================================================

/** Telegram's maximum message length */
export const TELEGRAM_MAX_LENGTH = 4096;

/** Characters to preserve at beginning and end when truncating */
export const PRESERVE_CHARS = 1500;

// ============================================================================
// Types
// ============================================================================

export interface StreamChunkData {
  type: 'text' | 'thinking' | 'tool_call' | 'tool_result' | 'done' | 'error';
  text?: string;
  thinking?: string;
  name?: string;
  id?: string;
  tool_call_id?: string;
  content?: string;
  is_error?: boolean;
  error?: string;
}

// ============================================================================
// MarkdownV2 Escaping
// ============================================================================

/**
 * Escape special characters for Telegram MarkdownV2 format.
 * Characters that need escaping: _ * [ ] ( ) ~ ` > # + - = | { } . ! <
 * Does NOT escape content inside code blocks.
 */
export function escapeMarkdownV2(text: string): string {
  const specialChars = /([_*[\]()~`>#+\-=|{}.!<])/g;

  // Split by code blocks to avoid escaping inside them
  const parts = text.split(/(```[\s\S]*?```|`[^`]+`)/);

  return parts
    .map((part, index) => {
      // Odd indices are code blocks (matched by the regex group)
      if (index % 2 === 1) {
        return part; // Don't escape inside code blocks
      }
      // Note: '\\$1' in JS source becomes '\$1' at runtime (backslash + captured group)
      return part.replace(specialChars, '\\$1');
    })
    .join('');
}

// ============================================================================
// Message Truncation
// ============================================================================

/**
 * Smart truncation that preserves beginning and end of message.
 * Properly handles code block boundaries.
 */
export function truncateMessage(
  text: string,
  maxLength: number = TELEGRAM_MAX_LENGTH
): string {
  if (text.length <= maxLength) {
    return text;
  }

  // Calculate how much we need to cut
  const omittedChars = text.length - PRESERVE_CHARS * 2;
  const indicator = `\n\n[...${omittedChars} chars omitted...]\n\n`;
  const targetLength = maxLength - indicator.length;
  const halfTarget = Math.floor(targetLength / 2);

  let beginning = text.slice(0, halfTarget);
  let ending = text.slice(-halfTarget);

  // Handle code block boundaries in the beginning part
  const beginningOpenBlocks = countOpenCodeBlocks(beginning);
  if (beginningOpenBlocks > 0) {
    // Close any open code blocks
    beginning += '\n```';
  }

  // Handle code block boundaries in the ending part
  const endingHasOpenBlock = hasUnclosedCodeBlock(ending);
  if (endingHasOpenBlock || beginningOpenBlocks > 0) {
    // Re-open code block if the ending contains code block content
    const firstCodeBlockInEnding = ending.indexOf('```');
    if (firstCodeBlockInEnding === -1 || endingHasOpenBlock) {
      ending = '```\n' + ending;
    }
  }

  return beginning + indicator + ending;
}

/**
 * Count unclosed code blocks (``` markers) in text.
 */
function countOpenCodeBlocks(text: string): number {
  const matches = text.match(/```/g);
  if (!matches) {
    return 0;
  }
  // Odd number means there's an unclosed block
  return matches.length % 2;
}

/**
 * Check if text starts inside an unclosed code block.
 * (i.e., the text would need an opening ``` prepended)
 */
function hasUnclosedCodeBlock(text: string): boolean {
  // Check if first ``` is a closing marker (no language specifier before it)
  const firstMarker = text.indexOf('```');
  if (firstMarker === -1) {
    return false;
  }
  // If there's text before the first ```, check if it looks like code block content
  const beforeMarker = text.slice(0, firstMarker);
  // If the text before contains newlines but no ```, we're inside a code block
  return beforeMarker.includes('\n') && !beforeMarker.includes('```');
}

// ============================================================================
// Stream Chunk Formatting
// ============================================================================

/**
 * Format a StreamChunk for Telegram display.
 */
export function formatForTelegram(
  chunk: StreamChunkData,
  toolNameMap?: Map<string, string>
): string {
  const map = toolNameMap || new Map();

  switch (chunk.type) {
    case 'text':
      return escapeMarkdownV2(chunk.text || '');

    case 'thinking':
      return `💭 ${escapeMarkdownV2(chunk.thinking || '')}`;

    case 'tool_call':
      // Store tool name for later correlation
      if (chunk.id && chunk.name) {
        map.set(chunk.id, chunk.name);
      }
      return `🔧 Running: ${escapeMarkdownV2(chunk.name || 'unknown')}`;

    case 'tool_result': {
      // Look up tool name from stored mapping
      const toolName = chunk.tool_call_id
        ? map.get(chunk.tool_call_id) || 'unknown'
        : 'unknown';
      const content = chunk.content || '';
      const prefix = chunk.is_error ? '❌ ' : '';
      return `${prefix}\\[${escapeMarkdownV2(toolName)}\\] ${escapeMarkdownV2(content)}`;
    }

    case 'done':
      return '✓';

    case 'error':
      return `❌ Error: ${escapeMarkdownV2(chunk.error || 'Unknown error')}`;

    default:
      return '';
  }
}

// ============================================================================
// Image Utilities
// ============================================================================

/**
 * Get media type from file path extension.
 * Defaults to image/jpeg for unknown or missing extensions (Telegram photos are typically JPEG).
 */
export function getMediaTypeFromPath(filePath: string): string {
  const ext = filePath.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'png':
      return 'image/png';
    case 'jpg':
    case 'jpeg':
      return 'image/jpeg';
    case 'gif':
      return 'image/gif';
    case 'webp':
      return 'image/webp';
    default:
      return 'image/jpeg'; // Default for Telegram photos
  }
}
